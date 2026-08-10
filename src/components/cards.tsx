// 四张可在线检测卡（规格 2.1 ~ 2.4）。
//
// 状态到卡片的映射规则：
//   - 「检测失败」一律给重试入口，并原样展示失败原因；失败绝不呈现为「没查出问题」
//   - 分项颜色（tone）只影响这张卡，不参与综合结论（规格 3.2）

import { useRef, useState } from "react";

import { CheckCard, type CardTone } from "./Card";
import { COPY } from "../copy";
import type { OnlineCheck } from "../domain/checks";
import type {
  GeoData,
  Ipv6Result,
  RiskData,
  TimezoneResult,
} from "../domain/types";
import { requestTurnstileToken, turnstileConfigured } from "../turnstile";

function Field({ label, value }: { label: string; value: string }) {
  return (
    <p className="field">
      <span className="field-label">{label}</span>
      <span className="field-value">{value}</span>
    </p>
  );
}

function Failure({ reason }: { reason: string }) {
  return <p className="failure">{reason}</p>;
}

export function O1Card({
  state,
  onRetry,
}: {
  state: OnlineCheck<GeoData>;
  onRetry: () => void;
}) {
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
      {state.status === "failed" ? <Failure reason={state.reason} /> : null}
      {state.status === "done" ? (
        <>
          <Field
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
          />
          <Field
            label={copy.fields.asn}
            value={
              state.data.asOrganization
                ? `${state.data.asOrganization}${state.data.asn ? ` (AS${state.data.asn})` : ""}`
                : unknown
            }
          />
          <Field
            label={copy.fields.timezone}
            value={state.data.timezone ?? unknown}
          />
          <Field label={copy.fields.colo} value={state.data.colo ?? unknown} />
        </>
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
  const copy = COPY.checks.O2;
  const data = state.status === "done" ? state.data : null;
  const tone: CardTone =
    data?.match === false ? "warn" : data ? "ok" : "neutral";

  return (
    <CheckCard
      id="O2"
      title={copy.title}
      status={state.status}
      tone={tone}
      meaning={copy.meaning}
      onRetry={state.status === "failed" ? onRetry : undefined}
    >
      {state.status === "failed" ? <Failure reason={state.reason} /> : null}
      {data ? (
        <>
          <p className="conclusion">
            {data.match === null
              ? copy.unknown
              : data.match
                ? copy.match
                : copy.mismatch}
          </p>
          <Field label={copy.browserLabel} value={data.browserTimezone} />
          <Field
            label={copy.exitLabel}
            value={data.exitTimezone ?? COPY.checks.O1.unknown}
          />
        </>
      ) : null}
      {/* 验收标准 2：必须显式区分两个时区来源，否则 CLI 用户会误以为 $TZ 已被检查。 */}
      <p className="scope-note">{copy.scopeNote}</p>
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
    >
      {/* 失败态说的是「无法判定」，绝不能滑向「没有 IPv6」（验收标准 3）。 */}
      {state.status === "failed" ? <Failure reason={copy.failed} /> : null}
      {data ? (
        <>
          <p className="conclusion">{data.leak ? copy.leak : copy.disabled}</p>
          {data.leak ? (
            <Field label={copy.ipv6Label} value={data.ipv6} />
          ) : null}
        </>
      ) : null}
    </CheckCard>
  );
}

function RiskDetail({ data }: { data: Extract<RiskData, { status: "ok" }> }) {
  const copy = COPY.checks.O4;
  const detections = (["proxy", "vpn", "tor", "scraper"] as const).filter(
    (key) => data[key],
  );

  return (
    <>
      <Field
        label={copy.fields.networkType}
        value={
          data.networkType
            ? copy.networkType[data.networkType]
            : copy.networkType.unknown
        }
      />
      <Field label={copy.fields.riskScore} value={`${data.riskScore} / 100`} />
      <Field
        label={copy.fields.detections}
        value={
          detections.length > 0
            ? detections.map((key) => copy.detectionLabels[key]).join(" · ")
            : copy.noDetection
        }
      />
      <Field
        label={copy.fields.abuse}
        value={
          data.abuseListed === null
            ? copy.abuse.unknown
            : data.abuseListed
              ? copy.abuse.listed
              : copy.abuse.clean
        }
      />
      {data.networkType === "Hosting" || detections.length > 0 ? (
        <p className="scope-note">{copy.hostingNote}</p>
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
      {state.status === "idle" ? (
        <p className="conclusion">{copy.idle}</p>
      ) : null}
      {state.status === "failed" ? <Failure reason={state.reason} /> : null}
      {quotaExhausted ? <Failure reason={copy.quotaExhausted} /> : null}
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
          <p className="consent-note">{copy.consentNote}</p>
        </>
      ) : null}
      {/* 失败态的触发控件是 CheckCard 的重试按钮（文案已换成 consentButton），
          这里补上第二个第三方 StopForumSpam 的披露，与首次触发时看到的一致。 */}
      {state.status === "failed" ? (
        <p className="consent-note">{copy.consentNote}</p>
      ) : null}

      <div ref={turnstileRef} className="turnstile" />
    </CheckCard>
  );
}
