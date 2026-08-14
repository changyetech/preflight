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
        <a className="brand" href={home}>
          <img src="/favicon.svg" alt="" width="20" height="19" />
          {COPY.nav.brand}
        </a>
        <div className="nav-links">
          <a href={`${anchorBase}#checks`}>{COPY.nav.checks}</a>
          <a href={`${anchorBase}#cli-checks`}>{COPY.nav.cliChecks}</a>
          <a href={`${anchorBase}#why`}>{COPY.nav.why}</a>
          <a href={`${anchorBase}#install`}>{COPY.nav.install}</a>
          <a href={`${anchorBase}#compare`}>{COPY.nav.compare}</a>
        </div>
        <div className="nav-tools">
          <ThemeSwitch />
          <LangSwitch lang={lang} pageSlug={pageSlug} />
        </div>
      </div>
    </nav>
  );
}
