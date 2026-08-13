// 首屏结论控制台（规格第 4 节 / ADR-0004 / ADR-0005 / 原型 .console）。
//
// 硬约束：结论与覆盖度必须同时呈现，且初步结论恒带「初步 · 未含 IP 风险评分」标注。
// 任何只显示档位不显示覆盖度的呈现都视为缺陷（ADR-0004）——本组件把两者钉死在同一个
// 无条件渲染路径里，没有任何分支能只画结论不画覆盖度。
//
// 配色不写在 `.level`/`.level-dot` 自身的修饰类上，而是让它们继承祖先 `.console.v-${level}`
// 的上下文色（原型写法）：`.v-none` 没有绿色规则可继承，`insufficient` 档结构上就拿不到
// 低风险的绿——不是靠一处判断把绿挡住，而是绿色规则本身只在 v-low 下存在。

import type { Copy } from "../copy";
import type { CoverageCell } from "../coverageMeter";
import type { OnlineCheck } from "../domain/checks";
import type { Coverage } from "../domain/coverage";
import type { GeoData } from "../domain/types";
import type { Verdict as VerdictValue } from "../domain/verdict";
import { useCopy } from "../i18n";

/** 分档取文案。初步结论没有「高」这一档、数据不足没有任何档——类型层面也没有，无需兜底。 */
function summaryOf(copy: Copy, verdict: VerdictValue): string {
  switch (verdict.stage) {
    case "insufficient":
      return copy.verdict.summary.insufficient;
    case "preliminary":
      return verdict.level === "low"
        ? copy.verdict.summary.preliminaryLow
        : copy.verdict.summary.preliminaryMedium;
    case "full":
      return verdict.level === "low"
        ? copy.verdict.summary.fullLow
        : verdict.level === "medium"
          ? copy.verdict.summary.fullMedium
          : copy.verdict.summary.fullHigh;
  }
}

function location(copy: Copy, geo: GeoData): string {
  const parts = [geo.city, geo.country].filter(Boolean);
  return parts.length > 0 ? parts.join(" · ") : copy.checks.O1.unknown;
}

/**
 * 运营商 / ASN 字段格式化，规则与 O1Card（src/components/cards.tsx）同名字段一致。
 * 就地重复而不从 cards.tsx 导出——cards.tsx 是 O1-O6 卡片流的地盘（W3 任务范围），
 * 控制台的 geo-grid 与卡片是两处独立呈现，不建立跨任务的耦合。
 */
function asnOf(geo: GeoData, unknown: string): string {
  return geo.asOrganization
    ? `${geo.asOrganization}${geo.asn ? ` (AS${geo.asn})` : ""}`
    : unknown;
}

/**
 * 10 格覆盖度 meter（规格 §4 要点 5 / W6 可达性走查，原型 refs/ipcheck-web-redesign.html:837
 * 的 `role="img" aria-label="覆盖度分布"`，动态描述取自同处 `meter.setAttribute("aria-label", ...)`）。
 * 10 个色块对屏幕阅读器就是一张图，必须有随状态更新的文字描述，标 aria-hidden 会让这块信息
 * 对读屏用户整段消失。文字描述不写死在 JSX 里，而是拼接 CoverageChips 已在用的同一批
 * 两语种 Copy 片段（coverage.total/done/needCli/failed/pending）+ 数字，不新增文案键。
 */
function CoverageMeter({
  copy,
  coverage,
  cells,
}: {
  copy: Copy;
  coverage: Coverage;
  cells: CoverageCell[];
}) {
  const label = [
    `${copy.coverage.total}: ${copy.coverage.done} ${coverage.done}`,
    `${copy.coverage.needCli} ${coverage.needCli}`,
    `${copy.coverage.failed} ${coverage.failed}`,
    `${copy.coverage.pending} ${coverage.pending}`,
  ].join(", ");

  return (
    <div className="cov-meter" role="img" aria-label={label}>
      {cells.map((cell, i) => (
        <span key={i} className={`cov-cell is-${cell}`} />
      ))}
    </div>
  );
}

function CoverageChips({ copy, coverage }: { copy: Copy; coverage: Coverage }) {
  return (
    <div className="cov-chips">
      <span className="cov-chip">
        <i className="sw-done" aria-hidden="true" />
        {copy.coverage.done} <b>{coverage.done}</b>
      </span>
      <span className="cov-chip">
        <i className="sw-cli" aria-hidden="true" />
        {copy.coverage.needCli} <b>{coverage.needCli}</b>
      </span>
      {/* 失败档恒久呈现（ADR-0004），零值时把数字调暗，而不是把整块染成警示色——
          0 个失败不该看起来像个警告。 */}
      <span
        className="cov-chip"
        data-zero={coverage.failed === 0 ? "true" : undefined}
      >
        <i className="sw-failed" aria-hidden="true" />
        {copy.coverage.failed} <b>{coverage.failed}</b>
      </span>
      {/* 未触发的按需项既非已完成也非失败，单列一档，不与上面两档混计（ADR-0004）。 */}
      {coverage.pending > 0 ? (
        <span className="cov-chip">
          <i className="sw-ondemand" aria-hidden="true" />
          {copy.coverage.pending} <b>{coverage.pending}</b>
        </span>
      ) : null}
      <span className="cov-chip cov-chip-total">
        <b>{copy.coverage.total}</b>
      </span>
    </div>
  );
}

