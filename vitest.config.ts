import { cloudflareTest } from "@cloudflare/vitest-pool-workers";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
  },
  plugins: [
    cloudflareTest({
      wrangler: { configPath: "./wrangler.jsonc" },
      miniflare: {
        // 测试用占位密钥。真值走 `wrangler secret put`，不入库（ADR-0008）；
        // 所有第三方调用在测试里都被 stub，这两个值不会被真的发出去。
        bindings: {
          PROXYCHECK_API_KEY: "test-proxycheck-key",
          TURNSTILE_SECRET_KEY: "test-turnstile-secret",
        },
      },
    }),
  ],
});
