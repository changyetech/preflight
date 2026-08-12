// 检测项状态模型（规格第 2 节总表 / 4.1 卡片五态）。
//
// 五态 = 未开始 / 检测中 / 已完成 / 检测失败 / 需 CLI，但这五态并非平铺在一个枚举上：
// 「需 CLI」是仅 CLI 项的**终态**，可在线项永远取不到；反过来仅 CLI 项也永远不可能「检测中」。
// 因此用两个互不相交的类型分别建模，让「仅 CLI 项处于检测中」这类非法状态在类型层面无法构造，
// 而不是用一个 status 枚举加可选字段去约定。

import type { GeoData, Ipv6Result, RiskData, TimezoneResult } from "./types";

/** 可在线检测项。 */
export const ONLINE_CHECK_IDS = ["O1", "O2", "O3", "O4"] as const;
/** 仅 CLI 项。C5（原厂商端点检测）已移除，编号废弃不复用（ADR-0013）。 */
export const CLI_CHECK_IDS = ["C1", "C2", "C3", "C4"] as const;

/** 覆盖度的分母恒为 8（契约第 1 节）。 */
export const TOTAL_CHECKS = ONLINE_CHECK_IDS.length + CLI_CHECK_IDS.length;

/** 可在线项的四态。「需 CLI」不在其取值域内。 */
export type OnlineCheck<T> =
  | { status: "idle" }
  | { status: "running" }
  | { status: "done"; data: T }
  | { status: "failed"; reason: string };

/** 仅 CLI 项的唯一状态，终态，不提供重试（规格 4.1）。 */
export type CliCheck = { status: "needCli" };

export const CLI_CHECK: CliCheck = { status: "needCli" };

/** 整个面板的状态。仅 CLI 项没有运行时状态，故不入此结构——它们恒为 CLI_CHECK。 */
export type PanelState = {
  o1: OnlineCheck<GeoData>;
  o2: OnlineCheck<TimezoneResult>;
  o3: OnlineCheck<Ipv6Result>;
  o4: OnlineCheck<RiskData>;
};

export const INITIAL_PANEL: PanelState = {
  o1: { status: "running" },
  o2: { status: "running" },
  o3: { status: "running" },
  // O4 是按需检测项，未经用户显式触发不会执行（ADR-0008）。
  o4: { status: "idle" },
};
