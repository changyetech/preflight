// proxycheck 日配额守卫：单实例 Durable Object（SQLite 后端），按 UTC 日重置。
//
// 为什么必须是 DO 而不是 Rate Limiting 绑定：后者按数据中心分别计数、最终一致，官方明说
// 「不应作为精确记账系统」，挡得住单点猛刷但挡不住全球总量硬上限（ADR-0002）。
//
// 隐私：这里只存「日期 + 当日计数」两个标量，不含 IP、不含任何用户数据（ADR-0008）。

import { DurableObject } from "cloudflare:workers";

import type { Env } from "./env";

/**
 * proxycheck 注册免费版每日 1,000 次（ADR-0007）。
 * 这个常量绑死在账户档位上：改 proxycheck 账户档位（免费 1,000 → 付费 10,000+）时必须同步改这里，
 * 否则要么白白闲置已付费的额度，要么在本地放行后撞上上游的硬拒绝。
 */
export const DAILY_LIMIT = 1000;

/** 单实例：全站共用一个计数器，名字固定。 */
const SINGLETON = "global";

export function quotaStub(env: Env) {
  return env.QUOTA.get(env.QUOTA.idFromName(SINGLETON));
}

export function utcDay(now: Date): string {
  return now.toISOString().slice(0, 10);
}

export class QuotaCounter extends DurableObject<Env> {
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    ctx.storage.sql.exec(
      "CREATE TABLE IF NOT EXISTS quota (id INTEGER PRIMARY KEY, day TEXT NOT NULL, used INTEGER NOT NULL)",
    );
  }

  /** 消耗一次配额；返回 false 表示当日额度已用尽。跨日自动归零。 */
  consume(day: string): boolean {
    const row = this.ctx.storage.sql
      .exec<{ day: string; used: number }>(
        "SELECT day, used FROM quota WHERE id = 1",
      )
      .toArray()[0];

    const used = row?.day === day ? row.used : 0;
    if (used >= DAILY_LIMIT) {
      return false;
    }

    this.ctx.storage.sql.exec(
      "INSERT OR REPLACE INTO quota (id, day, used) VALUES (1, ?, ?)",
      day,
      used + 1,
    );
    return true;
  }
}
