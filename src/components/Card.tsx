// 卡片壳与灰卡（规格 4.1 五态 / 4.2 每卡带「这意味着什么」）。

import { useState, type ReactNode } from "react";

import type { Copy } from "../copy";
import { CLI_CHECK } from "../domain/checks";
import { useCopy } from "../i18n";

export type CardTone = "neutral" | "ok" | "warn" | "danger" | "muted";

type CardStatus = keyof Copy["cardStatus"];

export function CopyButton({ text }: { text: string }) {
  const COPY = useCopy();
  const [copied, setCopied] = useState(false);

  return (
    <button
      type="button"
      className="copy"
      onClick={() => {
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
  /** 只有「检测失败」提供重试；灰卡（需 CLI）是终态，不传此项（规格 4.1）。 */
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
    <article className={`card tone-${tone}`}>
      <header>
        <span className="card-id">{id}</span>
        <h3>{title}</h3>
        <span className={`badge badge-${status}`}>
          {COPY.cardStatus[status]}
        </span>
      </header>

      {children ? <div className="card-body">{children}</div> : null}

      <p className="meaning">
        <span className="meaning-label">{COPY.actions.meaningLabel}</span>
        {meaning}
      </p>

      {onRetry ? (
        <button type="button" className="retry" onClick={onRetry}>
          {label}
        </button>
      ) : null}
    </article>
  );
}

/** 仅 CLI 项的灰卡：终态、无重试入口。安装命令只在落地内容「安装 CLI」段出现一次（规格第 4 节）。 */
export function CliCard({
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
    <CheckCard
      id={id}
      title={title}
      status={CLI_CHECK.status}
      tone="muted"
      meaning={meaning}
    >
      <p className="cli-hint">{COPY.cli.hint}</p>
    </CheckCard>
  );
}
