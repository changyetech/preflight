// 面板状态机：把三条独立的检测流程收束成一个 PanelState，并派生覆盖度与综合结论。

import { useCallback, useEffect, useMemo, useState } from "react";

import { fetchGeo, fetchRisk } from "./api";
import type { PanelState } from "./domain/checks";
import { INITIAL_PANEL } from "./domain/checks";
import { computeCoverage } from "./domain/coverage";
import { judgeIpv6 } from "./domain/ipv6";
import { browserTimezone, compareTimezone } from "./domain/timezone";
import { computeVerdict, verdictInputFrom } from "./domain/verdict";
import { useCopy } from "./i18n";
import { probeIpify } from "./probes/ipify";

function reasonOf(fallback: string, error: unknown): string {
  return error instanceof Error && error.message ? error.message : fallback;
}

/** 错误文案的语言只有一个真相来源：`useCopy()`（m3：去掉冗余的 `lang` 参数）。 */
export function usePanel() {
  const copy = useCopy();
  const [panel, setPanel] = useState<PanelState>(INITIAL_PANEL);

  // O1 与 O2 同源：O2 要拿出口 IP 的时区去比对，所以它们一起成功、一起失败、一起重试。
  const runGeo = useCallback(async () => {
    setPanel((prev) => ({
      ...prev,
      o1: { status: "running" },
      o2: { status: "running" },
    }));

    try {
      const geo = await fetchGeo(copy);
      setPanel((prev) => ({
        ...prev,
        o1: { status: "done", data: geo },
        o2: {
          status: "done",
          data: compareTimezone(browserTimezone(), geo.timezone),
        },
      }));
    } catch (error) {
      const failed = {
        status: "failed",
        reason: reasonOf(copy.errors.unknown, error),
      } as const;
      setPanel((prev) => ({ ...prev, o1: failed, o2: failed }));
    }
  }, [copy]);

  const runIpv6 = useCallback(async () => {
    setPanel((prev) => ({ ...prev, o3: { status: "running" } }));

    const { v4, v6 } = await probeIpify();
    setPanel((prev) => ({ ...prev, o3: judgeIpv6(v4, v6) }));
  }, []);

  /** O4 按需触发，token 由调用方（卡片内的 Turnstile 组件）取得后传入。 */
  const runRisk = useCallback(
    async (turnstileToken: string) => {
      setPanel((prev) => ({ ...prev, o4: { status: "running" } }));

      try {
        const data = await fetchRisk(turnstileToken, copy);
        setPanel((prev) => ({ ...prev, o4: { status: "done", data } }));
      } catch (error) {
        // proxycheck 不可用（5001）是检测失败，不是低风险——绝不能呈现成「没查出问题」。
        setPanel((prev) => ({
          ...prev,
          o4: {
            status: "failed",
            reason: reasonOf(copy.errors.unknown, error),
          },
        }));
      }
    },
    [copy],
  );

  const failRisk = useCallback((reason: string) => {
    setPanel((prev) => ({ ...prev, o4: { status: "failed", reason } }));
  }, []);

  useEffect(() => {
    void runGeo();
    void runIpv6();
  }, [runGeo, runIpv6]);

  const coverage = useMemo(() => computeCoverage(panel), [panel]);
  const verdict = useMemo(
    () => computeVerdict(verdictInputFrom(panel)),
    [panel],
  );

  return { panel, coverage, verdict, runGeo, runIpv6, runRisk, failRisk };
}
