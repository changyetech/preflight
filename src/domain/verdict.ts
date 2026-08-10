// 综合结论判定（规格 3.1 / 3.3，ADR-0004 / ADR-0005）。

import type { PanelState } from "./checks";

/**
 * 综合结论的输入信号。
 *
 * `signals` 为 `null` 表示 O2、O3 都还没产出结果（都在检测中，或都失败了）——
 * 此时手里一个信号都没有。它必须与「两个信号都为 false」区分开：后者是「查过了，没问题」，
 * 前者是「根本没查成」。用可空对象而不是两个 boolean 平铺，正是为了让
 * 「没有任何证据却报低风险」这个状态在类型层面构造不出来。
 *
 * `risk` 为 `null` 表示 O4 尚未产出可用结果（未触发 / 检测中 / 失败 / 配额耗尽），
 * 此时结论恒为「初步」。把 riskScore 与 abuseListed 捆在同一个可空对象里，
 * 是为了让「风险分未知但滥用收录已知」这种不存在的组合无法被构造出来。
 *
 * 网络类型 Hosting、代理检出等只是分项提醒（规格 3.2），刻意不出现在这里。
 */
export type VerdictInput = {
  signals: { timezoneMismatch: boolean; ipv6Leak: boolean } | null;
  risk: { riskScore: number; abuseListed: boolean | null } | null;
};

/**
 * 三种形态：
 * - `insufficient`：一个信号都没有，**没有 level 字段**——类型层面就渲染不出「低风险」
 * - `preliminary`：取值域只有低与中（ADR-0005 的设计意图，不是缺陷）
 * - `full`：唯一可能取「高」的形态
 */
export type Verdict =
  | { stage: "insufficient" }
  | { stage: "preliminary"; level: "low" | "medium" }
  | { stage: "full"; level: "low" | "medium" | "high" };

/**
 * 综合结论判「高」的风险分阈值（规格 3.1）。
 *
 * 这里刻意不复用 `riskLevel`：那是**分项**分级（规格 3.2），语义上与综合结论是两件事，
 * 只是当前阈值恰好都是 70。分项颜色由卡片直接吃 `riskLevel`，综合结论看这个常量。
 */
const HIGH_RISK_SCORE = 70;

export function computeVerdict(input: VerdictInput): Verdict {
  // 一个信号都没有就不给结论。规格 3.3 说的是「O1–O3 完成后」给出初步结论，
  // 而「什么都没测成」与「测了都没问题」在屏幕上必须长得不一样——
  // 后者是绿字「未发现异常」，前者若也这么写，就是拿检测失败冒充安全。
  if (input.signals === null && input.risk === null) {
    return { stage: "insufficient" };
  }

  const medium =
    input.signals?.timezoneMismatch === true ||
    input.signals?.ipv6Leak === true;

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
    // O2、O3 一个都没产出结果时给 null，让 computeVerdict 判「数据不足」。
    signals:
      timezone === null && ipv6 === null
        ? null
        : {
            // match 为 null 表示无从比对，不算不一致：检测不出来不等于有问题。
            timezoneMismatch: timezone?.match === false,
            ipv6Leak: ipv6?.leak === true,
          },
    risk:
      risk?.status === "ok"
        ? { riskScore: risk.riskScore, abuseListed: risk.abuseListed }
        : null,
  };
}
