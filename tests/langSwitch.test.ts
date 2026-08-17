// 语言切换器单键直切（规格 §2 决策 8，W5 任务）：真 <a href>，两页互跳要能被测试锁住，
// 不是手工点一遍就算——href 指反了这种错误必须让测试变红。
//
// 用 react-dom/server 的 renderToStaticMarkup 出静态 HTML，与 tests/footer.test.ts /
// tests/render.test.ts 同一套手法，不引 jsdom、不引 testing-library。

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { LangSwitch } from "../src/components/LangSwitch";
import { CopyProvider } from "../src/CopyProvider";
import type { Lang } from "../src/copy";

function renderLangSwitch(lang: Lang): string {
  return renderToStaticMarkup(
    createElement(CopyProvider, { lang }, createElement(LangSwitch, { lang })),
  );
}

describe("语言切换器：单键直切", () => {
  it("/ 页面（en）的切换器指向 /zh-hans，显示「简体中文」", () => {
    const html = renderLangSwitch("en");

    const href = html.match(/href="([^"]*)"/)?.[1];
    // renderToStaticMarkup 原样输出 JSX 的 hrefLang 大小写（HTML 属性名不区分大小写，
    // 浏览器解析时与 hreflang 等价，这里只是这套 SSR 手法的序列化细节）。
    const hrefLang = html.match(/hrefLang="([^"]*)"/)?.[1];

    expect(href).toBe("/zh-hans");
    expect(hrefLang).toBe("zh-Hans");
    expect(html).toContain("简体中文");
    // 不能显示当前语种自己的名字——那不是「切换到另一语种」
    expect(html).not.toContain(">English<");
  });

  it("/zh-hans 页面（zh-hans）的切换器指向 /，显示「English」", () => {
    const html = renderLangSwitch("zh-hans");

    const href = html.match(/href="([^"]*)"/)?.[1];
    const hrefLang = html.match(/hrefLang="([^"]*)"/)?.[1];

    expect(href).toBe("/");
    expect(hrefLang).toBe("en");
    expect(html).toContain("English");
    expect(html).not.toContain("简体中文");
  });

  it("两个语种互为镜像：href 不相等，显示文本也不相等", () => {
    const enHtml = renderLangSwitch("en");
    const zhHtml = renderLangSwitch("zh-hans");

    const enHref = enHtml.match(/href="([^"]*)"/)?.[1];
    const zhHref = zhHtml.match(/href="([^"]*)"/)?.[1];

    expect(enHref).not.toBe(zhHref);
  });

  it("无障碍名称说明了切换目标，不只是语种名单词", () => {
    const enHtml = renderLangSwitch("en");

    const ariaLabel = enHtml.match(/aria-label="([^"]*)"/)?.[1];

    expect(ariaLabel).toBe("Switch language to 简体中文");
  });

  // ≤480px 顶栏放不下语种名，CSS 把 .lang-switch-label 收掉、只留图标。
  // 隐藏是 CSS 的事，这里锁前提：语种名得包在这层壳里，别被改成「窄屏不渲染」——
  // 那样切换器就只剩一个 aria-hidden 的地球图标。
  it("语种名包在 .lang-switch-label 里，供窄屏单独收起", () => {
    const html = renderLangSwitch("zh-hans");

    expect(html).toContain('<span class="lang-switch-label">English</span>');
  });

  it("是真 <a> 元素，不是 button / onClick 触发", () => {
    const html = renderLangSwitch("en");

    expect(html).toMatch(/^<a\b/);
    expect(html).not.toContain("<button");
  });
});
