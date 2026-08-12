// O6 UDP 出口一致性判定（契约 §2.6 判定表）。
//
// 判定表按 N_ok（成功应答的 STUN 数）与 N_fam（其中与出口 IP 同协议族的数量）分层，
// 从上往下取第一条匹配的行。本文件逐行钉住它，外加协议族硬约束那条：
// 反射地址是 IPv6 而出口是 IPv4 **不得判命中**——那是 O3 已经在管的事。

import { describe, expect, it } from "vitest";

import type { UdpEgressResult } from "../src/domain/types";
import {
  judgeUdpEgress,
  UDP_EGRESS_STUN_UNANSWERED,
  UDP_EGRESS_WEBRTC_UNAVAILABLE,
} from "../src/domain/udpEgress";

const EXIT_V4 = "198.51.100.20";

function supported(reflexiveIps: string[]) {
  return { reflexiveIps, webrtcSupported: true };
}

function resultOf(
  probe: { reflexiveIps: string[]; webrtcSupported: boolean },
  exitIp: string | null,
): UdpEgressResult {
  const state = judgeUdpEgress(probe, exitIp);
  if (state.status !== "done") throw new Error("应为已完成");
  return state.data;
}

describe("judgeUdpEgress · 契约 §2.6 判定表", () => {
  it("第 1 行：N_ok < 2 → 检测失败", () => {
    // 单个 STUN 分不清「UDP 漏了」与「这个出口本来就是多地址集群」。
    for (const ips of [[], ["203.0.113.7"]]) {
      expect(judgeUdpEgress(supported(ips), EXIT_V4)).toEqual({
        status: "failed",
        reason: UDP_EGRESS_STUN_UNANSWERED,
      });
    }
  });

  it("第 1 行：浏览器禁用 WebRTC 同样是检测失败，不是绿色的「未泄露」", () => {
    // 禁用 WebRTC 只是把测量仪器关掉了，不是测出了没问题（契约 §2.6）。
    // 失败原因必须与「探测超时」可区分：前者刷新一万次也一样（契约 §5.6 呈现约束）。
    expect(
      judgeUdpEgress({ reflexiveIps: [], webrtcSupported: false }, EXIT_V4),
    ).toEqual({ status: "failed", reason: UDP_EGRESS_WEBRTC_UNAVAILABLE });
  });

  it("第 2 行：N_ok ≥ 2 但同族数 < 2 → 无从比对，且**不是**检测失败", () => {
    // 协议族筛选永远不会造成检测失败——它只会把结果推进「无从比对」（契约 §2.6）。
    expect(resultOf(supported(["2001:db8::1", EXIT_V4]), EXIT_V4)).toEqual({
      comparable: false,
      reason: "familyMismatch",
    });
  });

  it("第 2 行：反射地址全是 IPv6 而出口是 IPv4 → 无从比对，绝不判命中（协议族硬约束）", () => {
    // 判命中等于把 O3（IPv6 泄露）已经算过的同一个事实再算一次。
    expect(
      resultOf(supported(["2001:db8::1", "2001:db8::1"]), EXIT_V4),
    ).toEqual({ comparable: false, reason: "familyMismatch" });
  });

  it("第 3 行：出口 IP 未知（O1 未完成）→ 无从比对", () => {
    expect(resultOf(supported(["203.0.113.7", "203.0.113.7"]), null)).toEqual({
      comparable: false,
      reason: "unknownExitIp",
    });
  });

  it("第 4 行：两个 STUN 各报各的 → 无从比对，不是未命中，更不是命中", () => {
    // 多出口集群与对称 NAT 会自己落进这一行——用对照法让误报源自曝。
    expect(
      resultOf(supported(["203.0.113.7", "203.0.113.8"]), EXIT_V4),
    ).toEqual({ comparable: false, reason: "stunDisagree" });
  });

  it("第 5 行：一致且 = 出口 IP → 未命中", () => {
    expect(resultOf(supported([EXIT_V4, EXIT_V4]), EXIT_V4)).toEqual({
      comparable: true,
      mismatch: false,
      reflexiveIp: EXIT_V4,
      exitIp: EXIT_V4,
    });
  });

  it("第 6 行：一致且 ≠ 出口 IP → 命中，并带出泄露的地址", () => {
    expect(
      resultOf(supported(["203.0.113.7", "203.0.113.7"]), EXIT_V4),
    ).toEqual({
      comparable: true,
      mismatch: true,
      reflexiveIp: "203.0.113.7",
      exitIp: EXIT_V4,
    });
  });

  it("同族比对：IPv6 出口下，两个 IPv6 反射地址照常比", () => {
    expect(
      resultOf(supported(["2001:db8::1", "2001:DB8::1"]), "2001:db8::1"),
    ).toMatchObject({ comparable: true, mismatch: false });
  });

  it("第 1 行优先于第 2 行：一个超时 + 一个跨族，仍是检测失败", () => {
    // 成因可以并存，判定表因此按 N_ok / N_fam 两个计数分层，从上往下取第一条匹配的行。
    // 这一格若落进「无从比对」，就与契约的覆盖度归档相反了。
    expect(judgeUdpEgress(supported(["2001:db8::1"]), EXIT_V4).status).toBe(
      "failed",
    );
  });
});
