// O2 系统时区一致性（规格 2.2）。
// 比对两个 IANA 时区名：浏览器（= 系统）时区 vs 出口 IP 时区。

import { describe, expect, it } from "vitest";

import { compareTimezone } from "../src/domain/timezone";

describe("compareTimezone", () => {
  it("两侧一致 → match", () => {
    expect(compareTimezone("Asia/Shanghai", "Asia/Shanghai")).toEqual({
      browserTimezone: "Asia/Shanghai",
      exitTimezone: "Asia/Shanghai",
      match: true,
    });
  });

  it("两侧不一致 → 不一致（中风险信号）", () => {
    expect(compareTimezone("Asia/Shanghai", "America/New_York")).toEqual({
      browserTimezone: "Asia/Shanghai",
      exitTimezone: "America/New_York",
      match: false,
    });
  });

  it("出口时区缺失时判「无法比对」，不判不一致", () => {
    // 边缘没给时区就说「你的时区对不上」，是拿自己的数据缺口去指控用户。
    expect(compareTimezone("Asia/Shanghai", null)).toEqual({
      browserTimezone: "Asia/Shanghai",
      exitTimezone: null,
      match: null,
    });
  });

  it("比对不区分大小写与首尾空白：IANA 名同义写法不算不一致", () => {
    expect(compareTimezone(" Asia/Shanghai ", "asia/shanghai").match).toBe(
      true,
    );
  });
});
