// 检测项状态模型的类型层面约束（ui-panel 计划步骤 1）。
// 这里的断言主要靠 @ts-expect-error：非法状态必须**编译不过**，`make check` 的 tsc -b 会强制这一点。

import { describe, expect, it } from "vitest";

import type { CliCheck, OnlineCheck, PanelState } from "../src/domain/checks";
import {
  CLI_CHECK,
  CLI_CHECK_IDS,
  INITIAL_PANEL,
  ONLINE_CHECK_IDS,
  TOTAL_CHECKS,
} from "../src/domain/checks";

describe("检测项状态模型", () => {
  it("可在线 6 项 + 仅 CLI 4 项 = 覆盖度分母 10", () => {
    // O5／O6 是分流泄露两项，均为可在线（ADR-0014）。
    expect(ONLINE_CHECK_IDS).toEqual(["O1", "O2", "O3", "O4", "O5", "O6"]);
    // C5（原厂商端点检测）已移除，编号废弃不复用（ADR-0013）。
    expect(CLI_CHECK_IDS).toEqual(["C1", "C2", "C3", "C4"]);
    expect(TOTAL_CHECKS).toBe(10);
    // 分母由两张 ID 表派生，不是写死的常量——加检测项必然改到它。
    expect(TOTAL_CHECKS).toBe(ONLINE_CHECK_IDS.length + CLI_CHECK_IDS.length);
  });

  it("仅 CLI 项无法处于「检测中」等非终态", () => {
    // @ts-expect-error 仅 CLI 项不存在「检测中」
    const running: CliCheck = { status: "running" };
    // @ts-expect-error 仅 CLI 项不存在「已完成」
    const done: CliCheck = { status: "done" };
    // @ts-expect-error 仅 CLI 项不存在「检测失败」
    const failed: CliCheck = { status: "failed", reason: "x" };

    expect([running, done, failed].every(Boolean)).toBe(true);
    expect(CLI_CHECK.status).toBe("needCli");
  });

  it("可在线项无法取到「需 CLI」这一终态", () => {
    // @ts-expect-error 可在线项不存在「需 CLI」
    const needCli: OnlineCheck<string> = { status: "needCli" };
    // @ts-expect-error 「已完成」必须携带数据
    const dataless: OnlineCheck<string> = { status: "done" };

    expect([needCli, dataless].every(Boolean)).toBe(true);
  });

  it("面板初态：O1–O3 与 O5／O6 自动开跑，O4 按需不自动执行", () => {
    const panel: PanelState = INITIAL_PANEL;

    expect(panel.o1.status).toBe("running");
    expect(panel.o2.status).toBe("running");
    expect(panel.o3.status).toBe("running");
    expect(panel.o4.status).toBe("idle");
    // 按需的唯一理由是消耗本站共享的第三方配额，O5／O6 都不消耗（契约 §1）。
    expect(panel.o5.status).toBe("running");
    expect(panel.o6.status).toBe("running");
  });
});
