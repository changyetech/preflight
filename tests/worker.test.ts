// 路由骨架测试：未知 /api/* 路径 404，非 /api/* 交给 Static Assets。
import { SELF } from "cloudflare:test";
import { describe, expect, it } from "vitest";

describe("worker", () => {
  it("GET /api/unknown 返回 404", async () => {
    const response = await SELF.fetch("https://example.com/api/unknown");

    expect(response.status).toBe(404);
  });
});
