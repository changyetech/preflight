// 综合结论判定（规格 3.1 / 3.3，ADR-0005）。
//
// 最关键的一条是「未含 O4 时永不为高」：初步结论的取值域只有低与中。
// 一个不带 IP 风险评分却敢报「高」的结论，等于把唯一能触发高风险的输入凭空捏造出来。

import { describe, expect, it } from "vitest";

import type { VerdictInput } from "../src/domain/verdict";
import { computeVerdict, verdictInputFrom } from "../src/domain/verdict";

/** 全清的基线输入：无任何风险信号，且未含 O4。 */
const CLEAN: VerdictInput = {
  timezoneMismatch: false,
  ipv6Leak: false,
  risk: null,
};

/** 已含 O4 且 proxycheck 干净的基线输入。 */
const WITH_RISK: VerdictInput = {
  ...CLEAN,
  risk: { riskScore: 0, abuseListed: false },
};

describe("computeVerdict", () => {
  it("全清 → 低", () => {
    expect(computeVerdict(WITH_RISK)).toEqual({ stage: "full", level: "low" });
  });

  it("风险分 ≥ 70 → 高", () => {
    expect(
      computeVerdict({ ...WITH_RISK, risk: { riskScore: 70, abuseListed: false } }),
    ).toEqual({ stage: "full", level: "high" });
  });

  it("风险分 69 未达阈值，不判高", () => {
    expect(
      computeVerdict({ ...WITH_RISK, risk: { riskScore: 69, abuseListed: false } }),
    ).toEqual({ stage: "full", level: "low" });
  });

  it("时区不一致 → 中", () => {
    expect(computeVerdict({ ...WITH_RISK, timezoneMismatch: true })).toEqual({
      stage: "full",
      level: "medium",
    });
  });

  it("IPv6 泄露 → 中（ADR-0006）", () => {
    expect(computeVerdict({ ...WITH_RISK, ipv6Leak: true })).toEqual({
      stage: "full",
      level: "medium",
    });
  });

  it("StopForumSpam 有滥用收录 → 中", () => {
    expect(
      computeVerdict({ ...WITH_RISK, risk: { riskScore: 0, abuseListed: true } }),
    ).toEqual({ stage: "full", level: "medium" });
  });

  it("滥用收录未知（第三方不可用）不贡献风险", () => {
    expect(
      computeVerdict({ ...WITH_RISK, risk: { riskScore: 0, abuseListed: null } }),
    ).toEqual({ stage: "full", level: "low" });
  });

  it("未含 O4 时永不为高：即便中风险信号全中，也只到中", () => {
    const allMedium: VerdictInput = {
      timezoneMismatch: true,
      ipv6Leak: true,
      risk: null,
    };

    expect(computeVerdict(allMedium)).toEqual({
      stage: "preliminary",
      level: "medium",
    });
    expect(computeVerdict(CLEAN)).toEqual({
      stage: "preliminary",
      level: "low",
    });
  });

  it("高风险优先于中风险信号", () => {
    expect(
      computeVerdict({
        timezoneMismatch: true,
        ipv6Leak: true,
        risk: { riskScore: 100, abuseListed: true },
      }),
    ).toEqual({ stage: "full", level: "high" });
  });

  it("Hosting / 代理检出只是分项提醒，不拉高综合结论（规格 3.2）", () => {
    // 分项颜色由 networkType 与 proxy/vpn 决定，它们根本不进 VerdictInput——
    // 这条断言守的是「输入面只有四个信号」这个设计。
    expect(Object.keys(WITH_RISK).sort()).toEqual([
      "ipv6Leak",
      "risk",
      "timezoneMismatch",
    ]);
  });
});

describe("verdictInputFrom", () => {
  it("O4 未触发时 risk 为 null，结论停在初步", () => {
    const input = verdictInputFrom({
      o1: { status: "running" },
      o2: { status: "done", data: { browserTimezone: "Asia/Shanghai", exitTimezone: "Asia/Shanghai", match: true } },
      o3: { status: "done", data: { leak: false, ipv6: null } },
      o4: { status: "idle" },
    });

    expect(input).toEqual({
      timezoneMismatch: false,
      ipv6Leak: false,
      risk: null,
    });
    expect(computeVerdict(input).stage).toBe("preliminary");
  });

  it("检测失败的项不贡献风险信号——失败不等于安全，也不等于有风险", () => {
    const input = verdictInputFrom({
      o1: { status: "failed", reason: "network" },
      o2: { status: "failed", reason: "network" },
      o3: { status: "failed", reason: "ipify unreachable" },
      o4: { status: "failed", reason: "upstream unavailable" },
    });

    expect(input).toEqual({
      timezoneMismatch: false,
      ipv6Leak: false,
      risk: null,
    });
  });

  it("配额耗尽的 O4 不产生风险输入，结论保持初步（规格 5.3）", () => {
    const input = verdictInputFrom({
      o1: { status: "running" },
      o2: { status: "running" },
      o3: { status: "running" },
      o4: { status: "done", data: { status: "quotaExhausted" } },
    });

    expect(input.risk).toBeNull();
  });

  it("O4 完成后取出风险分与滥用收录", () => {
    const input = verdictInputFrom({
      o1: { status: "running" },
      o2: { status: "done", data: { browserTimezone: "Asia/Shanghai", exitTimezone: "America/New_York", match: false } },
      o3: { status: "done", data: { leak: true, ipv6: "2001:db8::1" } },
      o4: {
        status: "done",
        data: {
          status: "ok",
          ip: "1.2.3.4",
          networkType: "Hosting",
          proxy: true,
          vpn: true,
          tor: false,
          scraper: false,
          riskScore: 100,
          riskLevel: "high",
          abuseListed: true,
        },
      },
    });

    expect(input).toEqual({
      timezoneMismatch: true,
      ipv6Leak: true,
      risk: { riskScore: 100, abuseListed: true },
    });
  });

  it("时区无法比对（边缘未给出时区）不算不一致", () => {
    const input = verdictInputFrom({
      o1: { status: "running" },
      o2: { status: "done", data: { browserTimezone: "Asia/Shanghai", exitTimezone: null, match: null } },
      o3: { status: "running" },
      o4: { status: "idle" },
    });

    expect(input.timezoneMismatch).toBe(false);
  });
});
