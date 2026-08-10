// 首屏结论区（规格第 4 节 / ADR-0004 / ADR-0005）。
//
// 硬约束：结论与覆盖度必须同时呈现，且初步结论恒带「初步 · 未含 IP 风险评分」标注。
// 任何只显示档位不显示覆盖度的呈现都视为缺陷（ADR-0004）。

import { COPY } from "../copy";
import type { OnlineCheck } from "../domain/checks";
import type { Coverage } from "../domain/coverage";
import type { GeoData } from "../domain/types";
import type { Verdict as VerdictValue } from "../domain/verdict";

const SUMMARY = {
  preliminary: {
    low: COPY.verdict.summary.preliminaryLow,
    medium: COPY.verdict.summary.preliminaryMedium,
  },
  full: {
    low: COPY.verdict.summary.fullLow,
    medium: COPY.verdict.summary.fullMedium,
    high: COPY.verdict.summary.fullHigh,
  },
} as const;

/** 分档取文案。初步结论没有「高」这一档、数据不足没有任何档——类型层面也没有，无需兜底。 */
function summaryOf(verdict: VerdictValue): string {
  switch (verdict.stage) {
    case "insufficient":
      return COPY.verdict.summary.insufficient;
    case "preliminary":
      return SUMMARY.preliminary[verdict.level];
    case "full":
      return SUMMARY.full[verdict.level];
  }
}

function CoverageBar({ coverage }: { coverage: Coverage }) {
  return (
    <p className="coverage">
      <span className="chip chip-done">
        {COPY.coverage.done} {coverage.done}
      </span>
      <span className="chip chip-cli">
        {COPY.coverage.needCli} {coverage.needCli}
      </span>
      {/* 失败档恒久呈现（ADR-0004），但为 0 时不上警示色——0 个失败不该看起来像个警告。 */}
      <span className={`chip ${coverage.failed > 0 ? "chip-failed" : ""}`}>
        {COPY.coverage.failed} {coverage.failed}
      </span>
      {/* 未触发的按需项既非已完成也非失败，单列一档，不与上面两档混计（ADR-0004）。 */}
      {coverage.pending > 0 ? (
        <span className="chip chip-pending">
          {COPY.coverage.pending} {coverage.pending}
        </span>
      ) : null}
      <span className="chip chip-total">{COPY.coverage.total}</span>
    </p>
  );
}

function location(geo: GeoData): string {
  const parts = [geo.city, geo.country].filter(Boolean);
  return parts.length > 0 ? parts.join(" · ") : COPY.checks.O1.unknown;
}

export function VerdictPanel({
  geo,
  verdict,
  coverage,
}: {
  geo: OnlineCheck<GeoData>;
  verdict: VerdictValue;
  coverage: Coverage;
}) {
  const known = geo.status === "done" ? geo.data : null;
  // 数据不足时没有 level 可取——配色与文案都走中性档，绝不落到低风险绿。
  const level = verdict.stage === "insufficient" ? "none" : verdict.level;

  return (
    <section className={`verdict verdict-${level}`}>
      <p className="exit-ip-label">{COPY.verdict.exitIpLabel}</p>
      <p className="exit-ip">{known?.ip ?? COPY.verdict.exitIpUnknown}</p>
      <p className="exit-location">
        {known ? location(known) : COPY.checks.O1.unknown}
      </p>
      <p className="exit-ip-note">{COPY.verdict.exitIpNote}</p>

      <p className="level">
        <span className={`level-dot level-${level}`} aria-hidden="true" />
        {verdict.stage === "insufficient"
          ? COPY.verdict.insufficientLabel
          : COPY.verdict.level[verdict.level]}
        {/* 数据不足时不挂「初步 / 完整」标注：还没有结论，也就无所谓这个结论含不含 O4。 */}
        {verdict.stage === "insufficient" ? null : (
          <span className="stage-badge">
            {verdict.stage === "preliminary"
              ? COPY.verdict.preliminaryBadge
              : COPY.verdict.fullBadge}
          </span>
        )}
      </p>
      <p className="summary">{summaryOf(verdict)}</p>

      <CoverageBar coverage={coverage} />
      <p className="coverage-hint">{COPY.coverage.hint}</p>
    </section>
  );
}