function CoverageSection({
  copy,
  coverage,
  cells,
}: {
  copy: Copy;
  coverage: Coverage;
  cells: CoverageCell[];
}) {
  return (
    <div className="cov">
      <p className="eyebrow">{copy.coverage.label}</p>
      <CoverageMeter copy={copy} coverage={coverage} cells={cells} />
      <CoverageChips copy={copy} coverage={coverage} />
      <p className="cov-hint">{copy.coverage.hint}</p>
    </div>
  );
}

export function VerdictPanel({
  geo,
  verdict,
  coverage,
  cells,
}: {
  geo: OnlineCheck<GeoData>;
  verdict: VerdictValue;
  coverage: Coverage;
  cells: CoverageCell[];
}) {
  const COPY = useCopy();
  const known = geo.status === "done" ? geo.data : null;
  // 数据不足时没有 level 可取——配色与文案都走中性档，绝不落到低风险绿。
  const level = verdict.stage === "insufficient" ? "none" : verdict.level;
  // 「进行中」直接问格子数组，不必再单独扫一遍 panel——meter 已经把 running 拆出来了。
  const checking = cells.includes("running");

  return (
    // aria-busy 挂在整个控制台区域：结论/覆盖度只要还有在线项在跑就没定型，
    // 与原型给每张 O1-O6 卡片挂 aria-busy 是同一条规则在控制台这一级的呼应
    // （卡片自己的 aria-busy 归 W3，这里只管控制台整体，不重复也不越界）。
    <section className={`console v-${level}`} id="verdict" aria-busy={checking}>
      <div className="console-bar">
        <span className="live" data-live={checking ? "true" : "false"}>
          <i aria-hidden="true" />
          {checking ? COPY.verdict.live.checking : COPY.verdict.live.ready}
        </span>
      </div>

      <div className="console-grid">
        <div className="console-left">
          <p className="eyebrow">{COPY.verdict.exitIpLabel}</p>
          <p className={known ? "ip-value" : "ip-value is-pending"}>
            {known?.ip ?? COPY.verdict.exitIpUnknown}
          </p>
          <p className="ip-loc">
            {known ? location(COPY, known) : COPY.checks.O1.unknown}
          </p>
          <p className="ip-note">{COPY.verdict.exitIpNote}</p>

          <dl className="geo-grid">
            <div className="g-loc">
              <dt>{COPY.checks.O1.fields.location}</dt>
              <dd>{known ? location(COPY, known) : COPY.checks.O1.unknown}</dd>
            </div>
            <div className="g-tz">
              <dt>{COPY.checks.O1.fields.timezone}</dt>
              <dd>{known?.timezone ?? COPY.checks.O1.unknown}</dd>
            </div>
            <div className="g-colo">
              <dt>{COPY.checks.O1.fields.colo}</dt>
              <dd>{known?.colo ?? COPY.checks.O1.unknown}</dd>
            </div>
            <div className="g-asn">
              <dt>{COPY.checks.O1.fields.asn}</dt>
              <dd>
                {known
                  ? asnOf(known, COPY.checks.O1.unknown)
                  : COPY.checks.O1.unknown}
              </dd>
            </div>
          </dl>
        </div>

        <div className="console-right">
          <p className="eyebrow">{COPY.verdict.summaryLabel}</p>
          {/* 结论是个异步状态机，会从「暂无结论」一路走到「完整 · 高风险」——不播报的话
              屏幕阅读器用户看不到这个过程。atomic 让档位、阶段标注、摘要三者作为一句话
              整体播出，而不是碎成三次；播报只在这三者的文本真的变化时触发，不会因为
              父组件的其他 state（比如覆盖度数字跳动）而重复播报。 */}
          <div aria-live="polite" aria-atomic="true">
            <div className="level-row">
              <span className="level">
                <span className="level-dot" aria-hidden="true" />
                {verdict.stage === "insufficient"
                  ? COPY.verdict.insufficientLabel
                  : COPY.verdict.level[verdict.level]}
              </span>
              {/* 数据不足时不挂「初步 / 完整」标注：还没有结论，也就无所谓这个结论成色。 */}
              {verdict.stage === "insufficient" ? null : (
                <span className="stage-badge">
                  {verdict.stage === "preliminary"
                    ? COPY.verdict.preliminaryBadge
                    : COPY.verdict.fullBadge}
                </span>
              )}
            </div>
            <p className="summary">{summaryOf(COPY, verdict)}</p>
          </div>

          <CoverageSection copy={COPY} coverage={coverage} cells={cells} />
        </div>
      </div>
    </section>
  );
}
