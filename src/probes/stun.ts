// O6 的探测层：经浏览器的 WebRTC 栈向两个独立的 STUN 取反射地址（srflx）。
//
// 判定逻辑不在这里——它在 domain/udpEgress.ts，输入只是「几个 STUN 答上来了」。
// CLI 侧走裸 UDP socket 发 RFC 5389 binding request，两端因此可能得出不同结果（契约 §5.6），
// 这条缝让那处差异只落在探测层。
//
// 类型不用 DOM 的 RTCPeerConnection，而是自己声明最小结构：全局对象上到底有什么必须
// **运行时**判断——隐私扩展会塞一个只有构造函数、没有 createOffer 的桩进来，
// 而按 DOM 类型写代码会让这种桩在类型层面「看起来存在」。

import type { StunProbe } from "../domain/udpEgress";

/** 两个互相独立的 STUN。对照是必需的：单个 STUN 分不清 UDP 泄露与多地址出口集群。 */
const STUN_URLS = [
  "stun:stun.cloudflare.com:3478",
  "stun:stun.l.google.com:19302",
];

/** 与 ipify、DNS 出口探测对齐。 */
const TIMEOUT_MS = 5000;

type IceCandidateEvent = { candidate: { candidate: string } | null };

type PeerConnection = {
  createDataChannel: (label: string) => unknown;
  createOffer: () => Promise<unknown>;
  setLocalDescription: (description: unknown) => Promise<void>;
  addEventListener: (
    type: string,
    listener: (event: IceCandidateEvent) => void,
  ) => void;
  close: () => void;
};

type PeerConnectionCtor = new (config: {
  iceServers: { urls: string[] }[];
}) => Partial<PeerConnection>;

function usable(pc: Partial<PeerConnection>): pc is PeerConnection {
  return (
    typeof pc.createDataChannel === "function" &&
    typeof pc.createOffer === "function" &&
    typeof pc.setLocalDescription === "function" &&
    typeof pc.addEventListener === "function" &&
    typeof pc.close === "function"
  );
}

function open(ctor: PeerConnectionCtor, url: string): PeerConnection | null {
  try {
    const pc = new ctor({ iceServers: [{ urls: [url] }] });
    if (usable(pc)) return pc;
    pc.close?.();
    return null;
  } catch {
    // 企业策略／扩展可以让构造函数直接抛，那与「没有这个 API」是同一件事。
    return null;
  }
}

/**
 * `candidate:<foundation> <component> <transport> <priority> <address> <port> typ <type> …`
 *
 * **只接受 srflx / prflx**：host 候选是本机地址，证明不了 STUN 工作过，
 * 现代浏览器还会把它 mDNS 混淆成 `.local`，拿它当反射地址纯属自欺。
 */
function reflexiveAddressOf(line: string): string | null {
  const parts = line.trim().split(/\s+/);

  const typeIndex = parts.indexOf("typ");
  if (typeIndex < 0) return null;

  const type = parts[typeIndex + 1];
  if (type !== "srflx" && type !== "prflx") return null;

  const address = parts[4];
  return address ? address : null;
}

function isIpv4(ip: string): boolean {
  return /^\d{1,3}(\.\d{1,3}){3}$/.test(ip);
}

/**
 * 从这条连接收到的全部 srflx 里挑一个，**优先 IPv4**。
 *
 * **协议族不能由 ICE 候选的到达顺序决定。** 比对的另一侧恒为 IPv4（出口 IP 取自
 * `/api/geo`），而双栈主机上浏览器会为 IPv4 与 IPv6 分别收集 srflx，谁先到没有保证：
 * 拿第一个到的就收工，同一台机器可能正常出结论、可能 `N_fam = 1`、也可能 `N_fam = 0`
 * ——什么都没测出来，覆盖度却把它算作「已完成」，而且**平价验收无从复现**。
 * CLI 侧同一个不确定性由 `first_ipv4` 消掉（`cli/src/probe/stun.rs`，触发源是
 * `getaddrinfo` 顺序），这里对齐它，只是把「筛请求端」换成「筛候选」。
 */
function preferIpv4(addresses: string[]): string | null {
  return addresses.find(isIpv4) ?? addresses[0] ?? null;
}

/**
 * 单个 STUN 的取值：**收集到候选结束或超时为止的全部 srflx**，再按上面的规则挑一个。
 * 超时、中止、交换失败一律记作没答上来。
 */
function reflexiveFrom(
  pc: PeerConnection,
  signal: AbortSignal | undefined,
): Promise<string | null> {
  return new Promise((resolve) => {
    let timer: ReturnType<typeof setTimeout>;
    let settled = false;
    const reflexive: string[] = [];

    const finish = (ip: string | null) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      signal?.removeEventListener("abort", onAbort);
      // 组件卸载或本轮结束都必须关掉连接，否则在途的 ICE 会一直挂着。
      pc.close();
      resolve(ip);
    };
    const onAbort = () => finish(null);

    // 超时不等于白等：已经收到的 srflx 照常算数，只是没等到收集结束。
    timer = setTimeout(() => finish(preferIpv4(reflexive)), TIMEOUT_MS);
    signal?.addEventListener("abort", onAbort);

    pc.addEventListener("icecandidate", (event) => {
      // null 候选 = 收集结束。此时手里一个 srflx 都没有，就是这个 STUN 没答上来。
      if (event.candidate === null) return finish(preferIpv4(reflexive));

      const ip = reflexiveAddressOf(event.candidate.candidate);
      if (ip !== null) reflexive.push(ip);
    });

    // 没有 m 行就不会开始候选收集，数据通道是拿它的最省事的办法。
    pc.createDataChannel("preflight");
    pc.createOffer()
      .then((offer) => pc.setLocalDescription(offer))
      .catch(() => finish(null));
  });
}

/**
 * 产出 `{ reflexiveIps, webrtcSupported }`：列表里只放**答上来**的（即契约 §2.6 的 N_ok），
 * 没答上来的不占位。`webrtcSupported` 只用来区分检测失败的两种原因（契约 §5.6）。
 *
 * `signal` 由调用方在组件卸载时中止，用于关闭全部在途 RTCPeerConnection。
 */
export async function probeStun(signal?: AbortSignal): Promise<StunProbe> {
  const ctor = (globalThis as { RTCPeerConnection?: unknown })
    .RTCPeerConnection;
  if (typeof ctor !== "function") {
    return { reflexiveIps: [], webrtcSupported: false };
  }

  if (signal?.aborted) return { reflexiveIps: [], webrtcSupported: true };

  const connections = STUN_URLS.map((url) =>
    open(ctor as PeerConnectionCtor, url),
  );
  const opened = connections.filter((pc) => pc !== null);
  if (opened.length === 0) {
    // **全部**开不出来才说明整个 WebRTC 栈被禁或被桩掉了。只坏一条时照常探测：
    // 构造函数的成败与 STUN URL 无关，把它报成「你的浏览器禁用了 WebRTC」是一句
    // 可能不成立的话，而两个失败原因的可行动性正好相反（契约 §5.6）。
    return { reflexiveIps: [], webrtcSupported: false };
  }

  const addresses = await Promise.all(
    opened.map((pc) => reflexiveFrom(pc, signal)),
  );

  return {
    reflexiveIps: addresses.filter((ip) => ip !== null),
    webrtcSupported: true,
  };
}
