// O6 的探测层：浏览器 WebRTC 栈取反射地址（子计划 --web 第 1 步）。
//
// 最要紧的一格是「取不到候选」——浏览器有权拒绝暴露本机的 UDP 候选（契约 §5.6），
// 隐私扩展还会塞一个方法不全的桩进来。两种情形都必须走「拿不到候选」这条路，
// 而不是抛异常或静默当成 0 个反射地址却报告 WebRTC 可用（后者会把失败原因说反）。

import { afterEach, describe, expect, it } from "vitest";

import { probeStun } from "../src/probes/stun";

/** 测试环境（workerd）本来就没有 RTCPeerConnection，装桩后必须还原。 */
const scope = globalThis as unknown as { RTCPeerConnection?: unknown };

afterEach(() => {
  delete scope.RTCPeerConnection;
});

type CandidateListener = (event: {
  candidate: { candidate: string } | null;
}) => void;

/** 逐个吐出 candidate 行、最后以 null 收尾的假实现，形状照浏览器的 RTCPeerConnection。 */
function fakePeerConnection(candidatesOf: (url: string) => string[]) {
  return class {
    private listeners: CandidateListener[] = [];
    private readonly url: string;
    closed = false;

    constructor(config: { iceServers: { urls: string[] }[] }) {
      this.url = config.iceServers[0].urls[0];
    }

    createDataChannel() {}
    createOffer() {
      return Promise.resolve({ type: "offer", sdp: "" });
    }
    setLocalDescription() {
      for (const line of candidatesOf(this.url)) {
        for (const listener of this.listeners) {
          listener({ candidate: { candidate: line } });
        }
      }
      // null candidate = 候选收集结束。
      for (const listener of this.listeners) listener({ candidate: null });
      return Promise.resolve();
    }
    addEventListener(_type: string, listener: CandidateListener) {
      this.listeners.push(listener);
    }
    close() {
      this.closed = true;
    }
  };
}

/** 永不吐候选、也不收尾的假实现：用来把探测停在「在途」状态，好去测中止路径。 */
function pendingPeerConnection(instances: { closed: boolean }[]) {
  return class {
    closed = false;

    constructor() {
      instances.push(this);
    }

    createDataChannel() {}
    createOffer() {
      return new Promise<unknown>(() => {});
    }
    setLocalDescription() {
      return new Promise<void>(() => {});
    }
    addEventListener() {}
    close() {
      this.closed = true;
    }
  };
}

const SRFLX =
  "candidate:842163049 1 udp 1677729535 203.0.113.7 54321 typ srflx raddr 0.0.0.0 rport 0";
const SRFLX_V6 =
  "candidate:842163050 1 udp 1677729534 2001:db8::1 54321 typ srflx raddr :: rport 0";
const HOST =
  "candidate:1467250027 1 udp 2122260223 9b36eaac-bb2e.local 51234 typ host";

