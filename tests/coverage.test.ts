// 覆盖度计算（规格 3.4，ADR-0004）。
//
// 分母恒为 8。规格 3.4 只列了「已完成 / 需 CLI / 检测失败」三档，但按需项 O4 在被触发前
// 既没完成、也没失败、更不是需 CLI——三档装不下它。故模型多留一档「按需未测」，
// 由 done + needCli + failed + pending ≡ 8 这条不变量兜底；pending 归零时三档之和自然恒为 8。
// 把未触发的 O4 塞进「检测失败」会谎报故障，塞进「需 CLI」会与永久性状态混计（ADR-0004 明令禁止）。

import { describe, expect, it } from "vitest";

import type { PanelState } from "../src/domain/checks";
import { TOTAL_CHECKS } from "../src/domain/checks";
import { computeCoverage } from "../src/domain/coverage";

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

/** 首屏典型态：O1–O3 完成，O4 未触发。 */
const FIRST_SCREEN: PanelState = {
  o1: { status: "done", data: GEO },
  o2: { status: "done", data: TZ_OK },
  o3: { status: "done", data: IPV6_OFF },
  o4: { status: "idle" },
};

describe("computeCoverage", () => {
  it("四档之和恒为 8，需 CLI 恒为 4", () => {
    const panels: PanelState[] = [
      {
        o1: { status: "running" },
        o2: { status: "running" },
        o3: { status: "running" },
        o4: { status: "idle" },
      },
      FIRST_SCREEN,
      {
        ...FIRST_SCREEN,
        o3: { status: "failed", reason: "ipify unreachable" },
      },
      { ...FIRST_SCREEN, o4: { status: "running" } },
      {
        ...FIRST_SCREEN,
        o4: { status: "failed", reason: "upstream unavailable" },
      },
      {
        o1: { status: "failed", reason: "x" },
        o2: { status: "failed", reason: "x" },
        o3: { status: "failed", reason: "x" },
        o4: { status: "failed", reason: "x" },
      },
    ];

    for (const panel of panels) {
      const c = computeCoverage(panel);
      expect(c.done + c.needCli + c.failed + c.pending).toBe(TOTAL_CHECKS);
      expect(c.needCli).toBe(4);
    }
  });

  it("首屏：已完成 3 · 需 CLI 4 · 检测失败 0 · 按需未测 1", () => {
    expect(computeCoverage(FIRST_SCREEN)).toEqual({
      done: 3,
      needCli: 4,
      failed: 0,
      pending: 1,
    });
  });

  it("点击 O4 后已完成由 3 变 4，pending 归零，三档之和恒为 8（验收标准 5）", () => {
    const c = computeCoverage({
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

    expect(c).toEqual({ done: 4, needCli: 4, failed: 0, pending: 0 });
    expect(c.done + c.needCli + c.failed).toBe(TOTAL_CHECKS);
  });

  it("失败项独立成档，绝不与「需 CLI」混计（ADR-0004）", () => {
    const c = computeCoverage({
      ...FIRST_SCREEN,
      o3: { status: "failed", reason: "ipify unreachable" },
    });

    expect(c.failed).toBe(1);
    expect(c.needCli).toBe(4);
    expect(c.done).toBe(2);
  });

  it("配额耗尽计入「检测失败」而非「已完成」（规格 5.3）", () => {
    const c = computeCoverage({
      ...FIRST_SCREEN,
      o4: { status: "done", data: { status: "quotaExhausted" } },
    });

    expect(c).toEqual({ done: 3, needCli: 4, failed: 1, pending: 0 });
  });

  it("检测中计入 pending，不预支为已完成", () => {
    expect(
      computeCoverage({ ...FIRST_SCREEN, o2: { status: "running" } }),
    ).toEqual({ done: 2, needCli: 4, failed: 0, pending: 2 });
  });
});
