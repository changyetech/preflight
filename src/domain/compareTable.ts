// Web 与 CLI 完整功能对照表的数据源（规格 4「落地内容」第 3 段 / 第 2 节总表）。
//
// 单一事实来源是契约第 1 节的 8 项总表；这里只是把它的 归属 / 执行 两列结构化，
// 标题文案仍取自 copy.ts 的 checks[id].title，避免同一个检测项名字出现两处、迟早写岔。

export type CompareOwner = "web" | "cli";
export type CompareExecution = "auto" | "onDemand" | "none";

export type CompareRow = {
  id: "O1" | "O2" | "O3" | "O4" | "C1" | "C2" | "C3" | "C4";
  owner: CompareOwner;
  execution: CompareExecution;
};

/** 顺序与契约第 1 节总表逐行一致：O1, O2, O3, O4, C1, C2, C3, C4。 */
export const COMPARE_TABLE: readonly CompareRow[] = [
  { id: "O1", owner: "web", execution: "auto" },
  { id: "O2", owner: "web", execution: "auto" },
  { id: "O3", owner: "web", execution: "auto" },
  { id: "O4", owner: "web", execution: "onDemand" },
  { id: "C1", owner: "cli", execution: "none" },
  { id: "C2", owner: "cli", execution: "none" },
  { id: "C3", owner: "cli", execution: "none" },
  { id: "C4", owner: "cli", execution: "none" },
] as const;
