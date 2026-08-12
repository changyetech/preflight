// 判级契约的 golden 向量（docs/verdict.md 第 7 节）。
//
// 这个文件与 cli/src/domain/verdict.rs 的 `golden` 模块读**同一份** docs/verdict-cases.json。
// 它不是 verdict.test.ts 的重复：那边测的是 Web 实现的内部结构（面板状态 → VerdictInput），
// 这边测的是**契约本身**——同一组输入，Web 与 CLI 必须给出契约声明的同一个结论。
// 改判级规则时两端会同时变红，漂移因此在 CI 就暴露，而不必等到有人去比对两个实现。

import { describe, expect, it } from "vitest";

import { riskLevelOf } from "../worker/proxycheck";
import { compareDnsEgress } from "../src/domain/dnsEgress";
import { judgeUdpEgress } from "../src/domain/udpEgress";
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
  // O5／O6 给的是**原始观测值**而非派生布尔量（本文件 conventions.signals），
  // 因此这里出现的是四个观测字段，两个新信号由判定层从它们推出来。
  "dnsEcsCountry",
  "exitCountry",
  "stunReflexiveIps",
  "exitIp",
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
  dnsEcsCountry?: string | null;
  exitCountry?: string | null;
  stunReflexiveIps?: string[] | null;
  exitIp?: string | null;
};

type Case = {
  id: string;
  applies: string[];
  signals: CaseSignals;
  expect: { stage: string; level?: string };
  /** 指向一条**未命中**的基准用例，含义是「本条与它的结论必须完全相同」（见文件 conventions）。 */
  pairsWith?: string;
};

const CASES = (casesFile as unknown as { cases: Case[] }).cases;

const CASE_BY_ID = new Map(CASES.map((testCase) => [testCase.id, testCase]));

const webCases = CASES.filter((testCase) => testCase.applies.includes("web"));

/**
 * 解析 `pairsWith` 指向的基准用例。
 *
 * **悬空引用必须响亮失败**：那是数据错误，静默跳过会让配对断言整条失效——
 * 一条改错了 id 的用例从此再也证明不了任何事，而它看起来仍然是绿的。
 */
function baselineOf(testCase: Case, byId: Map<string, Case>): Case {
  const baseline = testCase.pairsWith && byId.get(testCase.pairsWith);
  if (!baseline) {
    throw new Error(
      `用例 ${testCase.id} 的 pairsWith 指向不存在的用例 ${testCase.pairsWith}`,
    );
  }
  return baseline;
}

/** 两条用例的 signals 中取值不同的键。 */
function differingSignals(a: CaseSignals, b: CaseSignals): string[] {
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  return [...keys].filter(
    (key) =>
      JSON.stringify(a[key as keyof CaseSignals]) !==
      JSON.stringify(b[key as keyof CaseSignals]),
  );
}

/**
 * 用例的扁平信号 → Web 实现的嵌套 `VerdictInput`。
 *
 * 两处语义映射必须小心：
 * - Web 的可测信号只有 `tzMismatchSystem` 与 `ipv6Leak`，两者都未知时 `signals` 整体为 null
 *   （「一个都没测成」与「测了都没问题」在类型层面就得是两回事）
 * - `riskScore` 未知 ⇒ `risk` 整体为 null；`riskScore` 已知而 `abuseListed` 未知 ⇒
 *   `risk` 存在、`abuseListed` 为 null（StopForumSpam 挂了不该连累 proxycheck 的结果）
 * - `riskScore` 与 `anonymous` **必定成对**（契约 §2.3）：判「高」的阈值由后者决定
 * - O5／O6 的观测值**过真正的判定层**（`compareDnsEgress` / `judgeUdpEgress`），不在这里
 *   另写一套推导——否则向量测的就是 harness 自己，两端实现漂移了也照样绿
 */
