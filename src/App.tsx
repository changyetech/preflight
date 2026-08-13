// 单页：首屏结论区 + 10 张检测卡 + 落地内容（规格第 4 节）。
// 语言由路径决定（`/en` 为英文，其余中文），不做 Accept-Language 自动跳转（规格第 7 节）。

import "./App.css";

import { CliListItem } from "./components/Card";
import {
  O1Card,
  O2Card,
  O3Card,
  O4Card,
  O5Card,
  O6Card,
} from "./components/cards";
import { coverageCells } from "./coverageMeter";
import { CLI_CHECK_IDS } from "./domain/checks";
import { langFromPathname, localeOf, type Lang } from "./copy";
import { BackToTop } from "./components/BackToTop";
import { Footer } from "./components/Footer";
import { LangSwitch } from "./components/LangSwitch";
import { Landing } from "./components/Landing";
import { ThemeSwitch } from "./components/ThemeSwitch";
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
      {/* 跳过导航：顶栏是 sticky 且带 5 个锚点，键盘用户每次进页面都要穿过它，
          平时移出视口，聚焦时落下（规格 §4 要点 6）。 */}
      <a className="skip" href="#main-content">
        {COPY.nav.skipToContent}
      </a>

      {/* 顶部 sticky 导航：品牌 + 各段锚点 + nav-tools（主题/语言）。锚点目标靠 CSS scroll-margin-top 避开吸顶条。 */}
      <nav className="nav">
        <div className="nav-in">
          {/* 品牌链回当前语种首页；停在首页时点它就是重新加载，符合 logo 的通行预期。 */}
          <a className="brand" href={localeOf(lang).path}>
            <img src="/favicon.svg" alt="" width="20" height="19" />
            {COPY.nav.brand}
          </a>
          <div className="nav-links">
            <a href="#checks">{COPY.nav.checks}</a>
            <a href="#cli-checks">{COPY.nav.cliChecks}</a>
            <a href="#why">{COPY.nav.why}</a>
            <a href="#install">{COPY.nav.install}</a>
            <a href="#compare">{COPY.nav.compare}</a>
          </div>
          <div className="nav-tools">
            <ThemeSwitch />
            <LangSwitch lang={lang} />
          </div>
        </div>
      </nav>

      <main className="page" id="main-content" tabIndex={-1}>
        <header className="masthead">
          <h1>{COPY.site.title}</h1>
          <p>{COPY.site.tagline}</p>
        </header>

        <VerdictPanel
          geo={panel.o1}
          verdict={verdict}
          coverage={coverage}
          cells={coverageCells(panel)}
        />

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
          {/* C1–C4 是终态名册，卡片壳（状态药丸、正文容器）是错的容器，改为发丝线列表
              （规格 §4 要点 6，删的只是卡片形态，四项内容不变）。 */}
          <ul className="cli-list">
            {CLI_CHECK_IDS.map((id) => (
              <CliListItem
                key={id}
                id={id}
                title={COPY.checks[id].title}
                meaning={COPY.checks[id].meaning}
              />
            ))}
          </ul>
        </section>

        <Landing />

        <Footer />
      </main>

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
