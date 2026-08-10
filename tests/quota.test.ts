// proxycheck 日配额守卫（单实例 Durable Object，SQLite 后端，按 UTC 日重置）。
// 见 ADR-0002：Rate Limiting 绑定是最终一致的，扛不住全球总量硬上限，所以硬配额必须走强一致的 DO。
import { env, runInDurableObject } from "cloudflare:test";
import { describe, expect, it } from "vitest";

import { DAILY_LIMIT, quotaStub, utcDay } from "../worker/quota";

/** 直接改写 DO 内的计数，用于把状态摆到「当日已用 N 次」再断言 consume 的行为。 */
async function seed(day: string, used: number) {
  await runInDurableObject(quotaStub(env), (_instance, state) => {
    state.storage.sql.exec(
      "INSERT OR REPLACE INTO quota (id, day, used) VALUES (1, ?, ?)",
      day,
      used,
    );
  });
}

async function usedCount(): Promise<number> {
  return runInDurableObject(quotaStub(env), (_instance, state) => {
    const rows = state.storage.sql
      .exec<{ used: number }>("SELECT used FROM quota WHERE id = 1")
      .toArray();
    return rows[0]?.used ?? 0;
  });
}

describe("utcDay", () => {
  it("按 UTC 而非本地时区取日期", () => {
    expect(utcDay(new Date("2026-08-10T23:59:59Z"))).toBe("2026-08-10");
    expect(utcDay(new Date("2026-08-11T00:00:00Z"))).toBe("2026-08-11");
  });
});

describe("QuotaCounter.consume", () => {
  it("未达上限时放行并累加计数", async () => {
    await seed("2026-08-10", DAILY_LIMIT - 1);

    await expect(quotaStub(env).consume("2026-08-10")).resolves.toBe(true);
    await expect(usedCount()).resolves.toBe(DAILY_LIMIT);
  });

  it("达上限时拒绝且不再累加", async () => {
    await seed("2026-08-10", DAILY_LIMIT);

    await expect(quotaStub(env).consume("2026-08-10")).resolves.toBe(false);
    await expect(usedCount()).resolves.toBe(DAILY_LIMIT);
  });

  it("跨 UTC 日时计数归零重新放行", async () => {
    await seed("2026-08-10", DAILY_LIMIT);

    await expect(quotaStub(env).consume("2026-08-11")).resolves.toBe(true);
    await expect(usedCount()).resolves.toBe(1);
  });
});
