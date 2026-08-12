// 综合结论判定（规格 3.1 / 3.3，ADR-0005）。
//
// 最关键的一条是「未含 O4 时永不为高」：初步结论的取值域只有低与中。
// 一个不带 IP 风险评分却敢报「高」的结论，等于把唯一能触发高风险的输入凭空捏造出来。

import { describe, expect, it } from "vitest";

import { INITIAL_PANEL } from "../src/domain/checks";
import type { GeoData } from "../src/domain/types";
import type { Verdict, VerdictInput } from "../src/domain/verdict";
import { computeVerdict, verdictInputFrom } from "../src/domain/verdict";

const GEO: GeoData = {
  ip: "1.2.3.4",
  country: "CN",
  region: "Shanghai",
  city: "Shanghai",
  postalCode: null,
  continent: "AS",
  latitude: "31.2",
  longitude: "121.4",
  timezone: "Asia/Shanghai",
  asn: 4134,
  asOrganization: "Chinanet",
  colo: "SHA",
};

/** 四个中档信号全未命中。 */
const NO_SIGNALS = {
  timezoneMismatch: false,
  ipv6Leak: false,
  dnsEgressLeak: false,
  udpEgressMismatch: false,
} as const;

/** O5／O6 尚未产出结果时的面板片段——它们不参与那几条用例，故恒为检测中。 */
const SPLIT_TUNNEL_PENDING = {
  o5: { status: "running" },
  o6: { status: "running" },
} as const;

/** 全清的基线输入：无任何风险信号，且未含 O4。 */
const CLEAN: VerdictInput = {
  signals: { ...NO_SIGNALS },
  risk: null,
};

/** 已含 O4 且 proxycheck 干净的基线输入。 */
const WITH_RISK: VerdictInput = {
  ...CLEAN,
  risk: { riskScore: 0, anonymous: false, abuseListed: false },
};

