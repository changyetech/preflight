// 覆盖度 10 格 meter 的格状态映射（规格 §4 要点 5）。
//
// 与 tests/coverage.test.ts 断言的是同一份不变量在另一粒度上的版本：那边锁的是
// done+needCli+failed+pending≡10，这里锁的是「格子总数恒为 10」——每条用例都要断言，
// 因为这是 meter 本任务里风险最高的一处：数错一格，用户看到的进度条就是假的。

import { describe, expect, it } from "vitest";

import type { CoverageCell } from "../src/coverageMeter";
import { coverageCells } from "../src/coverageMeter";
import type { PanelState } from "../src/domain/checks";

const GEO = {
  ip: "1.2.3.4",
  country: "CN",
  region: "Shanghai",
  city: "Shanghai",
  postalCode: null,
  continent: "AS",
  latitude: "31.2",
  longitude: "121.4",
  timezone: "Asia/Shanghai",
  asn: 4134,
  asOrganization: "Chinanet",
  colo: "SHA",
};

const TZ_OK = {
  browserTimezone: "Asia/Shanghai",
  exitTimezone: "Asia/Shanghai",
  match: true,
} as const;

const IPV6_OFF = { leak: false, ipv6: null } as const;

const DNS_EGRESS_OK = {
  resolverGeo: "Japan - Google LLC",
  comparison: {
    comparable: true,
    leak: false,
    ecsCountry: "CN",
    exitCountry: "CN",
  },
} as const;

const UDP_EGRESS_OK = {
  comparable: true,
  mismatch: false,
  reflexiveIp: "1.2.3.4",
  exitIp: "1.2.3.4",
} as const;

/** 首屏典型态：O1–O3 与 O5／O6 完成，O4 未触发。 */
const FIRST_SCREEN: PanelState = {
  o1: { status: "done", data: GEO },
  o2: { status: "done", data: TZ_OK },
  o3: { status: "done", data: IPV6_OFF },
  o4: { status: "idle" },
  o5: { status: "done", data: DNS_EGRESS_OK },
  o6: { status: "done", data: UDP_EGRESS_OK },
};

function count(cells: CoverageCell[], state: CoverageCell): number {
  return cells.filter((c) => c === state).length;
}

describe("coverageCells", () => {
  it("首屏典型态：5 已完成 + 1 按需未测 + 4 需 CLI，总格数恒为 10", () => {
    const cells = coverageCells(FIRST_SCREEN);

    expect(cells).toHaveLength(10);
    expect(count(cells, "done")).toBe(5);
    expect(count(cells, "ondemand")).toBe(1);
    expect(count(cells, "cli")).toBe(4);
    expect(count(cells, "running")).toBe(0);
    expect(count(cells, "failed")).toBe(0);
  });

  it("全部完成（点击 O4 后）：6 已完成 + 4 需 CLI，总格数恒为 10", () => {
    const cells = coverageCells({
      ...FIRST_SCREEN,
      o4: {
        status: "done",
        data: {
          status: "ok",
          ip: "1.2.3.4",
          networkType: "Residential",
          proxy: false,
          vpn: false,
          tor: false,
          scraper: false,
          riskScore: 0,
          riskLevel: "low",
          anonymous: false,
          abuseListed: false,
        },
      },
    });

    expect(cells).toHaveLength(10);
    expect(count(cells, "done")).toBe(6);
    expect(count(cells, "cli")).toBe(4);
    expect(count(cells, "ondemand")).toBe(0);
  });

  it("有检测失败项：失败格独立于「按需未测」，总格数恒为 10", () => {
    const cells = coverageCells({
      ...FIRST_SCREEN,
      o3: { status: "failed", reason: "ipify unreachable" },
    });

    expect(cells).toHaveLength(10);
    expect(count(cells, "failed")).toBe(1);
    expect(count(cells, "done")).toBe(4);
    expect(count(cells, "ondemand")).toBe(1);
    expect(count(cells, "cli")).toBe(4);
  });

  it("进行中（首屏刚加载）：running 与 ondemand 分属不同格状态，总格数恒为 10", () => {
    const cells = coverageCells({
      o1: { status: "running" },
      o2: { status: "running" },
      o3: { status: "running" },
      o4: { status: "idle" },
      o5: { status: "running" },
      o6: { status: "running" },
    });

    expect(cells).toHaveLength(10);
    expect(count(cells, "running")).toBe(5);
    expect(count(cells, "ondemand")).toBe(1);
    expect(count(cells, "cli")).toBe(4);
    expect(count(cells, "done")).toBe(0);
  });

  it("需 CLI 恒为 4 格，且顺序排在在线项之后（与原型 recompute() 拼接顺序一致）", () => {
    const cells = coverageCells(FIRST_SCREEN);

    expect(cells).toHaveLength(10);
    expect(cells.slice(6)).toEqual(["cli", "cli", "cli", "cli"]);
  });

  it("配额耗尽记为「检测失败」格，不是「已完成」（与 domain/coverage.ts 的 bucketOfRisk 同一判据）", () => {
    const cells = coverageCells({
      ...FIRST_SCREEN,
      o4: { status: "done", data: { status: "quotaExhausted" } },
    });

    expect(cells).toHaveLength(10);
    expect(count(cells, "failed")).toBe(1);
    expect(count(cells, "done")).toBe(5);
  });

  it("混合情形：完成 + 失败 + 进行中 + 按需未测同时出现，总格数恒为 10", () => {
    const cells = coverageCells({
      o1: { status: "done", data: GEO },
      o2: { status: "failed", reason: "x" },
      o3: { status: "running" },
      o4: { status: "idle" },
      o5: { status: "done", data: DNS_EGRESS_OK },
      o6: { status: "running" },
    });

    expect(cells).toHaveLength(10);
    expect(count(cells, "done")).toBe(2);
    expect(count(cells, "failed")).toBe(1);
    expect(count(cells, "running")).toBe(2);
    expect(count(cells, "ondemand")).toBe(1);
    expect(count(cells, "cli")).toBe(4);
  });
});
