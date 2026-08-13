// 覆盖度 10 格 meter 的格状态映射（规格 §4 要点 5，原型 .cov-meter）。
//
// 不复用 domain/coverage.ts 的 Coverage 汇总：Coverage.pending 把「按需未测」（O4 未触发）
// 与「检测中」合并计成一档（四档之和恒为 10 这条不变量只需要这一层粒度），但 meter 的格子
// 要把二者渲染成不同颜色（原型 is-ondemand 与 is-running），所以要素比 Coverage 细一档，
// 只能直接从 PanelState 的原始 status 派生，而不是从已经合并过的 Coverage 倒推。
// 这不是「另起一套计数」——没有新增计数逻辑，只是把同一份 status 映射成另一种粒度的呈现。

import type { OnlineCheck, PanelState } from "./domain/checks";
import { CLI_CHECK_IDS } from "./domain/checks";
import type { RiskData } from "./domain/types";

/** meter 单格状态，对应原型 .cov-cell 的五个修饰类（is-done/is-running/is-failed/is-ondemand/is-cli）。 */
export type CoverageCell = "done" | "running" | "failed" | "ondemand" | "cli";

function cellOf(check: OnlineCheck<unknown>): CoverageCell {
  switch (check.status) {
    case "idle":
      return "ondemand";
    case "running":
      return "running";
    case "failed":
      return "failed";
    case "done":
      return "done";
  }
}

/**
 * O4 配额耗尽特例，与 domain/coverage.ts 的 bucketOfRisk 同一判据：
 * 状态是 done，但没有查询发生，格子要显示成「检测失败」而不是「已完成」。
 * bucketOfRisk 未导出，且这里只是复用同一条最小判据来选格子颜色（不改变判定本身），
 * 因此就地重复这一行，而不是导出 domain 内部函数或改 domain（本任务范围不许碰 domain）。
 */
function cellOfRisk(check: OnlineCheck<RiskData>): CoverageCell {
  return check.status === "done" && check.data.status === "quotaExhausted"
    ? "failed"
    : cellOf(check);
}

/**
 * PanelState → 10 格状态数组，顺序 O1–O6 + 4×需 CLI（与原型 recompute() 的拼接顺序一致）。
 * 不变量：数组长度恒为 10（契约 §4 的 X+Y+Z+W=10）——CLI_CHECK_IDS 恒为 4 项，
 * 加上 6 个在线项，长度由类型结构保证，不需要运行时再校验。
 */
export function coverageCells(panel: PanelState): CoverageCell[] {
  return [
    cellOf(panel.o1),
    cellOf(panel.o2),
    cellOf(panel.o3),
    cellOfRisk(panel.o4),
    cellOf(panel.o5),
    cellOf(panel.o6),
    ...CLI_CHECK_IDS.map((): CoverageCell => "cli"),
  ];
}
