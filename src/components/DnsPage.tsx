// /dns/ 与 /zh-hans/dns/ 的独立页面（spec §5）。
// 只渲染静态表，不做实测（浏览器发不出 DNS 查询，spec §5.2）。

import dnsData from "../../docs/dns-servers.json";
import { type Lang } from "../copy";
import { LangSwitch } from "./LangSwitch";
import { ThemeSwitch } from "./ThemeSwitch";
import { useCopy } from "../i18n";

type Variant = "standard" | "security" | "family" | "adblock";

type DnsServer = {
  ip: string;
  name: string;
  region: string;
  domestic: boolean;
  variant: Variant;
};

const SERVERS = (dnsData as { servers: DnsServer[] }).servers;

function variantLabel(
  variant: Variant,
  COPY: ReturnType<typeof useCopy>,
): string {
  return COPY.dns.variants[variant];
}

function DnsShell({ lang }: { lang: Lang }) {
  const COPY = useCopy();

  return (
    <>
      <header className="dns-header">
        <div className="dns-header-bar">
          <a href={lang === "en" ? "/" : "/zh-hans"} className="dns-brand">
            {COPY.nav.brand}
          </a>
          <div className="dns-header-actions">
            <ThemeSwitch />
            <LangSwitch lang={lang} pageSlug="/dns/" />
          </div>
        </div>
      </header>

      <main className="dns-main">
        <h1>{COPY.dns.heading}</h1>

        <div className="dns-table-wrap">
          <table className="dns-table">
            <thead>
              <tr>
                <th>{COPY.dns.tableHeaders.ip}</th>
                <th>{COPY.dns.tableHeaders.provider}</th>
                <th>{COPY.dns.tableHeaders.region}</th>
                <th>{COPY.dns.tableHeaders.domestic}</th>
                <th>{COPY.dns.tableHeaders.filter}</th>
              </tr>
            </thead>
            <tbody>
              {SERVERS.map((s) => (
                <tr key={s.ip}>
                  <td className="dns-ip">{s.ip}</td>
                  <td>{s.name}</td>
                  <td className="dns-region">{s.region}</td>
                  <td>{s.domestic ? COPY.dns.domesticYes : ""}</td>
                  <td>{variantLabel(s.variant, COPY)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <p className="dns-cli-hint">{COPY.dns.cliHint}</p>

        <a href={lang === "en" ? "/" : "/zh-hans"} className="dns-back-link">
          {"<"} {COPY.dns.backHome}
        </a>
      </main>
    </>
  );
}

export function DnsPage({ lang }: { lang: Lang }) {
  return <DnsShell lang={lang} />;
}
