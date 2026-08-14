// 可达性收尾断言（W6 任务，规格 §4 要点 6/7）。
//
// 这组测试只补 W1–W5 遗漏、且能用 renderToStaticMarkup 断言的两处：
//   1. 覆盖度 meter 的 role="img" + 动态 aria-label——原来是 aria-hidden，读屏用户拿不到
//      这块信息，是本任务发现并修复的缺口。
//   2. 结论区 aria-live/aria-atomic 的播报边界——覆盖度小节必须在播报区之外，
//      否则覆盖度数字跳动会跟着重复播报（过度播报与不播报同样是缺陷）。
// 焦点顺序、Esc 关闭下拉后的焦点归还、焦点环在两个主题下的对比度，需要真实 DOM/键盘交互，
// 已用 Chrome DevTools/Playwright 做人工核查，方法与结论见 report，不在此补 jsdom。

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { VerdictPanel } from "../src/components/Verdict";
import type { CoverageCell } from "../src/coverageMeter";
import { COPY, COPY_ZH_HANS } from "../src/copy";
import type { Coverage } from "../src/domain/coverage";
import type { GeoData } from "../src/domain/types";
import { CopyProvider } from "../src/CopyProvider";
import type { Lang } from "../src/copy";

const GEO: GeoData = {
  ip: "1.2.3.4",
  country: "CN",
  region: "Shanghai",
  city: "Shanghai",
  postalCode: null,
  continent: "AS",
  latitude: "31.2",
  longitude: "121.4",
  timezone: "Asia/Shanghai",
  asn: 4134,
  asOrganization: "Chinanet",
  colo: "SHA",
};

const COVERAGE: Coverage = { done: 3, needCli: 4, failed: 2, pending: 1 };
const CELLS: CoverageCell[] = [
  "done",
  "done",
  "done",
  "failed",
  "failed",
  "ondemand",
  "cli",
  "cli",
  "cli",
  "cli",
];

function renderVerdict(lang: Lang): string {
  return renderToStaticMarkup(
    createElement(
      CopyProvider,
      { lang },
      createElement(VerdictPanel, {
        geo: { status: "done", data: GEO },
        verdict: { stage: "preliminary", level: "low" },
        coverage: COVERAGE,
        cells: CELLS,
      }),
    ),
  );
}

describe("覆盖度 meter 是 role=img 的图，带随状态更新的两语种 aria-label（原型 refs/ipcheck-web-redesign.html:837）", () => {
  it.each([["en", COPY] as const, ["zh-hans", COPY_ZH_HANS] as const])(
    "%s 版 meter 挂 role=img，aria-label 含四档数字",
    (lang, copy) => {
      const html = renderVerdict(lang);

      expect(html).toContain('class="cov-meter" role="img"');
      // aria-label 不写死文案，四个数字与四段 Copy 片段都要出现——
      // 破坏性验证过这条会打红：把 CoverageMeter 的 aria-label 拿掉，或把某个数字改错，
      // 这条断言都会因为 aria-label 属性缺失/内容不含该数字而失败。
      const label = html.match(/aria-label="([^"]*)"/)?.[1];
      expect(label).toBeTruthy();
      expect(label).toContain(copy.coverage.total);
      expect(label).toContain(`${copy.coverage.done} 3`);
      expect(label).toContain(`${copy.coverage.needCli} 4`);
      expect(label).toContain(`${copy.coverage.failed} 2`);
      expect(label).toContain(`${copy.coverage.pending} 1`);
    },
  );

  it("不再是 aria-hidden——W4/W5 之前的写法会让这块信息对读屏用户整段消失", () => {
    const html = renderVerdict("en");

    expect(html).not.toContain('class="cov-meter" aria-hidden="true"');
  });
});

describe("结论区 aria-live 播报边界（规格 §4 要点 6：变化要播报，且不因覆盖度跳动而重复播报）", () => {
  it("aria-live=polite + aria-atomic=true 只包住档位/阶段/摘要，不包住覆盖度小节", () => {
    const html = renderVerdict("en");

    const liveStart = html.indexOf('aria-live="polite"');
    expect(liveStart).toBeGreaterThan(-1);
    expect(html).toContain('aria-atomic="true"');

    // 播报区结束于 </div>，覆盖度小节（class="cov"）必须出现在它之后，
    // 否则覆盖度每完成一项就会跟着结论一起被重复播报一遍。
    const covStart = html.indexOf('class="cov"');
    const liveDivEnd = html.indexOf(
      "</div>",
      html.indexOf("summary", liveStart),
    );
    expect(covStart).toBeGreaterThan(liveDivEnd);
  });
});
