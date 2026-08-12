// 单页：首屏结论区 + 9 张检测卡 + 落地内容（规格第 4 节）。
// 语言由路径决定（`/en` 为英文，其余中文），不做 Accept-Language 自动跳转（规格第 7 节）。

import "./App.css";

import { CliCard } from "./components/Card";
import {
  O1Card,
  O2Card,
  O3Card,
  O4Card,
  O5Card,
  O6Card,
} from "./components/cards";
import { CLI_CHECK_IDS } from "./domain/checks";
import { langFromPathname, localeOf, type Lang } from "./copy";
import { BackToTop } from "./components/BackToTop";
import { LangSwitch } from "./components/LangSwitch";
import { Landing } from "./components/Landing";
import { VerdictPanel } from "./components/Verdict";
import { CopyProvider, useCopy } from "./i18n";
import { usePanel } from "./usePanel";

function AppShell({ lang }: { lang: Lang }) {
  const COPY = useCopy();
  const {
    panel,
    coverage,
    verdict,
    runGeo,
    runIpv6,
    runRisk,
    failRisk,
    runDnsEgress,
    runUdpEgress,
  } = usePanel();

  return (
    <>
      {/* 顶部 sticky 导航：品牌 + 各段锚点 + 语言切换。锚点目标靠 CSS scroll-margin-top 避开吸顶条。 */}
      <nav className="site-nav">
        <div className="site-nav-inner">
          {/* 品牌链回当前语种首页；停在首页时点它就是重新加载，符合 logo 的通行预期。 */}
          <a className="site-nav-brand" href={localeOf(lang).path}>
            <img src="/favicon.svg" alt="" width="20" height="19" />
            {COPY.nav.brand}
          </a>
          <div className="site-nav-links">
            <a href="#checks">{COPY.nav.checks}</a>
            <a href="#cli-checks">{COPY.nav.cliChecks}</a>
            <a href="#why">{COPY.nav.why}</a>
            <a href="#install">{COPY.nav.install}</a>
            <a href="#compare">{COPY.nav.compare}</a>
          </div>
          <LangSwitch lang={lang} />
        </div>
      </nav>

      <div className="page">
        <header className="site-header">
          <h1>{COPY.site.title}</h1>
          <p>{COPY.site.tagline}</p>
        </header>

        <VerdictPanel geo={panel.o1} verdict={verdict} coverage={coverage} />

        {/* 两个分区而非穿插（规格第 4 节第 2 项）：先「网页测到了什么」，再「哪些只有 CLI 能测」。 */}
        <section className="checks-group" id="checks">
          <header className="checks-group-head">
            <h2>{COPY.sections.online.title}</h2>
            <p>{COPY.sections.online.body}</p>
          </header>
          <div className="cards">
            <O1Card state={panel.o1} onRetry={() => void runGeo()} />
            <O2Card state={panel.o2} onRetry={() => void runGeo()} />
            <O3Card state={panel.o3} onRetry={() => void runIpv6()} />
            <O4Card
              state={panel.o4}
              onRun={(token) => void runRisk(token)}
              onFail={failRisk}
            />
            <O5Card state={panel.o5} onRetry={() => void runDnsEgress()} />
            <O6Card state={panel.o6} onRetry={() => void runUdpEgress()} />
          </div>
        </section>

        <section className="checks-group" id="cli-checks">
          <header className="checks-group-head">
            <h2>{COPY.sections.cli.title}</h2>
            <p>{COPY.sections.cli.body}</p>
          </header>
          <div className="cards">
            {CLI_CHECK_IDS.map((id) => (
              <CliCard
                key={id}
                id={id}
                title={COPY.checks[id].title}
                meaning={COPY.checks[id].meaning}
              />
            ))}
          </div>
        </section>

        <Landing />

        <footer className="site-footer">
          <p>{COPY.footer.privacy}</p>
          <p>{COPY.footer.thirdParty}</p>
        </footer>
      </div>

      <BackToTop />
    </>
  );
}

function App() {
  const lang = langFromPathname(window.location.pathname);

  return (
    <CopyProvider lang={lang}>
      <AppShell lang={lang} />
    </CopyProvider>
  );
}

export default App;
