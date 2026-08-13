// 文案中有几条是 ADR / 规格的硬性要求，删掉即违约。这些断言的作用是让后续改版删不掉它们。
//
// 分两组：
// - 与语言无关的硬性内容（第三方域名、命令、$TZ 这类标识符）对**每个语种**断言——
//   语种终态只有 en + zh-hans 两份完整文案（规格 §2 决策 9：删除字段级回落，
//   两语种都强制译全），逐语种跑一遍才锁得住。
// - 措辞层面的要求（初步结论怎么标、失败不许说成安全、禁用词）按语言分别断言。

import { describe, expect, it } from "vitest";

import { COPY, COPY_ZH_HANS } from "../src/copy";
import { TOTAL_CHECKS } from "../src/domain/checks";

describe.each([["en", COPY] as const, ["zh-hans", COPY_ZH_HANS] as const])(
  "受 ADR 约束的文案 · %s",
  (_id, copy) => {
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

    // 契约 §5.5 呈现约束：缺了这句，用户会拿浏览器 DoH 的绿色结论去为命令行工具背书。
    it("O5 必须写明本项测的是浏览器 DNS，且提示 DoH 与 CLI 的差异（契约 §5.5）", () => {
      expect(copy.checks.O5.scopeNote).toContain("DoH");
      expect(copy.checks.O5.scopeNote.toUpperCase()).toContain("CLI");
    });

    // ECS 缺失时状态是「已完成」而非「失败」，卡片必须点明是 DNS 服务商不发 ECS（brief 硬约束）。
    it("O5 的 ECS 缺失说明必须点出 ECS／EDNS 相关字样", () => {
      const text = copy.checks.O5.noEcs.toUpperCase();
      expect(text.includes("ECS") || text.includes("EDNS")).toBe(true);
    });

    // O5 三种「无从比对」各自独立成句，不得共用措辞（契约 §2.5 硬约束 3）。
    it("O5 三种「无从比对」文案互不相同", () => {
      const reasons = [
        copy.checks.O5.noEcs,
        copy.checks.O5.unmappedCountry,
        copy.checks.O5.unknownExitCountry,
      ];
      expect(new Set(reasons).size).toBe(reasons.length);
    });

    // 契约 §5.6 呈现约束：WebRTC 被禁用与探测超时的可恢复性相反，文案必须能区分。
    it("O6 两种失败原因文案不同，且禁用 WebRTC 那条写明 CLI 不受影响", () => {
      expect(copy.checks.O6.webrtcUnavailable).not.toBe(
        copy.checks.O6.stunUnanswered,
      );
      expect(copy.checks.O6.webrtcUnavailable.toUpperCase()).toContain(
        "WEBRTC",
      );
      expect(copy.checks.O6.webrtcUnavailable.toUpperCase()).toContain("CLI");
    });

    // O6「无从比对」（stunDisagree／familyMismatch／unknownExitIp）要与「未命中」在措辞上可区分。
    it("O6「无从比对」三条文案与「未命中」互不相同", () => {
      const distinct = [
        copy.checks.O6.noMismatch,
        copy.checks.O6.familyMismatch,
        copy.checks.O6.unknownExitIp,
        copy.checks.O6.stunDisagree,
      ];
      expect(new Set(distinct).size).toBe(distinct.length);
    });

    // ADR-0008：O5／O6 自动执行、没有触发控件，前置披露必须列出全部新增第三方域名。
    it("首屏前置告知列出 O5／O6 的第三方域名（ADR-0008）", () => {
      expect(copy.footer.thirdParty).toContain("ip-api.com");
      expect(copy.footer.thirdParty).toContain("stun.cloudflare.com");
      expect(copy.footer.thirdParty).toContain("stun.l.google.com");
    });

    // O5／O6 自动执行、无控件可挂披露，只能就地放在卡片说明位（与 O3 同一套处理）。
    it("O5／O6 的第三方披露写明各自访问的域名", () => {
      expect(copy.checks.O5.thirdPartyNote).toContain("ip-api.com");
      expect(copy.checks.O6.thirdPartyNote).toContain("stun.cloudflare.com");
      expect(copy.checks.O6.thirdPartyNote).toContain("stun.l.google.com");
    });

    // O5／O6 的重试按钮同样是触发控件，文案不得落回通用「重试」（ADR-0008 一致执行）。
    it("O5／O6 的重试按钮写明再次访问哪个第三方", () => {
      expect(copy.checks.O5.retryLabel).not.toBe(copy.actions.retry);
      expect(copy.checks.O5.retryLabel).toContain("ip-api.com");
      expect(copy.checks.O6.retryLabel).not.toBe(copy.actions.retry);
      expect(copy.checks.O6.retryLabel).toContain("stun.cloudflare.com");
    });

    // 覆盖度分母改为 10 后，总数文案不得停留在旧的 8（呈现层随分母变化联动）。
    // 断言串里唯一出现的数字就是 TOTAL_CHECKS，而不是「不许出现字符 8」——
    // 后者一旦分母哪天变成 8／18／28 会跟前一行自相矛盾（评审 M2）。
    it("覆盖度总数文案与 TOTAL_CHECKS 一致，串里没有另一个数字", () => {
      const digits = copy.coverage.total.match(/\d+/g) ?? [];
      expect(digits).toEqual([String(TOTAL_CHECKS)]);
    });
  },
);

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

  // 契约 §2.1／§2.5 硬约束 1：resolver 归属只展示不判定。空断言（`toBeTruthy`）证明不了
  // 这句话说了什么，必须匹配到「不影响判定结论」这层意思本身（评审 M3）。
  it("O5 的 resolverNote 必须写明它不参与判定", () => {
    expect(COPY.checks.O5.resolverNote).toContain("doesn't affect the verdict");
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

  // 契约 §2.1／§2.5 硬约束 1：resolver 归属只展示不判定，中文版同样要匹配到这层意思本身。
  it("O5 的 resolverNote 必须写明它不参与判定", () => {
    expect(COPY_ZH_HANS.checks.O5.resolverNote).toContain("不参与判定");
  });
});
