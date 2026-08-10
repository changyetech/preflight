// 卡片壳与灰卡（规格 4.1 五态 / 4.2 每卡带「这意味着什么」）。

import { useState, type ReactNode } from "react";

import { COPY } from "../copy";

export type CardTone = "neutral" | "ok" | "warn" | "danger" | "muted";

type CardStatus = keyof typeof COPY.cardStatus;

export function CopyButton({ text }: { text: string }) {
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
  children,
}: {
  id: string;
  title: string;
  status: CardStatus;
  tone?: CardTone;
  meaning: string;
  /** 只有「检测失败」提供重试；灰卡（需 CLI）是终态，不传此项（规格 4.1）。 */
  onRetry?: () => void;
  children?: ReactNode;
}) {
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
          {COPY.actions.retry}
        </button>
      ) : null}
    </article>
  );
}

/** 仅 CLI 项的灰卡：终态、无重试入口，带一键复制安装命令（规格第 4 节）。 */
export function CliCard({
  id,
  title,
  meaning,
}: {
  id: string;
  title: string;
  meaning: string;
}) {
  return (
    <CheckCard
      id={id}
      title={title}
      status="needCli"
      tone="muted"
      meaning={meaning}
    >
      <p className="cli-hint">{COPY.cli.hint}</p>
      <p className="install">
        <code>{COPY.actions.installCommand}</code>
        <CopyButton text={COPY.actions.installCommand} />
      </p>
    </CheckCard>
  );
}