describe("computeVerdict", () => {
  it("全清 → 低", () => {
    expect(computeVerdict(WITH_RISK)).toEqual({ stage: "full", level: "low" });
  });

  // 判「高」的阈值是**二维**的（契约 §3.1）：anonymous 为假 ⇒ 76，为真 ⇒ 51。
  // 两个数取自 proxycheck v3 官方的 deny 边界。
  it("非匿名：风险分 ≥ 76 → 高", () => {
    expect(
      computeVerdict({
        ...WITH_RISK,
        risk: { riskScore: 76, anonymous: false, abuseListed: false },
      }),
    ).toEqual({ stage: "full", level: "high" });
  });

  it("非匿名：风险分 75 未达阈值，不判高", () => {
    // 75 是 TOR/Scraper 的基准分。光是被判为那几类、却没在充当匿名地址，还不够。
    expect(
      computeVerdict({
        ...WITH_RISK,
        risk: { riskScore: 75, anonymous: false, abuseListed: false },
      }),
    ).toEqual({ stage: "full", level: "low" });
  });

  it("匿名：阈值降到 51，风险分 51 → 高", () => {
    expect(
      computeVerdict({
        ...WITH_RISK,
        risk: { riskScore: 51, anonymous: true, abuseListed: false },
      }),
    ).toEqual({ stage: "full", level: "high" });
  });

  it("匿名：风险分 50（VPN 基准分）仍判低", () => {
    // 「在用 VPN」本身不构成高风险——那是本产品用户的常态。
    expect(
      computeVerdict({
        ...WITH_RISK,
        risk: { riskScore: 50, anonymous: true, abuseListed: false },
      }),
    ).toEqual({ stage: "full", level: "low" });
  });

  it("同一个分数，匿名与否结论不同", () => {
    const at70 = (anonymous: boolean) =>
      computeVerdict({
        ...WITH_RISK,
        risk: { riskScore: 70, anonymous, abuseListed: false },
      });
    expect(at70(false)).toEqual({ stage: "full", level: "low" });
    expect(at70(true)).toEqual({ stage: "full", level: "high" });
  });

  it("时区不一致 → 中", () => {
    expect(
      computeVerdict({
        ...WITH_RISK,
        signals: { ...NO_SIGNALS, timezoneMismatch: true },
      }),
    ).toEqual({
      stage: "full",
      level: "medium",
    });
  });

  it("IPv6 泄露 → 中（ADR-0006）", () => {
    expect(
      computeVerdict({
        ...WITH_RISK,
        signals: { ...NO_SIGNALS, ipv6Leak: true },
      }),
    ).toEqual({
      stage: "full",
      level: "medium",
    });
  });

  it("DNS 出口泄露 → 中（契约 §2.5，ADR-0014）", () => {
    expect(
      computeVerdict({
        ...WITH_RISK,
        signals: { ...NO_SIGNALS, dnsEgressLeak: true },
      }),
    ).toEqual({ stage: "full", level: "medium" });
  });

  it("UDP 出口不一致 → 中（契约 §2.6，ADR-0014）", () => {
    expect(
      computeVerdict({
        ...WITH_RISK,
        signals: { ...NO_SIGNALS, udpEgressMismatch: true },
      }),
    ).toEqual({ stage: "full", level: "medium" });
  });

  it("分流泄露两项都只是「中」，风险分缺席时也带不出「高」（契约 §3.2 不变量）", () => {
    // 「高只出现在 full 形态」靠「唯一的高档信号来自 O4」维持。两个新信号若给成「高」，
    // 一个确定的分流泄露会在 proxycheck 失败时被压成「初步 · 中」。
    expect(
      computeVerdict({
        signals: {
          ...NO_SIGNALS,
          dnsEgressLeak: true,
          udpEgressMismatch: true,
        },
        risk: null,
      }),
    ).toEqual({ stage: "preliminary", level: "medium" });
  });

  it("StopForumSpam 有滥用收录 → 中", () => {
    expect(
      computeVerdict({
        ...WITH_RISK,
        risk: { riskScore: 0, anonymous: false, abuseListed: true },
      }),
    ).toEqual({ stage: "full", level: "medium" });
  });

  it("滥用收录未知（第三方不可用）不贡献风险", () => {
    expect(
      computeVerdict({
        ...WITH_RISK,
        risk: { riskScore: 0, anonymous: false, abuseListed: null },
      }),
    ).toEqual({ stage: "full", level: "low" });
  });

  it("未含 O4 时永不为高：即便中风险信号全中，也只到中", () => {
    const allMedium: VerdictInput = {
      signals: { ...NO_SIGNALS, timezoneMismatch: true, ipv6Leak: true },
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
        signals: { ...NO_SIGNALS, timezoneMismatch: true, ipv6Leak: true },
        risk: { riskScore: 100, anonymous: false, abuseListed: true },
      }),
    ).toEqual({ stage: "full", level: "high" });
  });

  it("Hosting / 代理检出只是分项提醒，不拉高综合结论（规格 3.2）", () => {
    // 分项颜色由 networkType 与 proxy/vpn 决定，它们根本不进 VerdictInput——
    // 这条断言守的是「输入面只有这几个信号」这个设计。
    expect(Object.keys(WITH_RISK).sort()).toEqual(["risk", "signals"]);
    expect(Object.keys(WITH_RISK.signals!).sort()).toEqual([
      "dnsEgressLeak",
      "ipv6Leak",
      "timezoneMismatch",
      "udpEgressMismatch",
    ]);
  });

  it("一个信号都没有时判「数据不足」，绝不判低风险", () => {
    // 「什么都没测成」与「测了都没问题」在结论区必须长得不一样。
    const verdict = computeVerdict({ signals: null, risk: null });

    expect(verdict).toEqual({ stage: "insufficient" });
    expect(verdict).not.toHaveProperty("level");
  });

  it("数据不足形态在类型层面就没有 level 字段", () => {
    const verdict: Verdict = { stage: "insufficient" };

    // @ts-expect-error 「数据不足」不带任何风险档位，渲染不出「低风险」
    expect(verdict.level).toBeUndefined();
  });

  it("O4 已出结果时不判数据不足——有证据就该给结论，哪怕 O2/O3 没跑成", () => {
    expect(
      computeVerdict({
        signals: null,
        risk: { riskScore: 100, anonymous: false, abuseListed: null },
      }),
    ).toEqual({ stage: "full", level: "high" });
  });
});

