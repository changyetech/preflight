// 覆盖度三分（规格 3.4，ADR-0004）：综合结论永远不得脱离覆盖度单独呈现。

import type { OnlineCheck, PanelState } from "./checks";
import { CLI_CHECK_IDS } from "./checks";
import type { RiskData } from "./types";

/**
 * 覆盖度分档。规格 3.4 列的是三档，这里多一档 `pending`：
 * 按需项 O4 在被触发前不属于任何一档，硬塞进「检测失败」是谎报故障，
 * 塞进「需 CLI」则把可解决的临时状态与永久状态混计（ADR-0004 明令禁止）。
 * 不变量：done + needCli + failed + pending ≡ 10。
 */
export type Coverage = {
  done: number;
  needCli: number;
  failed: number;
  /** 未开始或检测中 */
  pending: number;
};

type Bucket = "done" | "failed" | "pending";

function bucketOf(check: OnlineCheck<unknown>): Bucket {
  switch (check.status) {
    case "idle":
    case "running":
      return "pending";
    case "failed":
      return "failed";
    case "done":
      return "done";
  }
}

/**
 * O4 单独判：配额耗尽虽然是一次成功的 200 响应，却不是「已完成」——
 * 没有查询发生，就没有结果可报（规格 5.3 / docs/api.md 3.2）。
 *
 * 这个特例只属于 O4，所以写在 O4 自己的分支里，而不是去嗅探任意检测项数据上有没有
 * `status` 字段——否则日后哪个检测项的数据碰巧带 `status`，就会被误分到「检测失败」。
 */
function bucketOfRisk(check: OnlineCheck<RiskData>): Bucket {
  return check.status === "done" && check.data.status === "quotaExhausted"
    ? "failed"
    : bucketOf(check);
}

export function computeCoverage(panel: PanelState): Coverage {
  const coverage: Coverage = {
    done: 0,
    needCli: CLI_CHECK_IDS.length,
    failed: 0,
    pending: 0,
  };

  const buckets = [
    bucketOf(panel.o1),
    bucketOf(panel.o2),
    bucketOf(panel.o3),
    bucketOfRisk(panel.o4),
    // O5 的「无从比对」是**已完成**：探测成功了，只是回答里不含可判定的信息（契约 §2.5）。
    // 它落在 status 上，因此不需要像 O4 那样开特例。O6 同理（契约 §2.6 第 2–4 行）。
    bucketOf(panel.o5),
    bucketOf(panel.o6),
  ];

  for (const bucket of buckets) {
    coverage[bucket] += 1;
  }

  return coverage;
}
