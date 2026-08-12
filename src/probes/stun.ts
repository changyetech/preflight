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

/** 单个 STUN 的取值：拿到第一个反射地址即收工；超时、中止、交换失败一律记作没答上来。 */
function reflexiveFrom(
  pc: PeerConnection,
  signal: AbortSignal | undefined,
): Promise<string | null> {
  return new Promise((resolve) => {
    let timer: ReturnType<typeof setTimeout>;
    let settled = false;

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

    timer = setTimeout(() => finish(null), TIMEOUT_MS);
    signal?.addEventListener("abort", onAbort);

    pc.addEventListener("icecandidate", (event) => {
      // null 候选 = 收集结束。走到这里还没拿到 srflx，就是这个 STUN 没答上来。
      if (event.candidate === null) return finish(null);

      const ip = reflexiveAddressOf(event.candidate.candidate);
      if (ip !== null) finish(ip);
    });

    // 没有 m 行就不会开始候选收集，数据通道是拿它的最省事的办法。
    pc.createDataChannel("ipcheck");
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
  if (opened.length < connections.length) {
    // 一个开不出来说明整个 WebRTC 栈被禁或被桩掉了，此时已开的也一并关掉。
    for (const pc of opened) pc.close();
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
