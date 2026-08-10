// 落地内容渲染断言（--content 计划步骤 1-3 / 5）：
// 三段内容与对照表要真的渲染出来，且英文版切到英文文案而不是停留在中文默认值。

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Landing } from "../src/components/Landing";
import { CopyProvider } from "../src/i18n";
import { COPY, COPY_EN } from "../src/copy";

function renderLanding(lang: "zh" | "en"): string {
  return renderToStaticMarkup(
    createElement(CopyProvider, { lang }, createElement(Landing)),
  );
}

describe("落地内容三段", () => {
  it("中文版渲染出「为什么需要」「安装 CLI」「对照表」三段文案", () => {
    const html = renderLanding("zh");

    expect(html).toContain(COPY.landing.why.title);
    expect(html).toContain(COPY.landing.install.title);
    expect(html).toContain(COPY.landing.compare.title);
    expect(html).toContain(COPY.actions.installCommand);
  });

  it("对照表渲染出全部 9 个检测项标题", () => {
    const html = renderLanding("zh");

    for (const id of [
      "O1",
      "O2",
      "O3",
      "O4",
      "C1",
      "C2",
      "C3",
      "C4",
      "C5",
    ] as const) {
      expect(html).toContain(COPY.checks[id].title);
    }
  });

  it("「为什么需要」段落不得声称本站能检测 DNS 泄露（规格非目标 / --content 步骤 1）", () => {
    const html = renderLanding("zh");
    const whyIndex = html.indexOf(COPY.landing.why.body);

    expect(whyIndex).toBeGreaterThanOrEqual(0);
    // C2（DNS 泄露）的标题只应出现在对照表行里，不应出现在「为什么需要」正文中声称可测。
    expect(COPY.landing.why.body).not.toContain("检测 DNS 泄露");
  });

  it("英文版渲染的是英文文案，不回落到中文默认值", () => {
    const html = renderLanding("en");

    expect(html).toContain(COPY_EN.landing.why.title);
    expect(html).toContain(COPY_EN.landing.compare.title);
    expect(html).not.toContain(COPY.landing.why.title);
  });
});
