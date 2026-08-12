// 国际化路由：不做 Accept-Language 自动跳转，语言完全由路径决定（规格第 7 节）。
//
// 每个语种都是 Vite 多入口构建出的一份独立静态资源（见 vite.config.ts），不靠
// not_found_handling: "single-page-application" 兜底——那种兜底会让任意路径都 200，
// 在评审里被判定为软 404（--content 修复轮 1 I1）。
//
// 注意：这里跑在 `@cloudflare/vitest-pool-workers` 的 miniflare 环境里，`env.ASSETS.fetch`
// 读的是本地 `dist/`（跑测试前需要先 `pnpm build`）。miniflare 对 Static Assets 的
// `not_found_handling` / `html_handling` 模拟是否与线上 Cloudflare Workers 完全一致，
// 未做过线上对照验证——如实标注为待人工核实，不当作已验证过线上行为。
import { SELF } from "cloudflare:test";
import { describe, expect, it } from "vitest";

import { COPY, COPY_ZH_HANS, LOCALES } from "../src/copy";

describe("Accept-Language 不触发自动跳转", () => {
  it("Accept-Language: zh 请求根路径，仍原样返回英文（非重定向），与不带该头时一致", async () => {
    const withHeader = await SELF.fetch("https://example.com/", {
      headers: { "Accept-Language": "zh-CN" },
    });
    const withoutHeader = await SELF.fetch("https://example.com/");

    expect(withHeader.status).toBe(200);
    expect(withHeader.redirected).toBe(false);
    expect(withHeader.headers.get("location")).toBeNull();
    expect(await withHeader.text()).toBe(await withoutHeader.text());
  });
});

describe("每个语种都是独立静态资源，不是彼此的软 404 兜底", () => {
  it("根路径是英文（默认语言，规格第 7 节）", async () => {
    const response = await SELF.fetch("https://example.com/");
    const html = await response.text();

    expect(response.status).toBe(200);
    expect(html).toContain('lang="en"');
    // 绑定 COPY.site.title 而非字面量：文案改了标题，这条断言要能跟着变红
    // （N2：上一版直接写死英文字符串，锁不住 index.html 与文案的一致性）。
    expect(html).toContain(COPY.site.title);
  });

  it.each(LOCALES.map((locale) => [locale.path, locale.htmlLang] as const))(
    "%s 返回 200，且 <html lang> 是 %s",
    async (path, htmlLang) => {
      const response = await SELF.fetch(`https://example.com${path}`);
      const html = await response.text();

      expect(response.status).toBe(200);
      expect(html).toContain(`lang="${htmlLang}"`);
    },
  );

  it("/zh-hans 是真正的中文 HTML，不是英文页的副本", async () => {
    const html = await (await SELF.fetch("https://example.com/zh-hans")).text();

    expect(html).toContain(COPY_ZH_HANS.site.title);
    expect(html).not.toContain(COPY.site.title);
  });

  it("/ar 带 dir=rtl，RTL 布局不依赖译文进度（规格第 7 节）", async () => {
    const html = await (await SELF.fetch("https://example.com/ar")).text();

    expect(html).toContain('dir="rtl"');
  });

  it("/en 是根路径的别名，canonical 指向根路径而不是自成一份索引", async () => {
    const response = await SELF.fetch("https://example.com/en");
    const html = await response.text();

    expect(response.status).toBe(200);
    expect(html).toContain('lang="en"');
    expect(html).toContain(
      'rel="canonical" href="https://ipcheck.omnikit.run/"',
    );
  });

  it("每个语种页都声明全部 hreflang 备选与 x-default（多语 SEO）", async () => {
    const html = await (await SELF.fetch("https://example.com/zh-hans")).text();

    for (const locale of LOCALES) {
      expect(html).toContain(`hreflang="${locale.htmlLang}"`);
    }
    expect(html).toContain('hreflang="x-default"');
  });

  it("不存在的路径返回真实 404，而不是软 404（I1：曾经的 SPA 回退会让任意路径都 200）", async () => {
    const response = await SELF.fetch(
      "https://example.com/this-path-does-not-exist",
    );

    expect(response.status).toBe(404);
  });

  it("形似语种但未注册的路径同样是真实 404（如 /zh、/fr）", async () => {
    for (const path of ["/zh", "/fr"]) {
      const response = await SELF.fetch(`https://example.com${path}`);
      expect(response.status).toBe(404);
    }
  });
});
