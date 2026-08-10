// 覆盖度三分（规格 3.4，ADR-0004）：综合结论永远不得脱离覆盖度单独呈现。

import type { OnlineCheck, PanelState } from "./checks";
import { CLI_CHECK_IDS } from "./checks";

/**
 * 覆盖度分档。规格 3.4 列的是三档，这里多一档 `pending`：
 * 按需项 O4 在被触发前不属于任何一档，硬塞进「检测失败」是谎报故障，
 * 塞进「需 CLI」则把可解决的临时状态与永久状态混计（ADR-0004 明令禁止）。
 * 不变量：done + needCli + failed + pending ≡ 9。
 */
export type Coverage = {
  done: number;
  needCli: number;
  failed: number;
  /** 未开始或检测中 */
  pending: number;
};

type Bucket = "done" | "failed" | "pending";

/** 配额耗尽不是「已完成」：没有查询发生，就没有结果可报（规格 5.3 / docs/api.md 3.2）。 */
function bucketOf(check: OnlineCheck<unknown>): Bucket {
  switch (check.status) {
    case "idle":
    case "running":
      return "pending";
    case "failed":
      return "failed";
    case "done":
      return (check.data as { status?: string } | null)?.status ===
        "quotaExhausted"
        ? "failed"
        : "done";
  }
}

export function computeCoverage(panel: PanelState): Coverage {
  const coverage: Coverage = {
    done: 0,
    needCli: CLI_CHECK_IDS.length,
    failed: 0,
    pending: 0,
  };

  for (const check of [panel.o1, panel.o2, panel.o3, panel.o4]) {
    coverage[bucketOf(check)] += 1;
  }

  return coverage;
}