describe("verdictInputFrom", () => {
  it("O4 未触发时 risk 为 null，结论停在初步", () => {
    const input = verdictInputFrom({
      o1: { status: "running" },
      o2: {
        status: "done",
        data: {
          browserTimezone: "Asia/Shanghai",
          exitTimezone: "Asia/Shanghai",
          match: true,
        },
      },
      o3: { status: "done", data: { leak: false, ipv6: null } },
      o4: { status: "idle" },
      ...SPLIT_TUNNEL_PENDING,
    });

    expect(input).toEqual({ signals: { ...NO_SIGNALS }, risk: null });
    expect(computeVerdict(input).stage).toBe("preliminary");
  });

  it("自动检测项全部失败 → 数据不足，绝不是低风险", () => {
    const input = verdictInputFrom({
      o1: { status: "failed", reason: "network" },
      o2: { status: "failed", reason: "network" },
      o3: { status: "failed", reason: "ipify unreachable" },
      o4: { status: "failed", reason: "upstream unavailable" },
      o5: { status: "failed", reason: "dns-egress-probe-failed" },
      o6: { status: "failed", reason: "udp-egress-stun-unanswered" },
    });

    expect(input).toEqual({ signals: null, risk: null });
    expect(computeVerdict(input)).toEqual({ stage: "insufficient" });
  });

  it("首帧全部检测中 → 数据不足，绝不是低风险", () => {
    expect(computeVerdict(verdictInputFrom(INITIAL_PANEL))).toEqual({
      stage: "insufficient",
    });
  });

  it("只要有一项自动检测出了结果，就给初步结论——失败项不贡献信号", () => {
    const input = verdictInputFrom({
      o1: { status: "done", data: GEO },
      o2: {
        status: "done",
        data: {
          browserTimezone: "Asia/Shanghai",
          exitTimezone: "America/New_York",
          match: false,
        },
      },
      o3: { status: "failed", reason: "ipify unreachable" },
      o4: { status: "idle" },
      ...SPLIT_TUNNEL_PENDING,
    });

    expect(input.signals).toEqual({ ...NO_SIGNALS, timezoneMismatch: true });
    expect(computeVerdict(input)).toEqual({
      stage: "preliminary",
      level: "medium",
    });
  });

  it("配额耗尽的 O4 不产生风险输入，结论保持初步（规格 5.3）", () => {
    const input = verdictInputFrom({
      o1: { status: "done", data: GEO },
      o2: {
        status: "done",
        data: {
          browserTimezone: "Asia/Shanghai",
          exitTimezone: "Asia/Shanghai",
          match: true,
        },
      },
      o3: { status: "done", data: { leak: false, ipv6: null } },
      o4: { status: "done", data: { status: "quotaExhausted" } },
      ...SPLIT_TUNNEL_PENDING,
    });

    expect(input.risk).toBeNull();
    expect(computeVerdict(input)).toEqual({
      stage: "preliminary",
      level: "low",
    });
  });

  it("O4 完成后取出风险分与滥用收录", () => {
    const input = verdictInputFrom({
      o1: { status: "running" },
      o2: {
        status: "done",
        data: {
          browserTimezone: "Asia/Shanghai",
          exitTimezone: "America/New_York",
          match: false,
        },
      },
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
          anonymous: false,
          abuseListed: true,
        },
      },
      ...SPLIT_TUNNEL_PENDING,
    });

    expect(input).toEqual({
      signals: { ...NO_SIGNALS, timezoneMismatch: true, ipv6Leak: true },
      risk: { riskScore: 100, anonymous: false, abuseListed: true },
    });
  });

  it("O1–O3 失败 + O5／O6 已完成但无从比对 → 数据不足，绝不是「初步 · 低」", () => {
    // O1 失败（/api/geo 不可用或限流）时的**真实**面板形状：O5／O6 自己的探测成功了，
    // 但没有出口国／出口 IP 可比，因此 status 是 done 而 comparable 是 false。
    // 「产出了结果」与「产出了信号」是两件事——把 done 当成有信号，会让一条都比不出来的
    // 面板给出绿色的「未发现异常」，正是契约 §3.2 写在红线上的那类误报。
    const input = verdictInputFrom({
      o1: { status: "failed", reason: "network" },
      o2: { status: "failed", reason: "network" },
      o3: { status: "failed", reason: "ipify unreachable" },
      o4: { status: "idle" },
      o5: {
        status: "done",
        data: {
          resolverGeo: "Japan - Google LLC",
          comparison: { comparable: false, reason: "unknownExitCountry" },
        },
      },
      o6: {
        status: "done",
        data: { comparable: false, reason: "unknownExitIp" },
      },
    });

    expect(input.signals).toBeNull();
    expect(computeVerdict(input)).toEqual({ stage: "insufficient" });
  });

  it("O5／O6 的「无从比对」不贡献信号，与「未命中」同一个结论", () => {
    // 这是本功能最容易做错的一格：把「没测出来」渲染成绿色是本产品的红线，
    // 但在**综合结论**上，无从比对与未命中确实必须一致——差别只在该项自己的呈现。
    // 用一条真实存在的信号（O3 未泄露）垫底，否则整个面板会落进「数据不足」，
    // 那测的就是另一件事了。
    const panel = {
      o1: { status: "done", data: GEO },
      o2: { status: "running" },
      o3: { status: "done", data: { leak: false, ipv6: null } },
      o4: { status: "idle" },
    } as const;

    const incomparable = verdictInputFrom({
      ...panel,
      o5: {
        status: "done",
        data: {
          resolverGeo: "China - Alibaba",
          comparison: { comparable: false, reason: "noEcs" },
        },
      },
      o6: {
        status: "done",
        data: { comparable: false, reason: "stunDisagree" },
      },
    });

    const notLeaking = verdictInputFrom({
      ...panel,
      o5: {
        status: "done",
        data: {
          resolverGeo: "China - Alibaba",
          comparison: {
            comparable: true,
            leak: false,
            ecsCountry: "CN",
            exitCountry: "CN",
          },
        },
      },
      o6: {
        status: "done",
        data: {
          comparable: true,
          mismatch: false,
          reflexiveIp: "1.2.3.4",
          exitIp: "1.2.3.4",
        },
      },
    });

    expect(incomparable.signals).toEqual({ ...NO_SIGNALS });
    expect(incomparable).toEqual(notLeaking);
    expect(computeVerdict(incomparable)).toEqual({
      stage: "preliminary",
      level: "low",
    });
  });

  it("O5／O6 命中时信号为真，两个新信号各自独立", () => {
    const input = verdictInputFrom({
      o1: { status: "done", data: GEO },
      o2: { status: "running" },
      o3: { status: "running" },
      o4: { status: "idle" },
      o5: {
        status: "done",
        data: {
          resolverGeo: null,
          comparison: {
            comparable: true,
            leak: true,
            ecsCountry: "US",
            exitCountry: "JP",
          },
        },
      },
      o6: {
        status: "done",
        data: {
          comparable: true,
          mismatch: false,
          reflexiveIp: "1.2.3.4",
          exitIp: "1.2.3.4",
        },
      },
    });

    expect(input.signals).toEqual({ ...NO_SIGNALS, dnsEgressLeak: true });
    expect(computeVerdict(input)).toEqual({
      stage: "preliminary",
      level: "medium",
    });
  });

  it("时区无法比对（边缘未给出时区）不算不一致", () => {
    const input = verdictInputFrom({
      o1: { status: "running" },
      o2: {
        status: "done",
        data: {
          browserTimezone: "Asia/Shanghai",
          exitTimezone: null,
          match: null,
        },
      },
      o3: { status: "running" },
      o4: { status: "idle" },
      ...SPLIT_TUNNEL_PENDING,
    });

    expect(input.signals?.timezoneMismatch).toBe(false);
  });
});
