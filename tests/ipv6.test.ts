// O3 IPv6 泄露的对照实验（规格 2.3 判定表，ADR-0003）。
//
// 浏览器把 CORS 失败与网络失败抛成同一个不透明 TypeError，所以必须有 v4 对照：
// 没有对照就分不清「用户没有 IPv6」和「ipify 挂了」。这两格写反的代价不对称——
// 把断网的人报成「IPv6 未启用（无风险）」，等于把没测出来说成测过了且安全。

import { describe, expect, it } from "vitest";

import { judgeIpv6 } from "../src/domain/ipv6";

const V4_OK = { reachable: true, ip: "1.2.3.4" } as const;
const V6_OK = { reachable: true, ip: "2001:db8::1" } as const;
const UNREACHABLE = { reachable: false } as const;

describe("judgeIpv6", () => {
  it("v4 通 + v6 通 → IPv6 泄露，且带出 IPv6 地址", () => {
    expect(judgeIpv6(V4_OK, V6_OK)).toEqual({
      status: "done",
      data: { leak: true, ipv6: "2001:db8::1" },
    });
  });

  it("v4 通 + v6 不通 → IPv6 未启用（已完成、无风险）", () => {
    expect(judgeIpv6(V4_OK, UNREACHABLE)).toEqual({
      status: "done",
      data: { leak: false, ipv6: null },
    });
  });

  it("v4 不通 + v6 不通 → 检测失败，而非「IPv6 未启用」", () => {
    const state = judgeIpv6(UNREACHABLE, UNREACHABLE);

    expect(state.status).toBe("failed");
    // 这一格是整张表最容易写反的：判成「未启用」会把断网用户报成安全。
    expect(state).not.toEqual({
      status: "done",
      data: { leak: false, ipv6: null },
    });
  });

  it("v4 不通 + v6 通 → 异常，按第三方故障处理，同样是检测失败", () => {
    expect(judgeIpv6(UNREACHABLE, V6_OK).status).toBe("failed");
  });

  it("检测失败带可读原因，供失败卡展示与重试", () => {
    const state = judgeIpv6(UNREACHABLE, UNREACHABLE);

    if (state.status !== "failed") throw new Error("应为检测失败");
    expect(state.reason.length).toBeGreaterThan(0);
  });
});
