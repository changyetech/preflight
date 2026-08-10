// 功能对照表一致性（--content 计划步骤 3 / 验收标准）：
// 表格必须恰好 9 行，且与规格第 2 节总表逐项一致——顺序、归属都不能对不上。

import { describe, expect, it } from "vitest";

import { COMPARE_TABLE } from "../src/domain/compareTable";
import { COPY } from "../src/copy";

describe("Web 与 CLI 完整功能对照表", () => {
  it("恰好 9 行", () => {
    expect(COMPARE_TABLE).toHaveLength(9);
  });

  it("行顺序与规格第 2 节总表一致：O1, O2, O3, O4, C1, C2, C3, C4, C5", () => {
    expect(COMPARE_TABLE.map((row) => row.id)).toEqual([
      "O1",
      "O2",
      "O3",
      "O4",
      "C1",
      "C2",
      "C3",
      "C4",
      "C5",
    ]);
  });

  it("O 开头四项归属 web，C 开头五项归属 cli，与规格第 2 节总表的『归属』列一致", () => {
    for (const row of COMPARE_TABLE) {
      const expected = row.id.startsWith("O") ? "web" : "cli";
      expect(row.owner).toBe(expected);
    }
  });

  it("每一行都能在 copy.checks 里找到对应标题——不存在孤儿行", () => {
    for (const row of COMPARE_TABLE) {
      expect(COPY.checks[row.id].title).toBeTruthy();
    }
  });

  it("仅 O4 是按需执行，其余可在线项自动执行，仅 CLI 项没有执行方式（规格 2 / 4.1）", () => {
    const byId = Object.fromEntries(COMPARE_TABLE.map((r) => [r.id, r]));
    expect(byId.O1.execution).toBe("auto");
    expect(byId.O2.execution).toBe("auto");
    expect(byId.O3.execution).toBe("auto");
    expect(byId.O4.execution).toBe("onDemand");
    for (const id of ["C1", "C2", "C3", "C4", "C5"] as const) {
      expect(byId[id].execution).toBe("none");
    }
  });
});
