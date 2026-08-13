// 右上角语言切换：单键直切（规格 §2 决策 8），偏离原型的五语种下拉——原型按 5 语种
// 设计，语种收缩到 en/zh-hans 两个后下拉的前提不再成立，改为一个真链接直接跳到另一语种。
//
// 必须是真 <a href> 而非 onClick + location.assign：站点是多页构建、非 SPA，语言由路径
// 决定，跳转本就是整页导航；真链接才能中键新标签打开、右键复制链接、被爬虫与读屏正确理解。
// 不写 localStorage / cookie / sessionStorage，不记住上次选择（ADR-0008，语言不在主题的
// 持久化例外之列）。

import { otherLocale, type Lang } from "../copy";
import { useCopy } from "../i18n";

/** 地球图标。内联 SVG 而非图标字体：站点资源全自包含，且不为一个图标多一次请求。 */
function GlobeIcon() {
  return (
    <svg
      className="lang-switch-icon"
      viewBox="0 0 24 24"
      width="14"
      height="14"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="9" />
      <path d="M3 12h18" />
      <path d="M12 3a14 14 0 0 1 0 18a14 14 0 0 1 0-18" />
    </svg>
  );
}

export function LangSwitch({ lang }: { lang: Lang }) {
  const COPY = useCopy();
  // 只有两个语种：另一个即切换目标，从 LOCALES 派生，不在组件里硬编码路径。
  const other = otherLocale(lang);

  return (
    <a
      className="lang-switch"
      href={other.path}
      // 目标语种用自己的写法自称，故 hrefLang + lang 都标目标语言，读屏才会用
      // 对应语音读它，而不是按当前页语言硬读。
      hrefLang={other.htmlLang}
      lang={other.htmlLang}
      aria-label={`${COPY.nav.switchLanguageTo} ${other.label}`}
    >
      <GlobeIcon />
      {other.label}
    </a>
  );
}
