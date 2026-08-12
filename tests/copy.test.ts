// 文案中有几条是 ADR / 规格的硬性要求，删掉即违约。这些断言的作用是让后续改版删不掉它们。
//
// 分两组：
// - 与语言无关的硬性内容（第三方域名、命令、$TZ 这类标识符）对**每个语种**断言——
//   多语化之后，「某个语种漏译时把 proxycheck.io 的披露弄丢了」是新的失效方式，
//   逐语种跑一遍才锁得住（未译语种走英文回落，同样必须带上这些披露）。
// - 措辞层面的要求（初步结论怎么标、失败不许说成安全、禁用词）按语言分别断言。

import { describe, expect, it } from "vitest";

import { COPY, COPY_ZH_HANS, LOCALES, getCopy } from "../src/copy";

describe.each(
  LOCALES.map((locale) => [locale.id, getCopy(locale.id)] as const),
)("受 ADR 约束的文案 · %s", (_id, copy) => {
  it("O4 触发按钮必须写明第三方调用对象（ADR-0008 / 规格 2.4）", () => {
    expect(copy.checks.O4.consentButton).toContain("proxycheck.io");
  });

  it("O4 说明必须一并披露 StopForumSpam（ADR-0008：新增第三方须在触发控件上标注）", () => {
    expect(copy.checks.O4.consentNote).toContain("StopForumSpam");
  });

  it("O4 说明必须披露 Cloudflare Turnstile（终审修复波：challenges.cloudflare.com 与出口 IP 提交此前全站未提及）", () => {
    expect(copy.checks.O4.consentNote).toContain("Cloudflare Turnstile");
    expect(copy.checks.O4.consentNote).toContain("challenges.cloudflare.com");
  });

  it("O3 自动执行、无触发控件，披露文案必须写明直连 ipify（终审修复波：ipify 无就地披露）", () => {
    expect(copy.checks.O3.thirdPartyNote).toContain("ipify");
    expect(copy.checks.O3.thirdPartyNote).toContain("api.ipify.org");
  });

  it("O3 的重试按钮必须写明再次直连 ipify，不能落回通用「重试」（终审修复波：与 O4 一致执行 ADR-0008）", () => {
    expect(copy.checks.O3.retryLabel).not.toBe(copy.actions.retry);
    expect(copy.checks.O3.retryLabel).toContain("ipify");
  });

  it("页脚第三方披露也要提到 Turnstile", () => {
    expect(copy.footer.thirdParty).toContain("Turnstile");
  });

  // 契约 §5.1 呈现约束：缺了这句，CLI 用户会误以为自己的 $TZ 已被检查。
  it("O2 文案必须说明本项测的是系统时区、并指向 C4（契约 5.1 / 验收标准 2）", () => {
    expect(copy.checks.O2.scopeNote).toContain("$TZ");
    expect(copy.checks.O2.scopeNote).toContain("C4");
  });

  it("覆盖度四档各有独立文案，检测失败与需 CLI 不共用措辞（ADR-0004）", () => {
    const labels = Object.values(copy.coverage);

    expect(copy.coverage.failed).not.toBe(copy.coverage.needCli);
    expect(new Set(labels).size).toBe(labels.length);
  });

  it("滥用收录不可用时的文案不得与「无收录」共用（docs/api.md 3.1）", () => {
    expect(copy.checks.O4.abuse.unknown).not.toBe(copy.checks.O4.abuse.clean);
  });

  // 安装命令必须指向本仓库产出的 Rust CLI，不得回到已归档的 ai-ipcheck（README「安装 CLI」）。
  it("灰卡提示的安装命令与 README 的首选方式一致", () => {
    expect(copy.actions.installCommand).toBe(
      "brew install <owner>/tap/ipcheck",
    );
    expect(copy.actions.installCommand).not.toContain("ai-ipcheck");
  });
});

describe("受 ADR 约束的措辞 · 英文（源语言）", () => {
  it("初步结论标注必须写明未含 IP 风险评分（ADR-0005 / 验收标准 4）", () => {
    expect(COPY.verdict.preliminaryBadge).toContain("Preliminary");
    expect(COPY.verdict.preliminaryBadge).toContain("not included");
  });

  it("O2 文案必须点出图形界面这一侧的时区来源（验收标准 2）", () => {
    expect(COPY.checks.O2.scopeNote).toContain("GUI apps");
    expect(COPY.checks.O2.scopeNote).toContain("Command-line tools");
  });

  it("滥用收录不可用时显示「未知」而非「无收录」（docs/api.md 3.1）", () => {
    expect(COPY.checks.O4.abuse.unknown).toContain("Unknown");
  });

  it("O3 失败文案不得把「测不出来」说成「没有 IPv6」（规格 2.3 / 验收标准 3）", () => {
    expect(COPY.checks.O3.failed).toContain("can't be determined");
    expect(COPY.checks.O3.failed).toContain("does not mean you have no IPv6");
  });

  it("页脚声明零留存（ADR-0008）", () => {
    expect(COPY.footer.privacy).toContain("stores none");
  });
});

describe("受 ADR 约束的措辞 · 简体中文", () => {
  it("O4 触发按钮写明发送出口 IP 至 proxycheck.io（ADR-0008 / 规格 2.4）", () => {
    expect(COPY_ZH_HANS.checks.O4.consentButton).toContain(
      "将把你的出口 IP 发送至 proxycheck.io 查询",
    );
  });

  it("O2 文案必须点出图形界面这一侧的时区来源（验收标准 2）", () => {
    expect(COPY_ZH_HANS.checks.O2.scopeNote).toContain("图形界面应用");
    expect(COPY_ZH_HANS.checks.O2.scopeNote).toContain("命令行工具");
  });

  it("初步结论标注必须写明未含 IP 风险评分（ADR-0005 / 验收标准 4）", () => {
    expect(COPY_ZH_HANS.verdict.preliminaryBadge).toContain("初步");
    expect(COPY_ZH_HANS.verdict.preliminaryBadge).toContain("未含 IP 风险评分");
  });

  it("滥用收录不可用时显示「未知」而非「无收录」（docs/api.md 3.1）", () => {
    expect(COPY_ZH_HANS.checks.O4.abuse.unknown).toContain("未知");
  });

  it("O3 失败文案不得把「测不出来」说成「没有 IPv6」（规格 2.3 / 验收标准 3）", () => {
    expect(COPY_ZH_HANS.checks.O3.failed).toContain("无法判定");
    expect(COPY_ZH_HANS.checks.O3.failed).not.toContain("不存在 IPv6 泄露");
  });

  it("页脚声明零留存（ADR-0008）", () => {
    expect(COPY_ZH_HANS.footer.privacy).toContain("不存储");
  });

  it("不使用「真实 IP」一词描述出口 IP（CONTEXT.md 禁用词）", () => {
    // C1 是仅 CLI 项，「本机真实 IP」是 CLI 的既有术语，允许出现在它自己的标题里。
    const webCopy = JSON.stringify({
      verdict: COPY_ZH_HANS.verdict,
      O1: COPY_ZH_HANS.checks.O1,
      O2: COPY_ZH_HANS.checks.O2,
      O3: COPY_ZH_HANS.checks.O3,
      O4: COPY_ZH_HANS.checks.O4,
    });

    expect(webCopy).not.toContain("真实 IP");
  });
});
