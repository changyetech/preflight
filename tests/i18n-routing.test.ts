// 国际化路由：不做 Accept-Language 自动跳转，语言完全由路径决定（规格第 7 节 / --content 步骤 6）。
//
// /en 现在是 Vite 多入口构建出的一份独立静态资源（见 vite.config.ts），不再靠
// not_found_handling: "single-page-application" 兜底——那种兜底会让任意路径都 200，
// 在评审里被判定为软 404（--content 修复轮 1 I1）。
//
// 注意：这里跑在 `@cloudflare/vitest-pool-workers` 的 miniflare 环境里，`env.ASSETS.fetch`
// 读的是本地 `dist/`（跑测试前需要先 `pnpm build`）。miniflare 对 Static Assets 的
// `not_found_handling` / `html_handling` 模拟是否与线上 Cloudflare Workers 完全一致，
// 未做过线上对照验证——如实标注为待人工核实，不当作已验证过线上行为。
import { SELF } from "cloudflare:test";
import { describe, expect, it } from "vitest";

import { COPY, COPY_EN } from "../src/copy";

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
});

describe("/en 是独立的英文静态资源，不是中文页面的软 404 兜底", () => {
  it('/en 返回 200，且是真正的英文 HTML（lang="en" / 英文 <title>）', async () => {
    const response = await SELF.fetch("https://example.com/en");
    const html = await response.text();

    expect(response.status).toBe(200);
    expect(html).toContain('lang="en"');
    // 绑定 COPY_EN.site.title 而非字面量：copy.ts 改了标题，这条断言要能跟着变红
    // （N2：上一版直接写死英文字符串，锁不住 en/index.html 与 COPY_EN 的一致性）。
    expect(html).toContain(COPY_EN.site.title);
  });

  it("根路径仍是中文 HTML，两份入口互不覆盖", async () => {
    const response = await SELF.fetch("https://example.com/");
    const html = await response.text();

    expect(html).toContain('lang="zh-CN"');
    expect(html).toContain(COPY.site.title);
  });

  it("不存在的路径返回真实 404，而不是软 404（I1：曾经的 SPA 回退会让任意路径都 200）", async () => {
    const response = await SELF.fetch(
      "https://example.com/this-path-does-not-exist",
    );

    expect(response.status).toBe(404);
  });
});
