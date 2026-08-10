// 文案中有几条是 ADR / 规格的硬性要求，删掉即违约。这些断言的作用是让后续改版删不掉它们。

import { describe, expect, it } from "vitest";

import { COPY } from "../src/copy";

describe("受 ADR 约束的文案", () => {
  it("O4 触发按钮必须写明第三方调用对象（ADR-0008 / 规格 2.4）", () => {
    expect(COPY.checks.O4.consentButton).toContain(
      "将把你的出口 IP 发送至 proxycheck.io 查询",
    );
  });

  it("O4 说明必须一并披露 StopForumSpam（ADR-0008：新增第三方须在触发控件上标注）", () => {
    expect(COPY.checks.O4.consentNote).toContain("StopForumSpam");
  });

  it("O4 说明必须披露 Cloudflare Turnstile（终审修复波：challenges.cloudflare.com 与出口 IP 提交此前全站未提及）", () => {
    expect(COPY.checks.O4.consentNote).toContain("Cloudflare Turnstile");
    expect(COPY.checks.O4.consentNote).toContain("challenges.cloudflare.com");
  });

  it("O3 自动执行、无触发控件，披露文案必须写明直连 ipify（终审修复波：ipify 无就地披露）", () => {
    expect(COPY.checks.O3.thirdPartyNote).toContain("ipify");
    expect(COPY.checks.O3.thirdPartyNote).toContain("api.ipify.org");
  });

  it("O3 的重试按钮必须写明再次直连 ipify，不能落回通用「重试」（终审修复波：与 O4 一致执行 ADR-0008）", () => {
    expect(COPY.checks.O3.retryLabel).not.toBe(COPY.actions.retry);
    expect(COPY.checks.O3.retryLabel).toContain("ipify");
  });

  it("页脚第三方披露也要提到 Turnstile", () => {
    expect(COPY.footer.thirdParty).toContain("Turnstile");
  });

  it("O2 文案必须区分 Claude 桌面版与 Claude Code CLI 的时区来源（验收标准 2）", () => {
    const note = COPY.checks.O2.scopeNote;

    expect(note).toContain("桌面版");
    expect(note).toContain("Claude Code CLI");
    expect(note).toContain("$TZ");
  });

  it("初步结论标注必须写明未含 IP 风险评分（ADR-0005 / 验收标准 4）", () => {
    expect(COPY.verdict.preliminaryBadge).toContain("初步");
    expect(COPY.verdict.preliminaryBadge).toContain("未含 IP 风险评分");
  });

  it("覆盖度四档各有独立文案，检测失败与需 CLI 不共用措辞（ADR-0004）", () => {
    const labels = Object.values(COPY.coverage);

    expect(COPY.coverage.failed).not.toBe(COPY.coverage.needCli);
    expect(new Set(labels).size).toBe(labels.length);
  });

  it("滥用收录不可用时显示「未知」而非「无收录」（docs/api.md 3.1）", () => {
    expect(COPY.checks.O4.abuse.unknown).toContain("未知");
    expect(COPY.checks.O4.abuse.unknown).not.toBe(COPY.checks.O4.abuse.clean);
  });

  it("O3 失败文案不得把「测不出来」说成「没有 IPv6」（规格 2.3 / 验收标准 3）", () => {
    expect(COPY.checks.O3.failed).toContain("无法判定");
    expect(COPY.checks.O3.failed).not.toContain("不存在 IPv6 泄露");
  });

  it("页脚声明零留存（ADR-0008）", () => {
    expect(COPY.footer.privacy).toContain("不存储");
  });

  it("灰卡提示安装命令为 pip install ai-ipcheck（规格第 4 节）", () => {
    expect(COPY.actions.installCommand).toBe("pip install ai-ipcheck");
  });

  it("不使用「真实 IP」一词描述出口 IP（CONTEXT.md 禁用词）", () => {
    // C1 是仅 CLI 项，「本机真实 IP」是 CLI 的既有术语，允许出现在它自己的标题里。
    const webCopy = JSON.stringify({
      verdict: COPY.verdict,
      O1: COPY.checks.O1,
      O2: COPY.checks.O2,
      O3: COPY.checks.O3,
      O4: COPY.checks.O4,
    });

    expect(webCopy).not.toContain("真实 IP");
  });
});
