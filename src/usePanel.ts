// 面板状态机：把几条独立的检测流程收束成一个 PanelState，并派生覆盖度与综合结论。

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { fetchGeo, fetchRisk } from "./api";
import type { PanelState } from "./domain/checks";
import { INITIAL_PANEL } from "./domain/checks";
import { computeCoverage } from "./domain/coverage";
import { judgeDnsEgress } from "./domain/dnsEgress";
import { judgeIpv6 } from "./domain/ipv6";
import { browserTimezone, compareTimezone } from "./domain/timezone";
import type { GeoData } from "./domain/types";
import { judgeUdpEgress } from "./domain/udpEgress";
import { computeVerdict, verdictInputFrom } from "./domain/verdict";
import { useCopy } from "./i18n";
import { probeDnsEgress } from "./probes/dnsEgress";
import { probeIpify } from "./probes/ipify";
import { probeStun } from "./probes/stun";

function reasonOf(fallback: string, error: unknown): string {
  return error instanceof Error && error.message ? error.message : fallback;
}

/** 错误文案的语言只有一个真相来源：`useCopy()`（m3：去掉冗余的 `lang` 参数）。 */
export function usePanel() {
  const copy = useCopy();
  const [panel, setPanel] = useState<PanelState>(INITIAL_PANEL);
  /** 最近一次 O1 的结果，供 O5／O6 单独重试时取出口 IP／出口国，不必连带重跑 O1。 */
  const geoRef = useRef<GeoData | null>(null);

  // O1 与 O2 同源：O2 要拿出口 IP 的时区去比对，所以它们一起成功、一起失败、一起重试。
  // 返回值给 O5／O6：它们的**探测**不必等 O1，**比对**必须等（见 runDnsEgress／runUdpEgress）。
  const runGeo = useCallback(async (): Promise<GeoData | null> => {
    setPanel((prev) => ({
      ...prev,
      o1: { status: "running" },
      o2: { status: "running" },
    }));

    try {
      const geo = await fetchGeo(copy);
      geoRef.current = geo;
      setPanel((prev) => ({
        ...prev,
        o1: { status: "done", data: geo },
        o2: {
          status: "done",
          data: compareTimezone(browserTimezone(), geo.timezone),
        },
      }));
      return geo;
    } catch (error) {
      const failed = {
        status: "failed",
        reason: reasonOf(copy.errors.unknown, error),
      } as const;
      geoRef.current = null;
      setPanel((prev) => ({ ...prev, o1: failed, o2: failed }));
      return null;
    }
  }, [copy]);

  const runIpv6 = useCallback(async () => {
    setPanel((prev) => ({ ...prev, o3: { status: "running" } }));

    const { v4, v6 } = await probeIpify();
    setPanel((prev) => ({ ...prev, o3: judgeIpv6(v4, v6) }));
  }, []);

  /**
   * O5：探测立即发出，比对等 O1 落地。
   *
   * O1 失败 ⇒ 出口国未知 ⇒ **无从比对**（已完成、不产信号），而不是检测失败——
   * O5 自己的探测成功了，把它记成失败等于谎报一个刷新也解决不了的故障（契约 §2.5）。
   * 重试时不带参数，从 geoRef 取最近一次出口国。
   */
  const runDnsEgress = useCallback(
    async (
      geo: Promise<GeoData | null> = Promise.resolve(geoRef.current),
      signal?: AbortSignal,
    ) => {
      setPanel((prev) => ({ ...prev, o5: { status: "running" } }));

      const [probe, exit] = await Promise.all([probeDnsEgress(signal), geo]);
      if (signal?.aborted) return;
      setPanel((prev) => ({
        ...prev,
        o5: judgeDnsEgress(probe, exit?.country ?? null),
      }));
    },
    [],
  );

  /** O6：同上。`signal` 让组件卸载时关掉全部在途 RTCPeerConnection。 */
  const runUdpEgress = useCallback(
    async (
      geo: Promise<GeoData | null> = Promise.resolve(geoRef.current),
      signal?: AbortSignal,
    ) => {
      setPanel((prev) => ({ ...prev, o6: { status: "running" } }));

      const [probe, exit] = await Promise.all([probeStun(signal), geo]);
      if (signal?.aborted) return;
      setPanel((prev) => ({
        ...prev,
        o6: judgeUdpEgress(probe, exit?.ip ?? null),
      }));
    },
    [],
  );

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
    // 四条自动流程并发发起，O5／O6 不排在 O1 之后——它们只在**比对**那一步等 O1（geo）。
    const controller = new AbortController();
    const geo = runGeo();

    void runIpv6();
    void runDnsEgress(geo, controller.signal);
    void runUdpEgress(geo, controller.signal);

    // 卸载时关掉全部在途 RTCPeerConnection，并停掉 O5 还没打完的重试。
    return () => controller.abort();
  }, [runGeo, runIpv6, runDnsEgress, runUdpEgress]);

  const coverage = useMemo(() => computeCoverage(panel), [panel]);
  const verdict = useMemo(
    () => computeVerdict(verdictInputFrom(panel)),
    [panel],
  );

  return {
    panel,
    coverage,
    verdict,
    runGeo,
    runIpv6,
    runRisk,
    failRisk,
    runDnsEgress,
    runUdpEgress,
  };
}
