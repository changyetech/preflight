// 落地内容渲染断言（--content 计划步骤 1-3 / 5）：
// 三段内容与对照表要真的渲染出来，且切到简体中文时用的是中文文案而不是默认的英文源。

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Landing } from "../src/components/Landing";
import { CopyProvider } from "../src/i18n";
import { COPY, COPY_ZH_HANS } from "../src/copy";
import type { Lang } from "../src/copy";

function renderLanding(lang: Lang): string {
  return renderToStaticMarkup(
    createElement(CopyProvider, { lang }, createElement(Landing)),
  );
}

/** 安装命令里的 `<owner>` 占位符在 HTML 里是转义态，直接拿原串断言会假红。 */
function esc(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

describe("落地内容三段", () => {
  it("中文版渲染出「为什么需要」「安装 CLI」「对照表」三段文案", () => {
    const html = renderLanding("zh-hans");

    expect(html).toContain(COPY_ZH_HANS.landing.why.title);
    expect(html).toContain(COPY_ZH_HANS.landing.install.title);
    expect(html).toContain(COPY_ZH_HANS.landing.compare.title);
    expect(html).toContain(esc(COPY_ZH_HANS.actions.installCommand));
  });

  it("对照表渲染出全部 10 个检测项标题", () => {
    const html = renderLanding("zh-hans");

    for (const id of [
      "O1",
      "O2",
      "O3",
      "O4",
      "O5",
      "O6",
      "C1",
      "C2",
      "C3",
      "C4",
    ] as const) {
      expect(html).toContain(COPY_ZH_HANS.checks[id].title);
    }
  });

  // O5 上线后本站确实能在线检测 DNS 出口泄露（契约 §2.5），「不得声称能检测 DNS 泄露」这条
  // 旧断言的方向已经反了——它会拦下对 O5 的如实描述。仍然成立的边界是另一件事：
  // C2（本地 DNS 服务器配置）网页结构性读不到，正文得把这个边界安在 C2 的措辞上，
  // 而不是安在「DNS 泄露」这个已经被 O5 接管的说法上。
  //
  // （原先还有一条 `not.toMatch(/DNS 泄露.{0,20}仅 CLI/)`：正文现在根本不含「DNS 泄露」
  // 四个字，它对新旧文案都不匹配，是条永远为真的死断言。承重的是下面这句 toContain。）
  it("「为什么需要」段落把「仅 CLI」这个边界安在「本地 DNS 服务器配置」上", () => {
    const html = renderLanding("zh-hans");
    const body = COPY_ZH_HANS.landing.why.body;
    const whyIndex = html.indexOf(body);

    expect(whyIndex).toBeGreaterThanOrEqual(0);
    expect(body).toContain("本地 DNS 服务器配置");
  });

  it("简体中文版渲染的是中文文案，不停留在英文源", () => {
    const html = renderLanding("zh-hans");

    expect(html).toContain(COPY_ZH_HANS.landing.why.title);
    expect(html).not.toContain(COPY.landing.why.title);
  });

  it("英文版（默认语言）渲染英文文案", () => {
    const html = renderLanding("en");

    expect(html).toContain(COPY.landing.why.title);
    expect(html).toContain(COPY.landing.compare.title);
  });

  // 未译语种走字段级回落：ru 目前一条未译，因此整页应与英文源逐字相同（规格第 7 节）。
  it("未译语种回落英文源，而不是渲染出空文案", () => {
    expect(renderLanding("ru")).toBe(renderLanding("en"));
  });

  // C1 修复的渲染层回归：CLI README.md 功能表里 O1-O6 与 C1-C4 全部 10 项 CLI 都测得到，
  // 对照表的 CLI 列因此必须恒为「有」（compare.available），不能出现「—」——
  // compareTable.test.ts 只测数据（owner/execution 字段），测不出「数据对、CLI 列渲染成 —」
  // 这种呈现层缺陷，所以必须在这里对渲染出的 HTML 做断言。
  it("对照表 CLI 列 10 行全部渲染为「有」，不出现「—」", () => {
    const html = renderLanding("zh-hans");
    const tableStart = html.indexOf("compare-table");
    const table = html.slice(tableStart);

    // 每行两个单元格：Web 列可能是「—」（仅 CLI 项），但 CLI 列这一整列不该有任何「—」。
    const cliColumnCells = table.match(/<td>[^<]*<\/td>\s*<\/tr>/g) ?? [];
    expect(cliColumnCells).toHaveLength(10);
    for (const cell of cliColumnCells) {
      expect(cell).toContain(COPY_ZH_HANS.landing.compare.available);
      expect(cell).not.toContain(COPY_ZH_HANS.landing.compare.dash);
    }
  });
});
