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

  // C1 修复的渲染层回归：CLI README.md 功能表里 O1-O4 与 C1-C5 全部 9 项 CLI 都测得到，
  // 对照表的 CLI 列因此必须恒为「有」（compare.available），不能出现「—」——
  // compareTable.test.ts 只测数据（owner/execution 字段），测不出「数据对、CLI 列渲染成 —」
  // 这种呈现层缺陷，所以必须在这里对渲染出的 HTML 做断言。
  it("对照表 CLI 列 9 行全部渲染为「有」，不出现「—」", () => {
    const html = renderLanding("zh");
    const tableStart = html.indexOf("compare-table");
    const table = html.slice(tableStart);

    // 每行两个单元格：Web 列可能是「—」（仅 CLI 项），但 CLI 列这一整列不该有任何「—」。
    const cliColumnCells = table.match(/<td>[^<]*<\/td>\s*<\/tr>/g) ?? [];
    expect(cliColumnCells).toHaveLength(9);
    for (const cell of cliColumnCells) {
      expect(cell).toContain(COPY.landing.compare.available);
      expect(cell).not.toContain(COPY.landing.compare.dash);
    }
  });
});
