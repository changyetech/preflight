// Worker 入口：处理 /api/*，其余请求交给 Static Assets（见 wrangler.jsonc 的 assets 绑定）
// 接口契约见 docs/api.md。

import type { Env } from "./env";
import { handleGeo } from "./geo";
import { ERROR, fail } from "./response";
import { handleRisk } from "./risk";

// Durable Object 类必须从入口再导出，Workers 运行时才能找到它。
export { QuotaCounter } from "./quota";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === "/api/geo" && request.method === "GET") {
      return handleGeo(request);
    }

    if (url.pathname === "/api/risk" && request.method === "POST") {
      return handleRisk(request, env);
    }

    // /api/* 下的一切都走统一信封，路由未命中也不例外（docs/api.md 3.3）。
    if (url.pathname.startsWith("/api/")) {
      return fail(ERROR.NOT_FOUND, "no such endpoint");
    }

    return env.ASSETS.fetch(request);
  },
} satisfies ExportedHandler<Env>;
