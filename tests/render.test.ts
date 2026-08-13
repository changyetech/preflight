// 呈现层回归断言。
//
// `copy.test.ts` 只能证明「文案还在模块里」，证明不了「文案还渲染在卡片上」。
// 下面每一条守的都是一个 ADR 级硬约束，而它们在组件里都只是一个三元表达式——
// 删掉分支后字符串仍在 copy.ts 里，`copy.test.ts` 照样全绿。这组断言堵的是这个缝。
//
// 用 react-dom/server 的 renderToStaticMarkup 出静态 HTML，不引 jsdom、不引 testing-library：
// 断言的是「有没有渲染出来」，不需要一个真实 DOM。测试文件用 createElement 而非 JSX，
// 是为了留在现有的 `tests/**/*.test.ts` + workers pool 配置里，不动测试基建。

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { CheckCard, CliCard } from "../src/components/Card";
import { O3Card, O4Card, O5Card, O6Card } from "../src/components/cards";
import { VerdictPanel } from "../src/components/Verdict";
import type { CoverageCell } from "../src/coverageMeter";
import { COPY } from "../src/copy";
import {
  UDP_EGRESS_STUN_UNANSWERED,
  UDP_EGRESS_WEBRTC_UNAVAILABLE,
} from "../src/domain/udpEgress";
import { DNS_EGRESS_PROBE_FAILED } from "../src/domain/dnsEgress";
import type { Coverage } from "../src/domain/coverage";
import type { GeoData } from "../src/domain/types";
import type { Verdict } from "../src/domain/verdict";

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

/**
 * renderToStaticMarkup 会把 `'` 与 `"` 转义成实体，而英文源文案里两者都很常见。
 * 断言前对期望值做同样转义——否则 not.toContain 会因为「转义后本就匹配不上」永远通过，
 * 变成一条假绿的断言（多语化把默认语言从中文换成英文后才暴露出来）。
 */