function toVerdictInput(signals: CaseSignals): VerdictInput {
  const timezone = signals.tzMismatchSystem ?? null;
  const ipv6 = signals.ipv6Leak ?? null;
  const riskScore = signals.riskScore ?? null;
  const dnsEgress = dnsEgressOf(signals);
  const udpEgress = udpEgressOf(signals);

  return {
    signals:
      timezone === null &&
      ipv6 === null &&
      dnsEgress === null &&
      udpEgress === null
        ? null
        : {
            timezoneMismatch: timezone === true,
            ipv6Leak: ipv6 === true,
            dnsEgressLeak: dnsEgress === true,
            udpEgressMismatch: udpEgress === true,
          },
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

/**
 * O5 的观测值 → 信号。`null` = 该检测项没产出结果（未执行或失败），与
 * `verdictInputFrom` 里「`status !== "done"` 就不进 signals」是同一条语义。
 *
 * **「无从比对」返回 `false` 而不是 `null`**：探测成功了，只是回答里不含可判定的信息——
 * 它是一个产出，只不过不贡献信号（契约 §2.5）。
 */
function dnsEgressOf(signals: CaseSignals): boolean | null {
  if (signals.dnsEcsCountry == null && signals.exitCountry == null) return null;

  const ecs =
    signals.dnsEcsCountry == null
      ? ({ known: false, reason: "noEcs" } as const)
      : ({ known: true, iso2: signals.dnsEcsCountry } as const);
  const comparison = compareDnsEgress(ecs, signals.exitCountry ?? null);

  return comparison.comparable && comparison.leak;
}

/** O6 同上。`null` 与空数组同义 ⇒ `N_ok = 0` ⇒ 检测失败 ⇒ 没有产出（本文件 conventions）。 */
function udpEgressOf(signals: CaseSignals): boolean | null {
  if (signals.stunReflexiveIps == null && signals.exitIp == null) return null;

  const state = judgeUdpEgress(
    {
      reflexiveIps: signals.stunReflexiveIps ?? [],
      // 向量不区分「STUN 没答」与「浏览器禁用 WebRTC」——两者同为检测失败，
      // 差别只在呈现层的失败原因（契约 §5.6），不进判级。
      webrtcSupported: true,
    },
    signals.exitIp ?? null,
  );
  if (state.status !== "done") return null;

  return state.data.comparable && state.data.mismatch;
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
      expect(
        testCase.applies.length,
        `用例 ${testCase.id} 没有适用侧`,
      ).toBeGreaterThan(0);
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
      expect(
        hasAnonymous,
        `用例 ${testCase.id}：riskScore 与 anonymous 必须成对`,
      ).toBe(hasScore);
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

/**
 * `pairsWith`：证明某个取值是**无从比对**而不是**未命中**。
 *
 * 单看一条用例分不出这两者——只有让它与一条已知未命中的用例算出**同一个结论**才说得清。
 * 断言的是两条各自**算出来**的 verdict 相等，而不是各自等于某个硬编码值：后者会退化成
 * 把同一个期望写两遍，那样一条写错 `expect` 的新用例照样能全绿。
 *
 * 这与 `tests/verdict.test.ts` 里那条判定层断言是两层保障：那边锁的是实现，
 * 这边锁的是**向量本身**——将来有人加一条「无从比对」用例却把 `expect` 写错，只有这里抓得住。
 */
describe("判级契约 · pairsWith 配对（无从比对 ≠ 未命中）", () => {
  const pairedWebCases = webCases.filter(
    (testCase) => testCase.pairsWith !== undefined,
  );

  it("存在带 pairsWith 的 Web 用例（筛选逻辑没坏）", () => {
    expect(pairedWebCases.length).toBeGreaterThan(0);
  });

  for (const testCase of pairedWebCases) {
    it(`${testCase.id} 与基准 ${testCase.pairsWith} 结论相同`, () => {
      const baseline = baselineOf(testCase, CASE_BY_ID);

      // 基准必须同样适用于本端，否则拿它作对照不成立。
      expect(baseline.applies, baseline.id).toContain("web");

      // 除被比对的那一个字段外其余 signals 必须逐一相同，否则「结论相同」证明不了
      // 是那个字段不贡献信号——可能是别处的差异把结论又拉了回来。
      expect(
        differingSignals(testCase.signals, baseline.signals),
        `${testCase.id} 与 ${baseline.id} 只应差一个被比对的字段`,
      ).toHaveLength(1);

      expect(computeVerdict(toVerdictInput(testCase.signals))).toEqual(
        computeVerdict(toVerdictInput(baseline.signals)),
      );
    });
  }

  it("pairsWith 指向不存在的用例时响亮失败，不静默跳过", () => {
    const dangling: Case = {
      id: "dangling",
      applies: ["web"],
      signals: {},
      expect: { stage: "insufficient" },
      pairsWith: "no-such-case",
    };

    expect(() => baselineOf(dangling, CASE_BY_ID)).toThrow(/no-such-case/);
  });
});

/** 这一组只关心风险分那把尺子，四个中档信号一律未命中。 */
const NO_SIGNALS = {
  timezoneMismatch: false,
  ipv6Leak: false,
  dnsEgressLeak: false,
  udpEgressMismatch: false,
};

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
          signals: NO_SIGNALS,
          risk: { riskScore: 75, anonymous: false, abuseListed: false },
        }),
      ),
    ).toBe("low");

    expect(riskLevelOf(76)).toBe("high");
    expect(
      levelOf(
        computeVerdict({
          signals: NO_SIGNALS,
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
          signals: NO_SIGNALS,
          risk: { riskScore: 60, anonymous: true, abuseListed: false },
        }),
      ),
    ).toBe("high");
  });
});
