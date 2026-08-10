// 综合结论判定（规格 3.1 / 3.3，ADR-0004 / ADR-0005）。

import type { PanelState } from "./checks";

/**
 * 综合结论的输入信号。只有四个：两个自动项信号，加上按需项 O4 的两个字段。
 *
 * `risk` 为 `null` 表示 O4 尚未产出可用结果（未触发 / 检测中 / 失败 / 配额耗尽），
 * 此时结论恒为「初步」。把 riskScore 与 abuseListed 捆在同一个可空对象里，
 * 是为了让「风险分未知但滥用收录已知」这种不存在的组合无法被构造出来。
 *
 * 网络类型 Hosting、代理检出等只是分项提醒（规格 3.2），刻意不出现在这里。
 */
export type VerdictInput = {
  timezoneMismatch: boolean;
  ipv6Leak: boolean;
  risk: { riskScore: number; abuseListed: boolean | null } | null;
};

/**
 * 初步结论的取值域只有低与中——这是 ADR-0005 的设计意图，不是缺陷。
 * 用联合类型表达，使「初步 + 高」在类型层面就不成立。
 */
export type Verdict =
  | { stage: "preliminary"; level: "low" | "medium" }
  | { stage: "full"; level: "low" | "medium" | "high" };

/** 风险分分级阈值（规格 3.2）：< 30 低 / < 70 中 / ≥ 70 高。 */
export const HIGH_RISK_SCORE = 70;

export function computeVerdict(input: VerdictInput): Verdict {
  const medium = input.timezoneMismatch || input.ipv6Leak;

  if (input.risk === null) {
    return { stage: "preliminary", level: medium ? "medium" : "low" };
  }

  if (input.risk.riskScore >= HIGH_RISK_SCORE) {
    return { stage: "full", level: "high" };
  }

  // 滥用收录未知（第三方不可用）按「不贡献信号」处理：未知不是有收录，也不该冒充无收录。
  return {
    stage: "full",
    level: medium || input.risk.abuseListed === true ? "medium" : "low",
  };
}

export function verdictInputFrom(panel: PanelState): VerdictInput {
  const timezone = panel.o2.status === "done" ? panel.o2.data : null;
  const ipv6 = panel.o3.status === "done" ? panel.o3.data : null;
  const risk = panel.o4.status === "done" ? panel.o4.data : null;

  return {
    // match 为 null 表示无从比对，不算不一致：检测不出来不等于有问题。
    timezoneMismatch: timezone?.match === false,
    ipv6Leak: ipv6?.leak === true,
    risk:
      risk?.status === "ok"
        ? { riskScore: risk.riskScore, abuseListed: risk.abuseListed }
        : null,
  };
}
