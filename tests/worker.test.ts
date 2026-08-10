// 路由骨架测试：未知 /api/* 路径 404，非 /api/* 交给 Static Assets。
import { SELF } from "cloudflare:test";
import { describe, expect, it } from "vitest";

describe("worker", () => {
  it("GET /api/unknown 返回 404 / 4001，且走统一信封", async () => {
    const response = await SELF.fetch("https://example.com/api/unknown");
    const body = (await response.json()) as { code: number; message: string };

    expect(response.status).toBe(404);
    expect(response.headers.get("content-type")).toBe(
      "application/json; charset=utf-8",
    );
    expect(body.code).toBe(4001);
    expect(body.message).toBe("resource not found");
  });
});
