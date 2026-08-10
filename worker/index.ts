// Worker 入口：处理 /api/*，其余请求交给 Static Assets（见 wrangler.jsonc 的 assets 绑定）
// /api/ping 是临时探针端点，用于验证骨架的路由是否打通，后续任务会替换为真实 API 路由。

export interface Env {
  ASSETS: Fetcher;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === "/api/ping") {
      return Response.json({ ok: true });
    }

    if (url.pathname.startsWith("/api/")) {
      return new Response("Not Found", { status: 404 });
    }

    return env.ASSETS.fetch(request);
  },
} satisfies ExportedHandler<Env>;
