// 卡片壳（规格 4.1 五态 / 4.2 每卡带「这意味着什么」）与 kv/result/note 三个共享呈现原语——
// 原型（refs/ipcheck-web-redesign.html）把每张卡的正文统一收成这三类结构：
// 字段列表（.kv）、着色结论（.result）、不参与判定的补充说明（.note）。

import { useState, type ReactNode } from "react";

import type { Copy } from "../copy";
import { useCopy } from "../i18n";

export type CardTone = "neutral" | "ok" | "warn" | "danger";

type CardStatus = keyof Copy["cardStatus"];

export function CopyButton({ text }: { text: string }) {
  const COPY = useCopy();
  const [copied, setCopied] = useState(false);

  return (
    <button
      type="button"
      className="copy"
      onClick={() => {
        // 非安全上下文（http）或旧浏览器没有 navigator.clipboard——降级为「什么都不做」，
        // 不抛错、不留下点了没反应却报错的按钮（brief 要点 2）。
        if (!navigator.clipboard) return;
        void navigator.clipboard.writeText(text).then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 1500);
        });
      }}
    >
      {copied ? COPY.actions.copied : COPY.actions.copy}
    </button>
  );
}

/** 一组 key/value 字段（原型 .kv：dl/dt/dd），取代逐条独立 <p> 的旧 Field。 */
export function Kv({ children }: { children: ReactNode }) {
  return <dl className="kv">{children}</dl>;
}

export function KvRow({
  label,
  value,
  emphasis = false,
}: {
  label: string;
  value: string;
  /** 该行是否是这张卡最主要的值（原型 kv() 第三个可选参数 "em"），比如出口 IP 的归属地。 */
  emphasis?: boolean;
}) {
  return (
    <div>
      <dt>{label}</dt>
      <dd className={emphasis ? "em" : undefined}>{value}</dd>
    </div>
  );
}

/** 卡片的核心结论/失败原因（原型 .result，t-ok/t-warn/t-danger 三档着色）。
 *  "neutral" 用于「无从比对」这类既非成功也非失败的中性结论，不带颜色后缀。 */
export function Result({
  tone = "neutral",
  children,
}: {
  tone?: CardTone;
  children: ReactNode;
}) {
  return (
    <p className={tone === "neutral" ? "result" : `result t-${tone}`}>
      {children}
    </p>
  );
}

/** 不参与判定的补充说明（原型 .note）：第三方披露、范围限定、resolver 归属之类。 */
export function Note({ children }: { children: ReactNode }) {
  return <p className="note">{children}</p>;
}

export function CheckCard({
  id,
  title,
  status,
  tone = "neutral",
  meaning,
  onRetry,
  retryLabel,
  children,
}: {
  id: string;
  title: string;
  status: CardStatus;
  tone?: CardTone;
  meaning: string;
  /** 只有「检测失败」提供重试（规格 4.1）。 */
  onRetry?: () => void;
  /**
   * 重试按钮文案。会触发第三方调用的项必须在这里写明调用对象（ADR-0008）——
   * 重试就是那个触发控件，光写「重试」等于把披露藏了起来。
   */
  retryLabel?: string;
  children?: ReactNode;
}) {
  const COPY = useCopy();
  const label = retryLabel ?? COPY.actions.retry;

  return (
    // aria-busy 而非 role=status：多张卡片近乎同时变化，逐张播报会刷屏；
    // busy 只告诉辅助技术「这块还在变」，播报交给结论区的 live 区（W2）。
    <article className={`card tone-${tone}`} aria-busy={status === "running"}>
      <header className="card-head">
        <span className="card-id">{id}</span>
        <h3>{title}</h3>
        <span className={`pill pill-${status}`}>{COPY.cardStatus[status]}</span>
      </header>

      {children ? <div className="card-body">{children}</div> : null}

      {/* 默认折叠：结论与字段优先，解释文案点开即读。文案始终在 DOM 内，爬虫与读屏都拿得到。 */}
      <details className="meaning">
        <summary className="meaning-label">{COPY.actions.meaningLabel}</summary>
        {meaning}
      </details>

      {onRetry ? (
        <button type="button" className="retry" onClick={onRetry}>
          {label}
        </button>
      ) : null}
    </article>
  );
}

/**
 * C1–C4 发丝线列表的单行（原型 .cli-list li）：这四项没有运行时状态、没有字段、没有重试——
 * 永远是同一份终态名册，卡片壳（状态药丸、正文容器）是错的容器，改为一条 id + 标题 + 折叠说明
 * 的发丝线（规格 §4 要点 6）。检测功能集不变，删的只是卡片形态。
 */
export function CliListItem({
  id,
  title,
  meaning,
}: {
  id: string;
  title: string;
  meaning: string;
}) {
  const COPY = useCopy();

  return (
    <li>
      <span className="card-id">{id}</span>
      <div>
        <h3>{title}</h3>
        <details className="meaning">
          <summary className="meaning-label">
            {COPY.actions.meaningLabel}
          </summary>
          {meaning}
        </details>
      </div>
    </li>
  );
}
