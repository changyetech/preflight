// /dns/ 与 /zh-hans/dns/ 的独立页面（spec §5）。
// 只渲染静态表，不做实测（浏览器发不出 DNS 查询，spec §5.2）。

import dnsData from "../../docs/dns-servers.json";
import { type Lang } from "../copy";
import { CopyButton } from "./Card";
import { Nav } from "./Nav";
import { useCopy } from "../i18n";

type Variant = "standard" | "security" | "family" | "adblock";

type DnsServer = {
  ip: string;
  name: string;
  region: string;
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
      <Nav lang={lang} pageSlug="/dns/" />

      <main className="dns-main">
        <h1>{COPY.dns.heading}</h1>

        <div className="dns-table-wrap">
          <table className="dns-table">
            <thead>
              <tr>
                <th>{COPY.dns.tableHeaders.ip}</th>
                <th>{COPY.dns.tableHeaders.provider}</th>
                <th>{COPY.dns.tableHeaders.region}</th>
                <th>{COPY.dns.tableHeaders.filter}</th>
              </tr>
            </thead>
            <tbody>
              {SERVERS.map((s) => (
                <tr key={s.ip}>
                  <td className="dns-ip">
                    <span>{s.ip}</span>
                    <CopyButton text={s.ip} />
                  </td>
                  <td>{s.name}</td>
                  <td className="dns-region">{s.region}</td>
                  <td>{variantLabel(s.variant, COPY)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <p className="dns-cli-hint">{COPY.dns.cliHint}</p>
      </main>
    </>
  );
}

export function DnsPage({ lang }: { lang: Lang }) {
  return <DnsShell lang={lang} />;
}
