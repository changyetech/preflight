// 判级契约的 golden 向量（docs/verdict.md 第 7 节）。
//
// 这个文件与 cli/src/domain/verdict.rs 的 `golden` 模块读**同一份** docs/verdict-cases.json。
// 它不是 verdict.test.ts 的重复：那边测的是 Web 实现的内部结构（面板状态 → VerdictInput），
// 这边测的是**契约本身**——同一组输入，Web 与 CLI 必须给出契约声明的同一个结论。
// 改判级规则时两端会同时变红，漂移因此在 CI 就暴露，而不必等到有人去比对两个实现。

import { describe, expect, it } from "vitest";

import { riskLevelOf } from "../worker/proxycheck";
import type { Verdict, VerdictInput } from "../src/domain/verdict";
import { computeVerdict } from "../src/domain/verdict";
import casesFile from "../docs/verdict-cases.json";

/** 契约 §2 的信号域。用例里出现这之外的键一律判错——拼错的信号名会让用例静默失效。 */
const KNOWN_SIGNALS = [
  "tzMismatchCliEnv",
  "tzMismatchSystem",
  "ipv6Leak",
  "riskScore",
  "anonymous",
  "abuseListed",
  "tunOff",
] as const;

const KNOWN_SIDES = ["web", "cli"] as const;

type CaseSignals = {
  tzMismatchCliEnv?: boolean | null;
  tzMismatchSystem?: boolean | null;
  ipv6Leak?: boolean | null;
  riskScore?: number | null;
  anonymous?: boolean | null;
  abuseListed?: boolean | null;
  tunOff?: boolean | null;
};

type Case = {
  id: string;
  applies: string[];
  signals: CaseSignals;
  expect: { stage: string; level?: string };
};

const CASES = (casesFile as unknown as { cases: Case[] }).cases;

/**
 * 用例的扁平信号 → Web 实现的嵌套 `VerdictInput`。
 *
 * 两处语义映射必须小心：
 * - Web 的可测信号只有 `tzMismatchSystem` 与 `ipv6Leak`，两者都未知时 `signals` 整体为 null
 *   （「一个都没测成」与「测了都没问题」在类型层面就得是两回事）
 * - `riskScore` 未知 ⇒ `risk` 整体为 null；`riskScore` 已知而 `abuseListed` 未知 ⇒
 *   `risk` 存在、`abuseListed` 为 null（StopForumSpam 挂了不该连累 proxycheck 的结果）
 * - `riskScore` 与 `anonymous` **必定成对**（契约 §2.3）：判「高」的阈值由后者决定
 */
function toVerdictInput(signals: CaseSignals): VerdictInput {
  const timezone = signals.tzMismatchSystem ?? null;
  const ipv6 = signals.ipv6Leak ?? null;
  const riskScore = signals.riskScore ?? null;

  return {
    signals:
      timezone === null && ipv6 === null
        ? null
        : { timezoneMismatch: timezone === true, ipv6Leak: ipv6 === true },
    risk:
      riskScore === null
        ? null
        : {
            riskScore,
            anonymous: signals.anonymous === true,
            abuseListed: signals.abuseListed ?? null,
          },
  };
}

function levelOf(verdict: Verdict): string | undefined {
  return "level" in verdict ? verdict.level : undefined;
}

describe("判级契约 · golden 向量", () => {
  it("每条用例的信号名都在契约的信号域内", () => {
    for (const testCase of CASES) {
      for (const key of Object.keys(testCase.signals)) {
        expect(
          KNOWN_SIGNALS as readonly string[],
          `用例 ${testCase.id} 用了契约里没有的信号 ${key}`,
        ).toContain(key);
      }
    }
  });

  it("每条用例都声明了有效且非空的适用侧", () => {
    for (const testCase of CASES) {
      expect(testCase.applies.length, `用例 ${testCase.id} 没有适用侧`).toBeGreaterThan(0);
      for (const side of testCase.applies) {
        expect(
          KNOWN_SIDES as readonly string[],
          `用例 ${testCase.id} 的适用侧 ${side} 拼错了`,
        ).toContain(side);
      }
    }
  });

  it("Web 用例不含 Web 结构性测不到的信号", () => {
    // 契约 §5.2：`tzMismatchCliEnv` 与 `tunOff` 只有 CLI 能测。
    // 一条 Web 用例若声明了它们，说明用例本身写错了。
    for (const testCase of CASES.filter((c) => c.applies.includes("web"))) {
      expect(testCase.signals.tzMismatchCliEnv, testCase.id).toBeUndefined();
      expect(testCase.signals.tunOff, testCase.id).toBeUndefined();
    }
  });

  it("riskScore 与 anonymous 成对出现（契约 §2.3）", () => {
    // 判「高」的阈值由 anonymous 决定，只给其中一个是判不了的状态。
    for (const testCase of CASES) {
      const hasScore = testCase.signals.riskScore != null;
      const hasAnonymous = testCase.signals.anonymous != null;
      expect(hasAnonymous, `用例 ${testCase.id}：riskScore 与 anonymous 必须成对`).toBe(
        hasScore,
      );
    }
  });

  it("insufficient 的用例不带档位，其余必须带", () => {
    for (const testCase of CASES) {
      if (testCase.expect.stage === "insufficient") {
        expect(testCase.expect.level, testCase.id).toBeUndefined();
      } else {
        expect(testCase.expect.level, testCase.id).toBeDefined();
      }
    }
  });

  const webCases = CASES.filter((c) => c.applies.includes("web"));

  it("存在 Web 用例（筛选逻辑没坏）", () => {
    expect(webCases.length).toBeGreaterThan(0);
  });

  for (const testCase of webCases) {
    it(`${testCase.id}`, () => {
      const verdict = computeVerdict(toVerdictInput(testCase.signals));
      expect(verdict.stage).toBe(testCase.expect.stage);
      expect(levelOf(verdict)).toBe(testCase.expect.level);
    });
  }
});

describe("分项分级与综合结论是两把尺子（契约 §6）", () => {
  it("分项分级对齐 proxycheck v3 自己的四档", () => {
    // v3: 0–25 Allow / 26–50、51–75 Challenge / 76–100 Deny，收成三色。
    expect(riskLevelOf(25)).toBe("low");
    expect(riskLevelOf(26)).toBe("medium");
    expect(riskLevelOf(75)).toBe("medium");
    expect(riskLevelOf(76)).toBe("high");
  });

  it("非匿名时两把尺子同界，都在 76", () => {
    expect(riskLevelOf(75)).toBe("medium");
    expect(
      levelOf(
        computeVerdict({
          signals: { timezoneMismatch: false, ipv6Leak: false },
          risk: { riskScore: 75, anonymous: false, abuseListed: false },
        }),
      ),
    ).toBe("low");

    expect(riskLevelOf(76)).toBe("high");
    expect(
      levelOf(
        computeVerdict({
          signals: { timezoneMismatch: false, ipv6Leak: false },
          risk: { riskScore: 76, anonymous: false, abuseListed: false },
        }),
      ),
    ).toBe("high");
  });

  it("匿名时两把尺子不同界：结论已高，分项仍黄", () => {
    // 结论 51 起判高，分项要到 76 才转红。这一档全靠呈现层的文字解释（契约 §6）——
    // 用户看到高风险却找不到哪一项显红，会以为结论算错了。
    expect(riskLevelOf(60)).toBe("medium");
    expect(
      levelOf(
        computeVerdict({
          signals: { timezoneMismatch: false, ipv6Leak: false },
          risk: { riskScore: 60, anonymous: true, abuseListed: false },
        }),
      ),
    ).toBe("high");
  });
});
