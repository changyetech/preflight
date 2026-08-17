// 顶栏的跨页入口：DNS 清单 + CLI 手册。
//
// DNS 入口是全站通往 /dns/ 的唯一路径：Web 面板没有 DNS 检测卡（C2 是 CLI-only），
// 页脚的入口已撤（那里只留法务两页），落地内容里也只有说明没有链接。
// 断掉它 = DNS 页在站内不可达，因此单独立测。
// CLI 手册入口（spec docs/specs/2026-08-14-cli-guide-page.md）在站内的另一个入口
// 只有首页安装区块，顶栏这条同样值得锁住。

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { Nav } from "../src/components/Nav";
import { CopyProvider } from "../src/CopyProvider";
import { COPY, COPY_ZH_HANS } from "../src/copy";
import type { Lang } from "../src/copy";

// Nav 里嵌着 ThemeSwitch，它的 useState 初始值在渲染期就要读 window.matchMedia，
// 而 workers pool 里没有 window。只补这一个方法：useEffect 在 SSR 不执行，
// localStorage 那条路径 readStoredThemePref 自己有 try/catch 兜底。
beforeAll(() => {
  (globalThis as { window?: unknown }).window = {
    matchMedia: () => ({ matches: false }),
  };
});

afterAll(() => {
  delete (globalThis as { window?: unknown }).window;
});

function renderNav(lang: Lang, pageSlug?: string): string {
  return renderToStaticMarkup(
    createElement(
      CopyProvider,
      { lang },
      createElement(Nav, { lang, pageSlug }),
    ),
  );
}

// ≤352px 顶栏放不下品牌名，CSS 把 .brand-name 视觉隐藏（clip 而非 display:none）。
// 隐藏手段是 CSS 的事，这里锁的是它的前提：文字必须留在 DOM 里——一旦有人改成
// 「窄屏干脆不渲染品牌名」，这个链接就只剩 aria-hidden 的 svg，读屏读到一个无名链接。
describe("顶栏品牌链接", () => {
  it.each([
    ["en", COPY.nav.brand] as const,
    ["zh-hans", COPY_ZH_HANS.nav.brand] as const,
  ])("%s 版品牌名渲染在 .brand-name 里，始终留在 DOM 中", (lang, brand) => {
    const html = renderNav(lang);

    expect(html).toContain(`<span class="brand-name">${brand}</span>`);
  });
});

describe("顶栏 DNS 清单入口", () => {
  // 中文首页的顶栏不能把人送去英文 DNS 页。
  it.each([
    ["en", "/dns/", COPY.nav.dns] as const,
    ["zh-hans", "/zh-hans/dns/", COPY_ZH_HANS.nav.dns] as const,
  ])("%s 版入口带正确语种前缀与本语种标签", (lang, href, label) => {
    const html = renderNav(lang);

    expect(html).toContain(`href="${href}"`);
    expect(html).toContain(label);
  });

  // 顶栏常驻在检测面板上方，同标签跳走会打断正在进行的检测。
  it("在新标签打开并带 noopener", () => {
    const html = renderNav("en");
    const dnsAnchor = html.match(/<a[^>]*class="nav-dns"[^>]*>/)?.[0];

    expect(dnsAnchor).toContain('target="_blank"');
    expect(dnsAnchor).toContain('rel="noopener"');
  });

  it("只有身处 DNS 页时才标 aria-current", () => {
    expect(renderNav("en", "/dns/")).toContain('aria-current="page"');
    expect(renderNav("en", "/")).not.toContain('aria-current="page"');
  });
});

describe("顶栏 CLI 手册入口", () => {
  it.each([
    ["en", "/guide/", COPY.nav.guide] as const,
    ["zh-hans", "/zh-hans/guide/", COPY_ZH_HANS.nav.guide] as const,
  ])("%s 版入口带正确语种前缀与本语种标签", (lang, href, label) => {
    const html = renderNav(lang);

    expect(html).toContain(`href="${href}"`);
    expect(html).toContain(label);
  });

  it("在新标签打开并带 noopener", () => {
    const html = renderNav("en");
    const guideAnchor = html.match(/<a[^>]*class="nav-guide"[^>]*>/)?.[0];

    expect(guideAnchor).toContain('target="_blank"');
    expect(guideAnchor).toContain('rel="noopener"');
  });

  it("只有身处手册页时才标 aria-current", () => {
    const onGuide = renderNav("en", "/guide/");
    const guideAnchor = onGuide.match(/<a[^>]*class="nav-guide"[^>]*>/)?.[0];

    expect(guideAnchor).toContain('aria-current="page"');
    expect(renderNav("en", "/")).not.toContain('aria-current="page"');
  });
});
