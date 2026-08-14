// 页脚双栏披露渲染断言（规格 §4 要点 5，W4 任务）：
// 隐私声明 + 自动发起/需手动触发两栏的第三方披露都要真的渲染出来，且不得出现设计稿专属的
// demo-only 元素（页脚「本页为设计稿」声明、「示例数据 · 非真实检测」标记）。

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Footer } from "../src/components/Footer";
import { CopyProvider } from "../src/i18n";
import { COPY, COPY_ZH_HANS } from "../src/copy";
import type { Lang } from "../src/copy";

function renderFooter(lang: Lang): string {
  return renderToStaticMarkup(
    createElement(CopyProvider, { lang }, createElement(Footer, { lang })),
  );
}

describe("页脚双栏披露", () => {
  it("中文版渲染隐私声明 + 自动发起/需手动触发两栏披露", () => {
    const html = renderFooter("zh-hans");

    expect(html).toContain(COPY_ZH_HANS.footer.privacy);
    expect(html).toContain(COPY_ZH_HANS.footer.autoLabel);
    expect(html).toContain(COPY_ZH_HANS.footer.autoBody);
    expect(html).toContain(COPY_ZH_HANS.footer.onDemandLabel);
    expect(html).toContain(COPY_ZH_HANS.footer.onDemandBody);
  });

  it("英文版（默认语言）渲染英文披露文案", () => {
    const html = renderFooter("en");

    expect(html).toContain(COPY.footer.privacy);
    expect(html).toContain(COPY.footer.autoBody);
    expect(html).toContain(COPY.footer.onDemandBody);
  });

  // 子页入口（spec docs/specs/2026-08-14-legal-pages.md）：链接必须带当前语种前缀，
  // 中文首页的页脚不能把人送到英文子页。
  it.each([
    ["en", ["/dns/", "/privacy/", "/terms/"]] as const,
    [
      "zh-hans",
      ["/zh-hans/dns/", "/zh-hans/privacy/", "/zh-hans/terms/"],
    ] as const,
  ])("%s 版页脚的三个子页入口带正确语种前缀", (lang, hrefs) => {
    const html = renderFooter(lang);

    for (const href of hrefs) {
      expect(html).toContain(`href="${href}"`);
    }
    // 三个入口都在新标签打开，且带 noopener（版权行的公司外链另计，见下一条）。
    expect(html.match(/target="_blank"/g)).toHaveLength(hrefs.length + 1);
    expect(html.match(/rel="noopener"/g)).toHaveLength(hrefs.length);
  });

  // 版权行：公司名可点，指向官网；外站链接必须带 noreferrer。
  it("版权行的公司名链到官网并在新标签打开", () => {
    const html = renderFooter("en");

    expect(html).toContain("Hangzhou Changye Network Technology Co., Ltd.");
    expect(html).toContain('href="https://changyetech.com"');
    expect(html).toContain('rel="noopener noreferrer"');
  });

  // 硬性红线（brief 要点 1）：原型页脚的「本页为设计稿」声明、「示例数据 · 非真实检测」标记
  // 是设计稿的元信息，不是产品内容，落地就等于把设计稿声明发到生产站。
  it.each([["en", COPY] as const, ["zh-hans", COPY_ZH_HANS] as const])(
    "%s 版页脚不含 demo-only 声明",
    (_id, copy) => {
      const html = renderFooter(copy === COPY ? "en" : "zh-hans");

      expect(html).not.toContain("本页为设计稿");
      expect(html).not.toContain("示例数据");
      expect(html).not.toContain("非真实检测");
      expect(html.toLowerCase()).not.toContain("this page is a mockup");
      expect(html.toLowerCase()).not.toContain("sample data");
    },
  );
});
