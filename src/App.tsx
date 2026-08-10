// 单页：首屏结论区 + 9 张检测卡（规格第 4 节）。
// 落地内容与 i18n 不在本任务范围内（归 --content 子计划）。

import "./App.css";

import { CliCard } from "./components/Card";
import { O1Card, O2Card, O3Card, O4Card } from "./components/cards";
import { VerdictPanel } from "./components/Verdict";
import { COPY } from "./copy";
import { usePanel } from "./usePanel";

function App() {
  const { panel, coverage, verdict, runGeo, runIpv6, runRisk, failRisk } =
    usePanel();

  return (
    <div className="page">
      <header className="site-header">
        <h1>{COPY.site.title}</h1>
        <p>{COPY.site.tagline}</p>
      </header>

      <VerdictPanel geo={panel.o1} verdict={verdict} coverage={coverage} />

      {/* 灰卡穿插在语义相邻的可在线项旁边，而不是堆到末尾：
          用户看完「网页测到了什么」，紧接着就该看到「同一件事 CLI 还能多测什么」。 */}
      <section className="cards">
        <O1Card state={panel.o1} onRetry={() => void runGeo()} />
        <CliCard
          id="C1"
          title={COPY.checks.C1.title}
          meaning={COPY.checks.C1.meaning}
        />

        <O2Card state={panel.o2} onRetry={() => void runGeo()} />
        <CliCard
          id="C4"
          title={COPY.checks.C4.title}
          meaning={COPY.checks.C4.meaning}
        />

        <O3Card state={panel.o3} onRetry={() => void runIpv6()} />
        <CliCard
          id="C2"
          title={COPY.checks.C2.title}
          meaning={COPY.checks.C2.meaning}
        />

        <CliCard
          id="C3"
          title={COPY.checks.C3.title}
          meaning={COPY.checks.C3.meaning}
        />

        <O4Card
          state={panel.o4}
          onRun={(token) => void runRisk(token)}
          onFail={failRisk}
        />
        <CliCard
          id="C5"
          title={COPY.checks.C5.title}
          meaning={COPY.checks.C5.meaning}
        />
      </section>

      <footer className="site-footer">
        <p>{COPY.footer.privacy}</p>
        <p>{COPY.footer.thirdParty}</p>
      </footer>
    </div>
  );
}

export default App;
