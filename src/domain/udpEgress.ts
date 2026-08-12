// O6 UDP 出口一致性判定（判级契约 §2.6 判定表）。
//
// 需要至少两个互相独立的 STUN 作对照，否则无法区分「UDP 真的绕过了代理」与
// 「这个出口本来就是多地址集群」——ADR-0003 那条「用对照区分未知与未命中」的第二次应用。

import type { OnlineCheck } from "./checks";
import type { UdpEgressResult } from "./types";

/** STUN 探测的原始观测。`reflexiveIps` 只放**答上来**的，没答上来的不占位（即 §2.6 的 N_ok）。 */
export type StunProbe = {
  reflexiveIps: string[];
  /**
   * 浏览器是否提供了可用的 WebRTC 栈。它只决定检测失败的**原因**：
   * 探测超时刷新可能就好，浏览器不允许则刷新一万次也一样（契约 §5.6 呈现约束）。
   */
  webrtcSupported: boolean;
};

/** 两个失败原因是内部诊断标识，呈现层据此选文案，不直接展示给用户。 */
export const UDP_EGRESS_WEBRTC_UNAVAILABLE = "udp-egress-webrtc-unavailable";
export const UDP_EGRESS_STUN_UNANSWERED = "udp-egress-stun-unanswered";

/** IP 协议族。认不出的地址两端都不属于，因而不会与任何出口 IP 同族。 */
function familyOf(ip: string): "v4" | "v6" | null {
  if (ip.includes(":")) return "v6";
  return /^\d{1,3}(\.\d{1,3}){3}$/.test(ip) ? "v4" : null;
}

function normalize(ip: string): string {
  return ip.trim().toLowerCase();
}

/**
 * 判定表（契约 §2.6），**从上往下取第一条匹配的行**。各行按 `N_ok` → `N_fam` → 取值分层，
 * 任何一组输入只会落进一行：
 *
 * | # | 条件 | 判定 |
 * |---|---|---|
 * | 1 | `N_ok < 2` | 检测失败 |
 * | 2 | `N_ok ≥ 2` 且 `N_fam < 2` | 无从比对 |
 * | 3 | `N_fam ≥ 2` 且出口 IP 未知 | 无从比对 |
 * | 4 | `N_fam ≥ 2`，同族地址彼此不一致 | 无从比对 |
 * | 5 | `N_fam ≥ 2`，一致且 = 出口 IP | 未命中 |
 * | 6 | `N_fam ≥ 2`，一致且 ≠ 出口 IP | 命中 |
 *
 * **「检测失败」只由第 1 行产生**：协议族筛选永远不会造成检测失败，它只会把结果推进
 * 第 2 行的「无从比对」。两者的可恢复性相反，所以这条分工是刻意的。
 */
export function judgeUdpEgress(
  probe: StunProbe,
  exitIp: string | null,
): OnlineCheck<UdpEgressResult> {
  const observed = probe.reflexiveIps.map(normalize).filter((ip) => ip !== "");

  // 第 1 行。禁用 WebRTC 落在这里，而不是绿色的「未泄露」——在「UDP 走没走代理」这个
  // 语义下，禁用 WebRTC 只是把测量仪器关掉了，不是测出了没问题（契约 §2.6）。
  if (observed.length < 2) {
    return {
      status: "failed",
      reason: probe.webrtcSupported
        ? UDP_EGRESS_STUN_UNANSWERED
        : UDP_EGRESS_WEBRTC_UNAVAILABLE,
    };
  }

  const exit = exitIp === null ? null : normalize(exitIp) || null;

  // 第 2 行。反射地址是 IPv6 而出口是 IPv4（或反之）不得判命中——那正是 O3 在管的事，
  // 在这里再算一次等于同一个事实进了两次判级（契约 §2.6 协议族硬约束）。
  // 出口 IP 未知时无从筛选，N_fam = N_ok，输入因此落到第 3 行。
  const exitFamily = exit === null ? null : familyOf(exit);
  const sameFamily =
    exit === null
      ? observed
      : // 出口 IP 认不出协议族时**一个都不留**：`familyOf` 两边同为 null 不是「同族」，
        // 否则两个 STUN 报出同一个不可解析的串就会一路走到第 6 行判命中。
        observed.filter(
          (ip) => exitFamily !== null && familyOf(ip) === exitFamily,
        );
  if (sameFamily.length < 2) {
    return done({ comparable: false, reason: "familyMismatch" });
  }

  // 第 3 行。
  if (exit === null)
    return done({ comparable: false, reason: "unknownExitIp" });

  // 第 4 行。多出口集群与对称 NAT 会自己落进这里——让误报源在对照里自曝，
  // 比用 /24 之类的启发式去猜它们要诚实。这是无从比对，不是未命中。
  const [reflexiveIp] = sameFamily;
  if (sameFamily.some((ip) => ip !== reflexiveIp)) {
    return done({ comparable: false, reason: "stunDisagree" });
  }

  // 第 5 / 6 行。
  return done({
    comparable: true,
    mismatch: reflexiveIp !== exit,
    reflexiveIp,
    exitIp: exit,
  });
}

function done(data: UdpEgressResult): OnlineCheck<UdpEgressResult> {
  return { status: "done", data };
}
