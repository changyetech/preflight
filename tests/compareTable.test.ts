// 功能对照表数据一致性（--content 计划步骤 3 / 验收标准）：
// COMPARE_TABLE 必须恰好 9 行，顺序、归属与规格第 2 节总表当前的取值逐项一致。
//
// 这只是数据层回归锁，锁的是「改动 compareTable.ts 时是否还跟规格总表对得上」，
// 不是规格与代码的机械双向绑定——规格文档改了，这些测试不会自动变红，也测不出
// 「数据对、但渲染出来的 HTML 错了」这类呈现层缺陷（评审 I3：曾经的报告把这条
// 表述成「钉住了与规格总表的一致性」，是夸大）。呈现层的等价断言见
// tests/landing.test.ts 的「对照表 CLI 列」用例。

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
