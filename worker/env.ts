// Worker 绑定与 Secret，逐项对应 wrangler.jsonc。
// 密钥只存在于 Worker Secret（`wrangler secret put`），不进仓库、不进响应、不进日志（ADR-0008）。

export interface Env {
  ASSETS: Fetcher;
  PROXYCHECK_API_KEY: string;
  TURNSTILE_SECRET_KEY: string;
}
