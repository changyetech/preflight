// 六张可在线检测卡（规格 2.1 ~ 2.4，O5／O6 见判级契约 §2.5／§2.6）。
//
// 状态到卡片的映射规则：
//   - 「检测失败」一律给重试入口，并原样展示失败原因；失败绝不呈现为「没查出问题」
//   - 分项颜色（tone）只影响这张卡，不参与综合结论（规格 3.2）
//
// 正文顺序统一为 kv → result → note（原型 refs/ipcheck-web-redesign.html 的 landO1~landO6 范式）：
// 字段列表在前、着色结论居中、不参与判定的补充说明收在最后。

import { useRef, useState } from "react";

import { CheckCard, Kv, KvRow, Note, Result, type CardTone } from "./Card";
import { useCopy } from "../i18n";
import type { OnlineCheck } from "../domain/checks";
import { UDP_EGRESS_WEBRTC_UNAVAILABLE } from "../domain/udpEgress";
import type {
  DnsEgressResult,
  GeoData,
  Ipv6Result,
  RiskData,
  TimezoneResult,
  UdpEgressResult,
} from "../domain/types";
import { requestTurnstileToken, turnstileConfigured } from "../turnstile";

export function O1Card({
  state,
  onRetry,
}: {
  state: OnlineCheck<GeoData>;
  onRetry: () => void;
}) {
  const COPY = useCopy();
  const copy = COPY.checks.O1;
  const unknown = copy.unknown;

  return (
    <CheckCard
      id="O1"
      title={copy.title}
      status={state.status}
      meaning={copy.meaning}
      onRetry={state.status === "failed" ? onRetry : undefined}
    >
      {state.status === "failed" ? (
        <Result tone="danger">{state.reason}</Result>
      ) : null}
      {state.status === "done" ? (
        <Kv>
          <KvRow
            label={copy.fields.location}
            value={
              // 城市与地区常常同名（如 Osaka · Osaka），去重免得读起来像故障。
              [
                ...new Set(
                  [
                    state.data.city,
                    state.data.region,
                    state.data.country,
                  ].filter(Boolean),
                ),
              ].join(" · ") || unknown
            }
            emphasis
          />
          <KvRow
            label={copy.fields.asn}
            value={
              state.data.asOrganization
                ? `${state.data.asOrganization}${state.data.asn ? ` (AS${state.data.asn})` : ""}`
                : unknown
            }
          />
          <KvRow
            label={copy.fields.timezone}
            value={state.data.timezone ?? unknown}
          />
          <KvRow label={copy.fields.colo} value={state.data.colo ?? unknown} />
        </Kv>
      ) : null}
    </CheckCard>
  );
}

export function O2Card({
  state,
  onRetry,
}: {
  state: OnlineCheck<TimezoneResult>;
  onRetry: () => void;
}) {
  const COPY = useCopy();
  const copy = COPY.checks.O2;
  const data = state.status === "done" ? state.data : null;
  // match === null 是「无从比对」（边缘未给出出口 IP 时区），既非一致也非不一致——
  // 中性色，不得落到绿色（review 修复：Result 把这句话包进实心 t-ok 底色框后，
  // 原先 `data ? "ok" : "neutral"` 的写法会让「无从比对」被渲染成显著的绿色结论）。
  const tone: CardTone =
    data?.match === false ? "warn" : data?.match === true ? "ok" : "neutral";

  return (
    <CheckCard
      id="O2"
      title={copy.title}
      status={state.status}
      tone={tone}
      meaning={copy.meaning}
      onRetry={state.status === "failed" ? onRetry : undefined}
    >
      {state.status === "failed" ? (
        <Result tone="danger">{state.reason}</Result>
      ) : null}
      {data ? (
        <>
          <Kv>
            <KvRow
              label={copy.browserLabel}
              value={data.browserTimezone}
              emphasis
            />
            <KvRow
              label={copy.exitLabel}
              value={data.exitTimezone ?? COPY.checks.O1.unknown}
              emphasis
            />
          </Kv>
          <Result tone={tone}>
            {data.match === null
              ? copy.unknown
              : data.match
                ? copy.match
                : copy.mismatch}
          </Result>
        </>
      ) : null}
      {/* 验收标准 2：必须显式区分两个时区来源，否则 CLI 用户会误以为 $TZ 已被检查。 */}
      <Note>{copy.scopeNote}</Note>
    </CheckCard>
  );
}