function esc(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/'/g, "&#x27;")
    .replace(/"/g, "&quot;");
}

const COVERAGE: Coverage = { done: 3, needCli: 4, failed: 0, pending: 1 };
/** 与 COVERAGE 对应的 10 格状态：3 已完成 + 4 需 CLI + 0 失败 + 1 按需未测 + 2 进行中。 */
const CELLS: CoverageCell[] = [
  "done",
  "done",
  "done",
  "ondemand",
  "running",
  "running",
  "cli",
  "cli",
  "cli",
  "cli",
];

function renderVerdict(
  verdict: Verdict,
  coverage: Coverage = COVERAGE,
  cells: CoverageCell[] = CELLS,
): string {
  return renderToStaticMarkup(
    createElement(VerdictPanel, {
      geo: { status: "done", data: GEO },
      verdict,
      coverage,
      cells,
    }),
  );
}

describe("结论区渲染", () => {
  it("初步结论必须渲染出「初步 · 未含 IP 风险评分」标注（验收标准 4 / ADR-0005）", () => {
    const html = renderVerdict({ stage: "preliminary", level: "low" });

    expect(html).toContain(esc(COPY.verdict.preliminaryBadge));
  });

  it("完整结论渲染的是完整标注，不是初步标注", () => {
    const html = renderVerdict({ stage: "full", level: "high" });

    expect(html).toContain(esc(COPY.verdict.fullBadge));
    expect(html).not.toContain(esc(COPY.verdict.preliminaryBadge));
  });

  it("覆盖度四档与分母都必须渲染在结论旁（ADR-0004）", () => {
    const html = renderVerdict({ stage: "preliminary", level: "low" });

    expect(html).toContain(esc(COPY.coverage.done));
    expect(html).toContain("<b>3</b>");
    expect(html).toContain(esc(COPY.coverage.needCli));
    expect(html).toContain("<b>4</b>");
    expect(html).toContain(esc(COPY.coverage.failed));
    expect(html).toContain("<b>0</b>");
    expect(html).toContain(esc(COPY.coverage.pending));
    expect(html).toContain("<b>1</b>");
    expect(html).toContain(esc(COPY.coverage.total));
    // 10 格 meter 恒渲染 10 个格子（规格 §4 要点 5）。
    expect(html.match(/class="cov-cell /g) ?? []).toHaveLength(10);
  });

  it("数据不足时不得出现「低风险」字样与低风险配色（C-1）", () => {
    const html = renderVerdict({ stage: "insufficient" });

    expect(html).toContain(esc(COPY.verdict.insufficientLabel));
    expect(html).toContain(esc(COPY.verdict.summary.insufficient));
    expect(html).not.toContain(esc(COPY.verdict.level.low));
    expect(html).not.toContain(esc(COPY.verdict.summary.preliminaryLow));
    // 配色类名也不许落到低风险绿——用户读色块的速度快过读字。
    // v-low 是唯一挂着绿色 CSS 规则的类名（.v-low .level），数据不足时它不该出现。
    expect(html).not.toContain("v-low");
    // 还没有结论，就不该挂「初步 / 完整」这种描述结论成色的标注。
    expect(html).not.toContain(esc(COPY.verdict.preliminaryBadge));
    expect(html).not.toContain(esc(COPY.verdict.fullBadge));
  });

  it("数据不足时覆盖度照常呈现（ADR-0004 不因无结论而豁免）", () => {
    const html = renderVerdict(
      { stage: "insufficient" },
      { done: 0, needCli: 5, failed: 3, pending: 1 },
      [
        "failed",
        "failed",
        "failed",
        "ondemand",
        "running",
        "cli",
        "cli",
        "cli",
        "cli",
        "cli",
      ],
    );

    expect(html).toContain(esc(COPY.coverage.failed));
    expect(html).toContain("<b>3</b>");
    expect(html).toContain(esc(COPY.coverage.done));
    expect(html).toContain("<b>0</b>");
  });

  it("控制台恒挂 v-none/v-low/v-medium/v-high 之一，永远不脱离覆盖度单独出现", () => {
    for (const verdict of [
      { stage: "insufficient" } as const,
      { stage: "preliminary", level: "low" } as const,
      { stage: "preliminary", level: "medium" } as const,
      { stage: "full", level: "high" } as const,
    ]) {
      const html = renderVerdict(verdict);
      // section 与 cov（覆盖度小节）必须同时出现在同一份 HTML 里。
      expect(html).toContain('class="console v-');
      expect(html).toContain('class="cov"');
    }
  });

  it("控制台在有在线项检测中时挂 aria-busy=true，检测都结束后挂 false（规格 §4 要点 7）", () => {
    const runningCells: CoverageCell[] = [
      "running",
      "done",
      "done",
      "ondemand",
      "done",
      "done",
      "cli",
      "cli",
      "cli",
      "cli",
    ];
    const idleCells: CoverageCell[] = [
      "done",
      "done",
      "done",
      "ondemand",
      "done",
      "done",
      "cli",
      "cli",
      "cli",
      "cli",
    ];
    const running = renderVerdict(
      { stage: "insufficient" },
      COVERAGE,
      runningCells,
    );
    const idle = renderVerdict(
      { stage: "preliminary", level: "low" },
      COVERAGE,
      idleCells,
    );

    expect(running).toContain('aria-busy="true"');
    expect(idle).not.toContain('aria-busy="true"');
    expect(idle).toContain('aria-busy="false"');
  });
});

describe("卡片重试入口（规格 4.1）", () => {
  // 安装命令全站只在落地内容「安装 CLI」段出现一次（规格第 4 节第 2 项），
  // 灰卡内不再重复，因此这里断言灰卡内不含它。
  it("灰卡是终态，没有重试入口，也不重复安装命令", () => {
    const html = renderToStaticMarkup(
      createElement(CliCard, {
        id: "C1",
        title: COPY.checks.C1.title,
        meaning: COPY.checks.C1.meaning,
      }),
    );

    expect(html).not.toContain('class="retry"');
    expect(html).not.toContain(esc(COPY.actions.installCommand));
    expect(html).toContain(esc(COPY.cli.hint));
    expect(html).toContain(esc(COPY.cardStatus.needCli));
  });

  it("失败卡有重试入口", () => {
    const html = renderToStaticMarkup(
      createElement(CheckCard, {
        id: "O3",
        title: COPY.checks.O3.title,
        status: "failed",
        meaning: COPY.checks.O3.meaning,
        onRetry: () => {},
      }),
    );

    expect(html).toContain('class="retry"');
    expect(html).toContain(esc(COPY.actions.retry));
  });
});

describe("「这意味着什么」折叠（规格 4.2）", () => {
  // 折叠只应改变默认可见性，不得把解释文案从 DOM 里拿掉——
  // 拿掉就等于删了规格 4.2 要求的每卡解释，且爬虫也读不到。
  it("渲染为 details/summary，解释文案仍在 HTML 内", () => {
    const html = renderToStaticMarkup(
      createElement(CliCard, {
        id: "C1",
        title: COPY.checks.C1.title,
        meaning: COPY.checks.C1.meaning,
      }),
    );

    expect(html).toContain('<details class="meaning"');
    expect(html).toContain(`<summary class="meaning-label">`);
    expect(html).toContain(esc(COPY.checks.C1.meaning));
  });

  // ADR-0008：第三方披露挂在触发它的控件旁，必须默认可见，不能跟着一起折叠。
  it("O3 的 ipify 披露不在折叠块内", () => {
    const html = renderToStaticMarkup(
      createElement(O3Card, {
        state: { status: "done", data: { leak: false, ipv6: null } },
        onRetry: () => {},
      }),
    );

    const disclosure = html.indexOf(esc(COPY.checks.O3.thirdPartyNote));
    expect(disclosure).toBeGreaterThan(-1);
    expect(disclosure).toBeLessThan(html.indexOf("<details"));
  });
});

describe("O4 第三方披露（ADR-0008）", () => {
  it("未触发时按钮上写明 proxycheck.io", () => {
    const html = renderToStaticMarkup(
      createElement(O4Card, {
        state: { status: "idle" },
        onRun: () => {},
        onFail: () => {},
      }),
    );

    expect(html).toContain(esc(COPY.checks.O4.consentButton));
    expect(html).toContain("proxycheck.io");
    expect(html).toContain(esc(COPY.checks.O4.consentNote));
  });

  it("失败后的重试入口同样写明 proxycheck.io——重试就是那个触发控件", () => {
    const html = renderToStaticMarkup(
      createElement(O4Card, {
        state: { status: "failed", reason: COPY.errors.upstream },
        onRun: () => {},
        onFail: () => {},
      }),
    );

    expect(html).toContain('class="retry"');
    expect(html).toContain(esc(COPY.checks.O4.consentButton));
    expect(html).toContain("proxycheck.io");
    expect(html).toContain(esc(COPY.checks.O4.consentNote));
    // 光写「重试」二字就等于把披露藏了起来。
    expect(html).not.toContain(`class="retry">${COPY.actions.retry}<`);
  });

  it("配额耗尽呈现为检测失败，且不给重试入口——今天重试多少次都一样", () => {
    const html = renderToStaticMarkup(
      createElement(O4Card, {
        state: { status: "done", data: { status: "quotaExhausted" } },
        onRun: () => {},
        onFail: () => {},
      }),
    );

    expect(html).toContain(esc(COPY.cardStatus.failed));
    expect(html).toContain(esc(COPY.checks.O4.quotaExhausted));
    expect(html).not.toContain('class="retry"');
  });
});

describe("O3 第三方披露（终审修复波：ipify 无就地披露）", () => {
  it("O3 自动执行、无触发控件——披露文案必须始终渲染在卡片说明位", () => {
    const html = renderToStaticMarkup(
      createElement(O3Card, {
        state: { status: "running" },
        onRetry: () => {},
      }),
    );

    expect(html).toContain(esc(COPY.checks.O3.thirdPartyNote));
    expect(html).toContain("ipify");
  });

  it("失败后的重试按钮写明再次直连 ipify，不落回通用「重试」（与 O4 一致执行 ADR-0008）", () => {
    const html = renderToStaticMarkup(
      createElement(O3Card, {
        state: { status: "failed", reason: COPY.checks.O3.failed },
        onRetry: () => {},
      }),
    );

    expect(html).toContain('class="retry"');
    expect(html).toContain(esc(COPY.checks.O3.retryLabel));
    // 破坏性验证过这条会打红：把 O3Card 的 retryLabel 去掉后，这行会因为
    // 按钮落回通用「重试」而失败（见 --content 计划终审修复波报告）。
    expect(html).not.toContain(`class="retry">${COPY.actions.retry}<`);
  });
});

describe("O5 DNS 出口泄露卡（契约 §2.5／§5.5）", () => {
  it("ECS 缺失时状态是「已完成」，且给出明确说明，不落到「未泄露」", () => {
    const html = renderToStaticMarkup(
      createElement(O5Card, {
        state: {
          status: "done",
          data: {
            resolverGeo: null,
            comparison: { comparable: false, reason: "noEcs" },
          },
        },
        onRetry: () => {},
      }),
    );

    expect(html).toContain(esc(COPY.cardStatus.done));
    expect(html).not.toContain(esc(COPY.cardStatus.failed));
    expect(html).toContain(esc(COPY.checks.O5.noEcs));
    expect(html).not.toContain(esc(COPY.checks.O5.leak));
    expect(html).not.toContain(esc(COPY.checks.O5.noLeak));
  });

  it("三种「无从比对」渲染出各自的说明，互不混用", () => {
    const reasonOf = (
      reason: "noEcs" | "unmappedCountry" | "unknownExitCountry",
    ) =>
      renderToStaticMarkup(
        createElement(O5Card, {
          state: {
            status: "done",
            data: {
              resolverGeo: "Japan - Google LLC",
              comparison: { comparable: false, reason },
            },
          },
          onRetry: () => {},
        }),
      );

    expect(reasonOf("noEcs")).toContain(esc(COPY.checks.O5.noEcs));
    expect(reasonOf("unmappedCountry")).toContain(
      esc(COPY.checks.O5.unmappedCountry),
    );
    expect(reasonOf("unknownExitCountry")).toContain(
      esc(COPY.checks.O5.unknownExitCountry),
    );
  });

  it("展示 resolver 归属，且卡内注明它不参与判定", () => {
    const html = renderToStaticMarkup(
      createElement(O5Card, {
        state: {
          status: "done",
          data: {
            resolverGeo: "Japan - Google LLC",
            comparison: {
              comparable: true,
              leak: false,
              ecsCountry: "CN",
              exitCountry: "CN",
            },
          },
        },
        onRetry: () => {},
      }),
    );

    expect(html).toContain("Japan - Google LLC");
    expect(html).toContain(esc(COPY.checks.O5.resolverNote));
  });

  it("卡内写明本项测的是浏览器 DNS（契约 §5.5，缺了这句 CLI 用户会误以为命令行已被检查）", () => {
    const html = renderToStaticMarkup(
      createElement(O5Card, {
        state: { status: "running" },
        onRetry: () => {},
      }),
    );

    expect(html).toContain(esc(COPY.checks.O5.scopeNote));
  });

  it("探测失败给出重试入口，不渲染成泄露或未泄露的结论", () => {
    const html = renderToStaticMarkup(
      createElement(O5Card, {
        state: { status: "failed", reason: DNS_EGRESS_PROBE_FAILED },
        onRetry: () => {},
      }),
    );

    expect(html).toContain('class="retry"');
    expect(html).toContain(esc(COPY.checks.O5.failed));
    expect(html).not.toContain(esc(COPY.checks.O5.leak));
    expect(html).not.toContain(esc(COPY.checks.O5.noLeak));
  });
});

describe("O6 UDP 出口一致性卡（契约 §2.6／§5.6）", () => {
  it("WebRTC 被禁用 ⇒ 检测失败，且不渲染为绿色（tone-ok 不出现）", () => {
    const html = renderToStaticMarkup(
      createElement(O6Card, {
        state: { status: "failed", reason: UDP_EGRESS_WEBRTC_UNAVAILABLE },
        onRetry: () => {},
      }),
    );

    expect(html).toContain(esc(COPY.cardStatus.failed));
    expect(html).toContain(esc(COPY.checks.O6.webrtcUnavailable));
    expect(html).not.toContain("tone-ok");
    expect(html).not.toContain(esc(COPY.checks.O6.stunUnanswered));
  });

  it("STUN 未应答与 WebRTC 被禁用文案不同（可恢复性相反，契约 §5.6）", () => {
    const html = renderToStaticMarkup(
      createElement(O6Card, {
        state: { status: "failed", reason: UDP_EGRESS_STUN_UNANSWERED },
        onRetry: () => {},
      }),
    );

    expect(html).toContain(esc(COPY.checks.O6.stunUnanswered));
    expect(html).not.toContain(esc(COPY.checks.O6.webrtcUnavailable));
  });

  it("「无从比对」（stunDisagree）与「未命中」文案可区分，且不落到绿色", () => {
    const disagree = renderToStaticMarkup(
      createElement(O6Card, {
        state: {
          status: "done",
          data: { comparable: false, reason: "stunDisagree" },
        },
        onRetry: () => {},
      }),
    );
    const noMismatch = renderToStaticMarkup(
      createElement(O6Card, {
        state: {
          status: "done",
          data: {
            comparable: true,
            mismatch: false,
            reflexiveIp: "203.0.113.7",
            exitIp: "203.0.113.7",
          },
        },
        onRetry: () => {},
      }),
    );

    expect(disagree).toContain(esc(COPY.checks.O6.stunDisagree));
    expect(disagree).not.toContain(esc(COPY.checks.O6.noMismatch));
    expect(disagree).not.toContain("tone-ok");
    expect(noMismatch).toContain(esc(COPY.checks.O6.noMismatch));
  });

  it("命中（mismatch）时展示反射地址与出口 IP 两个字段", () => {
    const html = renderToStaticMarkup(
      createElement(O6Card, {
        state: {
          status: "done",
          data: {
            comparable: true,
            mismatch: true,
            reflexiveIp: "203.0.113.7",
            exitIp: "198.51.100.20",
          },
        },
        onRetry: () => {},
      }),
    );

    expect(html).toContain(esc(COPY.checks.O6.mismatch));
    expect(html).toContain("203.0.113.7");
    expect(html).toContain("198.51.100.20");
  });
});

describe("O5／O6 第三方披露（ADR-0008）", () => {
  it("自动执行、无触发控件——披露文案必须始终渲染在卡片说明位", () => {
    const o5 = renderToStaticMarkup(
      createElement(O5Card, {
        state: { status: "running" },
        onRetry: () => {},
      }),
    );
    const o6 = renderToStaticMarkup(
      createElement(O6Card, {
        state: { status: "running" },
        onRetry: () => {},
      }),
    );

    expect(o5).toContain(esc(COPY.checks.O5.thirdPartyNote));
    expect(o5).toContain("ip-api.com");
    expect(o6).toContain(esc(COPY.checks.O6.thirdPartyNote));
    expect(o6).toContain("stun.cloudflare.com");
    expect(o6).toContain("stun.l.google.com");
  });
});
