// Worker 绑定与 Secret，逐项对应 wrangler.jsonc。
// 密钥只存在于 Worker Secret（`wrangler secret put`），不进仓库、不进响应、不进日志（ADR-0008）。

import type { QuotaCounter } from "./quota";

interface Bindings {
  ASSETS: Fetcher;
  /** proxycheck 日配额守卫，单实例 DO（ADR-0002）。 */
  QUOTA: DurableObjectNamespace<QuotaCounter>;
  /** 单 IP 限流，只作用于 /api/risk。 */
  RISK_RATE_LIMITER: RateLimit;
  PROXYCHECK_API_KEY: string;
  TURNSTILE_SECRET_KEY: string;
}

export type Env = Bindings;

// 让 `cloudflare:test` 里的 env（类型是 Cloudflare.Env）拿到本项目的绑定类型。
declare global {
  namespace Cloudflare {
    interface Env extends Bindings {}
  }
}
