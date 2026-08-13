// 国际化路由：不做 Accept-Language 自动跳转，语言完全由路径决定（规格第 7 节）。
//
// 每个语种都是 Vite 多入口构建出的一份独立静态资源（见 vite.config.ts），不靠
// not_found_handling: "single-page-application" 兜底——那种兜底会让任意路径都 200，
// 在评审里被判定为软 404（--content 修复轮 1 I1）。
//
// 语种终态（规格 §3 决策 2 / docs/adr/0016）：en（`/`，x-default）+ zh-hans（`/zh-hans`）。
// `/en`（旧别名）、`/zh-hant`、`/ru`、`/ar` 一律真 404，不重定向、不做 SPA 回落。
//
// 注意：这里跑在 `@cloudflare/vitest-pool-workers` 的 miniflare 环境里，`env.ASSETS.fetch`
// 读的是本地 `dist/`（跑测试前需要先 `pnpm build`）。miniflare 对 Static Assets 的
// `not_found_handling` / `html_handling` 模拟是否与线上 Cloudflare Workers 完全一致，
// 未做过线上对照验证——如实标注为待人工核实，不当作已验证过线上行为。
import { SELF } from "cloudflare:test";
import { describe, expect, it } from "vitest";

import { COPY, COPY_ZH_HANS } from "../src/copy";

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

describe("两语种各自是独立静态资源，不是彼此的软 404 兜底", () => {
  it("根路径是英文（默认语言，规格第 7 节）", async () => {
    const response = await SELF.fetch("https://example.com/");
    const html = await response.text();

    expect(response.status).toBe(200);
    expect(html).toContain('lang="en"');
    // 绑定 COPY.site.title 而非字面量：文案改了标题，这条断言要能跟着变红
    // （N2：上一版直接写死英文字符串，锁不住 index.html 与文案的一致性）。
    expect(html).toContain(COPY.site.title);
  });

  it("/zh-hans 返回 200，且是真正的中文 HTML，不是英文页的副本", async () => {
    const response = await SELF.fetch("https://example.com/zh-hans");
    const html = await response.text();

    expect(response.status).toBe(200);
    expect(html).toContain('lang="zh-Hans"');
    expect(html).toContain(COPY_ZH_HANS.site.title);
    expect(html).not.toContain(COPY.site.title);
  });

  // 规格 §3 要求两个入口各自声明同一份 hreflang 集合——用 it.each 对 `/` 与
  // `/zh-hans` 各跑一遍，而不是只测其中一个入口（曾经的缺口：只测过 /zh-hans）。
  it.each(["/", "/zh-hans"])(
    "%s 恰好声明 3 条 hreflang：en、zh-Hans、x-default（规格 §3）",
    async (path) => {
      const html = await (
        await SELF.fetch(`https://example.com${path}`)
      ).text();

      // 解析出所有 <link rel="alternate"> 标签本身，再从标签内取 hreflang/href——
      // 不依赖标签在源码里被 prettier 排成一行还是拆成多行（对格式化不敏感）。
      const linkTags = html.match(/<link\s+rel="alternate"[^>]*\/?>/g) ?? [];
      const hreflangToHref = new Map(
        linkTags.map((tag) => [
          tag.match(/hreflang="([^"]*)"/)?.[1],
          tag.match(/href="([^"]*)"/)?.[1],
        ]),
      );

      expect(
        linkTags,
        `实际 alternate 标签：\n${linkTags.join("\n")}`,
      ).toHaveLength(3);
      expect(
        hreflangToHref,
        `实际 hreflang→href 映射：${JSON.stringify([...hreflangToHref])}`,
      ).toEqual(
        new Map([
          ["en", "https://preflight.omnikit.run/"],
          ["zh-Hans", "https://preflight.omnikit.run/zh-hans"],
          ["x-default", "https://preflight.omnikit.run/"],
        ]),
      );
    },
  );

  it("不存在的路径返回真实 404，而不是软 404（I1：曾经的 SPA 回退会让任意路径都 200）", async () => {
    const response = await SELF.fetch(
      "https://example.com/this-path-does-not-exist",
    );

    expect(response.status).toBe(404);
  });

  it.each(["/en", "/zh-hant", "/ru", "/ar"])(
    "%s 已从语种终态删除，返回真实 404，不重定向、不回落（规格 §2 决策 2）",
    async (path) => {
      const response = await SELF.fetch(`https://example.com${path}`);

      expect(response.status).toBe(404);
      expect(response.headers.get("location")).toBeNull();
    },
  );

  it("形似语种但未注册的路径同样是真实 404（如 /zh、/fr）", async () => {
    for (const path of ["/zh", "/fr"]) {
      const response = await SELF.fetch(`https://example.com${path}`);
      expect(response.status).toBe(404);
    }
  });
});