describe("probeStun", () => {
  it("RTCPeerConnection 不存在 → 拿不到候选，且标明 WebRTC 不可用", async () => {
    expect(scope.RTCPeerConnection).toBeUndefined();

    await expect(probeStun()).resolves.toEqual({
      reflexiveIps: [],
      webrtcSupported: false,
    });
  });

  it("隐私扩展塞进来的桩（缺 createOffer）同样按 WebRTC 不可用处理", async () => {
    scope.RTCPeerConnection = class {
      close() {}
    };

    await expect(probeStun()).resolves.toEqual({
      reflexiveIps: [],
      webrtcSupported: false,
    });
  });

  it("构造函数直接抛（企业策略禁用）也不冒泡，按 WebRTC 不可用处理", async () => {
    scope.RTCPeerConnection = class {
      constructor() {
        throw new Error("blocked by policy");
      }
    };

    await expect(probeStun()).resolves.toEqual({
      reflexiveIps: [],
      webrtcSupported: false,
    });
  });

  it("每个 STUN 各取一个反射地址：两个服务器 ⇒ 两个地址", async () => {
    scope.RTCPeerConnection = fakePeerConnection(() => [SRFLX]);

    const probe = await probeStun();

    expect(probe.webrtcSupported).toBe(true);
    expect(probe.reflexiveIps).toEqual(["203.0.113.7", "203.0.113.7"]);
  });

  it("host 候选不算反射地址——它证明不了 STUN 工作过", async () => {
    // 现代浏览器还会把 host 候选 mDNS 混淆成 .local，拿它当出口地址纯属自欺。
    scope.RTCPeerConnection = fakePeerConnection(() => [HOST]);

    const probe = await probeStun();

    expect(probe.webrtcSupported).toBe(true);
    expect(probe.reflexiveIps).toEqual([]);
  });

  it("双栈：协议族不由候选到达顺序决定，两条连接都取 IPv4", async () => {
    // 比对的另一侧恒为 IPv4（出口 IP 来自 /api/geo）。若拿第一个到达的就收工，
    // 下面这组输入会让一条取到 IPv6、另一条取到 IPv4 ⇒ N_fam = 1 ⇒「无从比对」，
    // 而同一台机器换个时刻又可能正常出结论——平价验收无从复现。
    scope.RTCPeerConnection = fakePeerConnection((url) =>
      url.includes("cloudflare") ? [SRFLX_V6, SRFLX] : [SRFLX, SRFLX_V6],
    );

    expect((await probeStun()).reflexiveIps).toEqual([
      "203.0.113.7",
      "203.0.113.7",
    ]);
  });

  it("只有 IPv6 srflx 时如实返回它，不假装没测到", async () => {
    // 同族筛选是 domain/udpEgress 的职责（契约 §2.6 第 2 行），探测层不替它做判断。
    scope.RTCPeerConnection = fakePeerConnection(() => [SRFLX_V6]);

    expect((await probeStun()).reflexiveIps).toEqual([
      "2001:db8::1",
      "2001:db8::1",
    ]);
  });

  it("两条里只坏一条时照常探测，不谎报「浏览器禁用了 WebRTC」", async () => {
    // 构造函数的成败与 STUN URL 无关，报成禁用是一句可能不成立的话；
    // 而两个失败原因的可行动性正好相反（契约 §5.6），这一格值得保守。
    let created = 0;
    scope.RTCPeerConnection = class extends fakePeerConnection(() => [SRFLX]) {
      constructor(config: { iceServers: { urls: string[] }[] }) {
        if (created++ === 0) throw new Error("first one blows up");
        super(config);
      }
    };

    expect(await probeStun()).toEqual({
      reflexiveIps: ["203.0.113.7"],
      webrtcSupported: true,
    });
  });

  it("只有一个 STUN 答上来时就只报一个——没答上来的不占位（N_ok 的定义）", async () => {
    scope.RTCPeerConnection = fakePeerConnection((url) =>
      url.includes("cloudflare") ? [SRFLX] : [HOST],
    );

    expect((await probeStun()).reflexiveIps).toEqual(["203.0.113.7"]);
  });

  it("探测进行中被中止 ⇒ 两个在途 RTCPeerConnection 都被关闭", async () => {
    // brief 点名的硬约束：组件卸载时必须关闭所有在途连接，否则 ICE 会一直挂着。
    const instances: { closed: boolean }[] = [];
    scope.RTCPeerConnection = pendingPeerConnection(instances);

    const controller = new AbortController();
    const probe = probeStun(controller.signal);
    // probeStun 在第一个 await 之前已同步建好两条连接并挂上 abort 监听。
    expect(instances).toHaveLength(2);
    expect(instances.every((pc) => pc.closed)).toBe(false);

    controller.abort();

    await expect(probe).resolves.toEqual({
      reflexiveIps: [],
      webrtcSupported: true,
    });
    expect(instances.every((pc) => pc.closed)).toBe(true);
  });

  it("已中止的 signal（组件已卸载）不再发起探测", async () => {
    scope.RTCPeerConnection = fakePeerConnection(() => [SRFLX]);

    await expect(probeStun(AbortSignal.abort())).resolves.toEqual({
      reflexiveIps: [],
      webrtcSupported: true,
    });
  });
});