export function O3Card({
  state,
  onRetry,
}: {
  state: OnlineCheck<Ipv6Result>;
  onRetry: () => void;
}) {
  const COPY = useCopy();
  const copy = COPY.checks.O3;
  const data = state.status === "done" ? state.data : null;
  const tone: CardTone = data?.leak ? "warn" : data ? "ok" : "neutral";

  return (
    <CheckCard
      id="O3"
      title={copy.title}
      status={state.status}
      tone={tone}
      meaning={copy.meaning}
      onRetry={state.status === "failed" ? onRetry : undefined}
      // O3 自动执行、没有触发控件可挂披露；重试就是唯一的触发控件，文案必须写明第三方
      // （终审修复波：之前落回了通用「重试」，与 O4 的 consentButton 处理不一致）。
      retryLabel={copy.retryLabel}
    >
      {/* 失败态说的是「无法判定」，绝不能滑向「没有 IPv6」（验收标准 3）。 */}
      {state.status === "failed" ? (
        <Result tone="danger">{copy.failed}</Result>
      ) : null}
      {data ? (
        <>
          {data.leak ? (
            <Kv>
              <KvRow label={copy.ipv6Label} value={data.ipv6} emphasis />
            </Kv>
          ) : null}
          <Result tone={tone}>{data.leak ? copy.leak : copy.disabled}</Result>
        </>
      ) : null}
      {/* 无控件的自动检测项，披露只能就地放在说明位（终审修复波：ipify 无就地披露）。 */}
      <Note>{copy.thirdPartyNote}</Note>
    </CheckCard>
  );
}

function RiskDetail({ data }: { data: Extract<RiskData, { status: "ok" }> }) {
  const COPY = useCopy();
  const copy = COPY.checks.O4;
  const detections = (["proxy", "vpn", "tor", "scraper"] as const).filter(
    (key) => data[key],
  );

  return (
    <>
      <Kv>
        <KvRow
          label={copy.fields.networkType}
          value={
            data.networkType
              ? copy.networkType[data.networkType]
              : copy.networkType.unknown
          }
          emphasis
        />
        <KvRow
          label={copy.fields.riskScore}
          value={`${data.riskScore} / 100`}
          emphasis
        />
        <KvRow
          label={copy.fields.detections}
          value={
            detections.length > 0
              ? detections.map((key) => copy.detectionLabels[key]).join(" · ")
              : copy.noDetection
          }
        />
        <KvRow
          label={copy.fields.abuse}
          value={
            data.abuseListed === null
              ? copy.abuse.unknown
              : data.abuseListed
                ? copy.abuse.listed
                : copy.abuse.clean
          }
        />
      </Kv>
      {data.networkType === "Hosting" || detections.length > 0 ? (
        <Note>{copy.hostingNote}</Note>
      ) : null}
    </>
  );
}

export function O4Card({
  state,
  onRun,
  onFail,
}: {
  state: OnlineCheck<RiskData>;
  onRun: (token: string) => void;
  onFail: (reason: string) => void;
}) {
  const COPY = useCopy();
  const copy = COPY.checks.O4;
  const turnstileRef = useRef<HTMLDivElement>(null);
  const [verifying, setVerifying] = useState(false);

  const data = state.status === "done" ? state.data : null;
  const ok = data?.status === "ok" ? data : null;
  // 分项颜色直接吃契约给的 riskLevel（docs/api.md 3.1 已按规格 3.2 的阈值分好级），
  // 不在前端拿 riskScore 再算一遍——同一套阈值放两处迟早会分叉。
  // 唯一的本地叠加：Hosting 与代理检出把绿拉成黄（规格 3.2 的分项提醒），但不拉高综合结论。
  const tone: CardTone = ok
    ? ok.riskLevel === "high"
      ? "danger"
      : ok.riskLevel === "medium" ||
          ok.networkType === "Hosting" ||
          ok.proxy ||
          ok.vpn ||
          ok.tor ||
          ok.scraper
        ? "warn"
        : "ok"
    : "neutral";

  // 配额耗尽是容量状态而非故障：卡片按「检测失败」呈现，但不给重试——今天重试多少次都一样。
  const quotaExhausted = data?.status === "quotaExhausted";
  const status = quotaExhausted ? "failed" : state.status;

  async function trigger() {
    if (!turnstileConfigured) {
      onFail(copy.turnstileMissing);
      return;
    }

    setVerifying(true);
    try {
      const container = turnstileRef.current;
      if (!container) throw new Error(COPY.errors.humanVerification);
      onRun(await requestTurnstileToken(container));
    } catch {
      onFail(COPY.errors.humanVerification);
    } finally {
      setVerifying(false);
    }
  }

  return (
    <CheckCard
      id="O4"
      title={copy.title}
      status={status}
      tone={tone}
      meaning={copy.meaning}
      onRetry={state.status === "failed" ? () => void trigger() : undefined}
      // 重试同样会把出口 IP 发往 proxycheck.io，披露必须跟着这个按钮走（ADR-0008）。
      retryLabel={copy.consentButton}
    >
      {state.status === "idle" ? <Note>{copy.idle}</Note> : null}
      {state.status === "failed" ? (
        <Result tone="danger">{state.reason}</Result>
      ) : null}
      {quotaExhausted ? (
        <Result tone="danger">{copy.quotaExhausted}</Result>
      ) : null}
      {ok ? <RiskDetail data={ok} /> : null}

      {state.status === "idle" ? (
        <>
          {/* ADR-0008：第三方调用写在触发它的控件上，点击即知情同意。 */}
          <button
            type="button"
            className="consent"
            disabled={verifying}
            onClick={() => void trigger()}
          >
            {copy.consentButton}
          </button>
          <Note>{copy.consentNote}</Note>
        </>
      ) : null}
      {/* 失败态的触发控件是 CheckCard 的重试按钮（文案已换成 consentButton），
          这里补上第二个第三方 StopForumSpam 的披露，与首次触发时看到的一致。 */}
      {state.status === "failed" ? <Note>{copy.consentNote}</Note> : null}

      <div ref={turnstileRef} className="turnstile" />
    </CheckCard>
  );
}

