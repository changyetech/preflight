// 骨架验证测试：/api/ping 通过 + 未知 /api/* 路径 404。
// 后续任务会替换 /api/ping 为真实路由测试。
import { SELF } from "cloudflare:test";
import { describe, expect, it } from "vitest";

describe("worker", () => {
  it("GET /api/ping 返回 ok", async () => {
    const response = await SELF.fetch("https://example.com/api/ping");

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ ok: true });
  });

  it("GET /api/unknown 返回 404", async () => {
    const response = await SELF.fetch("https://example.com/api/unknown");

    expect(response.status).toBe(404);
  });
});
