// 国际化路由：不做 Accept-Language 自动跳转，语言完全由路径决定（规格第 7 节 / --content 步骤 6）。
import { SELF } from "cloudflare:test";
import { describe, expect, it } from "vitest";

describe("Accept-Language 不触发自动跳转", () => {
  it("Accept-Language: en 请求根路径，仍原样返回（非重定向），与不带该头时一致", async () => {
    const withHeader = await SELF.fetch("https://example.com/", {
      headers: { "Accept-Language": "en" },
    });
    const withoutHeader = await SELF.fetch("https://example.com/");

    expect(withHeader.status).toBe(200);
    expect(withHeader.redirected).toBe(false);
    expect(withHeader.headers.get("location")).toBeNull();
    expect(await withHeader.text()).toBe(await withoutHeader.text());
  });

  it("/en 直接访问也返回 200（SPA 回退到同一份 index.html，语言由前端接管）", async () => {
    const response = await SELF.fetch("https://example.com/en");

    expect(response.status).toBe(200);
  });
});