export function O5Card({
  state,
  onRetry,
}: {
  state: OnlineCheck<DnsEgressResult>;
  onRetry: () => void;
}) {
  const COPY = useCopy();
  const copy = COPY.checks.O5;
  const data = state.status === "done" ? state.data : null;
  const comparison = data?.comparison ?? null;
  // 「无从比对」不是绿色的「未泄露」，也不是失败——中性色，避免读成任何一种结论（契约 §2.5）。
  const tone: CardTone =
    comparison?.comparable === true
      ? comparison.leak
        ? "warn"
        : "ok"
      : "neutral";

  return (
    <CheckCard
      id="O5"
      title={copy.title}
      status={state.status}
      tone={tone}
      meaning={copy.meaning}
      onRetry={state.status === "failed" ? onRetry : undefined}
      retryLabel={copy.retryLabel}
    >
      {/* 探测本身失败，唯一失败原因，不必按 reason 分支（契约 §2.5 判定表最后一行）。 */}
      {state.status === "failed" ? (
        <Result tone="danger">{copy.failed}</Result>
      ) : null}
      {/* ECS／出口国与 resolver 归属同属「字段列表」，收进同一个 kv（原型 landO5）；
          resolver 归属不参与判定，说明紧跟在字段后面（契约 §2.1／§2.5 硬约束 1）。 */}
      {data ? (
        <Kv>
          {comparison?.comparable ? (
            <>
              <KvRow
                label={copy.ecsLabel}
                value={comparison.ecsCountry}
                emphasis
              />
              <KvRow
                label={copy.exitLabel}
                value={comparison.exitCountry}
                emphasis
              />
            </>
          ) : null}
          <KvRow
            label={copy.resolverLabel}
            value={data.resolverGeo ?? COPY.checks.O1.unknown}
          />
        </Kv>
      ) : null}
      {comparison ? (
        <Result tone={tone}>
          {comparison.comparable
            ? comparison.leak
              ? copy.leak
              : copy.noLeak
            : comparison.reason === "noEcs"
              ? copy.noEcs
              : comparison.reason === "unmappedCountry"
                ? copy.unmappedCountry
                : copy.unknownExitCountry}
        </Result>
      ) : null}
      {data ? <Note>{copy.resolverNote}</Note> : null}
      {/* 无控件的自动检测项，第三方披露就地放在说明位（与 O3 同一套处理）。
          thirdPartyNote／scopeNote 恒渲染，不随状态隐藏——O5 的降级代理说明必须始终在位。 */}
      <Note>{copy.thirdPartyNote}</Note>
      <Note>{copy.scopeNote}</Note>
    </CheckCard>
  );
}

export function O6Card({
  state,
  onRetry,
}: {
  state: OnlineCheck<UdpEgressResult>;
  onRetry: () => void;
}) {
  const COPY = useCopy();
  const copy = COPY.checks.O6;
  const data = state.status === "done" ? state.data : null;
  // 同 O5：无从比对是中性色，绝不落到绿色——WebRTC 被禁用（检测失败）同样不得渲染为绿色。
  const tone: CardTone =
    data?.comparable === true ? (data.mismatch ? "warn" : "ok") : "neutral";

  return (
    <CheckCard
      id="O6"
      title={copy.title}
      status={state.status}
      tone={tone}
      meaning={copy.meaning}
      onRetry={state.status === "failed" ? onRetry : undefined}
      retryLabel={copy.retryLabel}
    >
      {/* 两个失败原因文案不同：浏览器不允许 vs 探测超时，刷新的可恢复性相反（契约 §5.6）。 */}
      {state.status === "failed" ? (
        <Result tone="danger">
          {state.reason === UDP_EGRESS_WEBRTC_UNAVAILABLE
            ? copy.webrtcUnavailable
            : copy.stunUnanswered}
        </Result>
      ) : null}
      {data ? (
        <>
          {data.comparable ? (
            <Kv>
              <KvRow
                label={copy.reflexiveLabel}
                value={data.reflexiveIp}
                emphasis
              />
              <KvRow label={copy.exitLabel} value={data.exitIp} emphasis />
            </Kv>
          ) : null}
          <Result tone={tone}>
            {data.comparable
              ? data.mismatch
                ? copy.mismatch
                : copy.noMismatch
              : data.reason === "familyMismatch"
                ? copy.familyMismatch
                : data.reason === "unknownExitIp"
                  ? copy.unknownExitIp
                  : copy.stunDisagree}
          </Result>
        </>
      ) : null}
      <Note>{copy.thirdPartyNote}</Note>
    </CheckCard>
  );
}
