import { resolve } from "node:path";

import { cloudflare } from "@cloudflare/vite-plugin";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
export default defineConfig({
  // cloudflare()：开发期把 worker/index.ts 跑在真实 workerd 里（DO 配额、限流、Turnstile 全是真实现），
  // 与前端共用一个 Vite dev server，因此 HMR 与 /api/* 同时可用，不需要另起 wrangler dev 做反代。
  // 构建期它读 wrangler.jsonc 作为输入配置，产出可直接部署的输出配置（assets.directory 由它填）。
  plugins: [react(), cloudflare()],
  // 多页入口只能挂在 client 环境上：cloudflare() 会额外建一个 worker 构建环境，
  // 写在顶层 build 里会被 worker 环境继承，把两份 HTML 当成 Worker 入口去解析而构建失败。
  environments: {
    client: {
      build: {
        // 多页构建：每个语种各产出一份独立的 index.html（<html lang> / <title> /
        // meta description / canonical 各自正确），而不是共用一份 HTML 壳（--content 计划 I1 / I2 修复）。
        // 语种清单的单一事实来源是 src/copy.ts 的 LOCALES 表；这里的入口要与它逐条对应。
        rollupOptions: {
          input: {
            main: resolve(import.meta.dirname, "index.html"),
            zhHans: resolve(import.meta.dirname, "zh-hans/index.html"),
            dns: resolve(import.meta.dirname, "dns/index.html"),
            zhHansDns: resolve(import.meta.dirname, "zh-hans/dns/index.html"),
            privacy: resolve(import.meta.dirname, "privacy/index.html"),
            zhHansPrivacy: resolve(
              import.meta.dirname,
              "zh-hans/privacy/index.html",
            ),
            terms: resolve(import.meta.dirname, "terms/index.html"),
            zhHansTerms: resolve(
              import.meta.dirname,
              "zh-hans/terms/index.html",
            ),
          },
        },
      },
    },
  },
});
