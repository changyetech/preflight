// 顶部 sticky 导航：首页与 /dns/ 共用同一套结构与样式。
// 锚点指向首页各分区：在首页用纯片段（原地平滑滚动），在子页带上语种首页路径（整页跳转后落到锚点）。

import { pageUrl, type Lang } from "../copy";
import { LangSwitch } from "./LangSwitch";
import { ThemeSwitch } from "./ThemeSwitch";
import { useCopy } from "../i18n";

export function Nav({
  lang,
  pageSlug = "/",
}: {
  lang: Lang;
  pageSlug?: string;
}) {
  const COPY = useCopy();
  const home = pageUrl(lang, "/");
  const anchorBase = pageSlug === "/" ? "" : home;

  return (
    <nav className="nav">
      <div className="nav-in">
        {/* 品牌链回当前语种首页；停在首页时点它就是重新加载，符合 logo 的通行预期。 */}
        {/* 内联 SVG 而非 <img src="/favicon.svg">：颜色要跟站点 data-theme 主题走，
            <img> 里的 favicon 只认系统 prefers-color-scheme。几何为 logo 简化层，
            规格见 docs/specs/2026-08-14-logo-redesign.md。 */}
        <a className="brand" href={home}>
          <svg viewBox="0 0 64 64" width="20" height="20" aria-hidden="true">
            <g fill="none" strokeLinecap="round" strokeLinejoin="round">
              <path
                stroke="var(--accent)"
                opacity=".4"
                strokeWidth="4.5"
                d="M7 47a27 27 0 0 1 50 0"
              />
              <path stroke="var(--accent)" strokeWidth="9" d="M15 38l10 10" />
              <path stroke="var(--accent)" strokeWidth="9" d="M25 48L52 15" />
              <path stroke="var(--ok)" strokeWidth="9" d="M47 21.2L52 15" />
            </g>
          </svg>
          {COPY.nav.brand}
        </a>
        <div className="nav-links">
          <a href={`${anchorBase}#checks`}>{COPY.nav.checks}</a>
          <a href={`${anchorBase}#cli-checks`}>{COPY.nav.cliChecks}</a>
          <a href={`${anchorBase}#why`}>{COPY.nav.why}</a>
          <a href={`${anchorBase}#install`}>{COPY.nav.install}</a>
          <a href={`${anchorBase}#compare`}>{COPY.nav.compare}</a>
        </div>
        {/* DNS 清单与 CLI 手册是顶栏里仅有的跨页链接，故意不放进 .nav-links：
            那一组在 900px 以下整组隐藏——锚点靠垂直滚动就能到达，跨页链接不能，
            隐藏它等于在窄屏彻底断掉这两个页面的入口（Web 面板没有 DNS 检测卡，
            C2 是 CLI-only；手册页在站内的另一个入口只有首页安装区块）。 */}
        {/* 新标签打开：顶栏在检测面板上方常驻，同标签跳走会打断用户正在进行的检测。
            带 rel="noopener"——即便同源也照挂，避免新页面拿到 window.opener。 */}
        <div className="nav-pages">
          <a
            className="nav-dns"
            href={pageUrl(lang, "/dns/")}
            target="_blank"
            rel="noopener"
            aria-current={pageSlug === "/dns/" ? "page" : undefined}
          >
            {COPY.nav.dns}
          </a>
          <a
            className="nav-guide"
            href={pageUrl(lang, "/guide/")}
            target="_blank"
            rel="noopener"
            aria-current={pageSlug === "/guide/" ? "page" : undefined}
          >
            {COPY.nav.guide}
          </a>
        </div>
        <div className="nav-tools">
          <ThemeSwitch />
          <LangSwitch lang={lang} pageSlug={pageSlug} />
        </div>
      </div>
    </nav>
  );
}
