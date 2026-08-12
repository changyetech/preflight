// 右上角语言菜单：纯路径链接，不写 localStorage / cookie，不做 Accept-Language 自动跳转（规格第 7 节）。
//
// 用原生 <details> 而非自写下拉：无 JS 即可展开、Esc 与键盘导航由浏览器负责，
// 也不必为「点外部关闭」引入全局监听——语言切换是低频操作，多点一下收起完全可接受。

import { LOCALES, localeOf, type Lang } from "../copy";
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
  const current = localeOf(lang);

  return (
    <details className="lang-menu">
      <summary className="lang-switch" aria-label={COPY.nav.language}>
        <GlobeIcon />
        {current.label}
      </summary>

      <ul className="lang-menu-list">
        {LOCALES.map((locale) => (
          <li key={locale.id}>
            {/* 各语言用自己的写法自称，故 hrefLang + lang 都标目标语言，
                屏幕阅读器才会用对应语音读它，而不是按当前页语言硬读。 */}
            <a
              href={locale.path}
              hrefLang={locale.htmlLang}
              lang={locale.htmlLang}
              aria-current={locale.id === lang ? "true" : undefined}
            >
              {locale.label}
            </a>
          </li>
        ))}
      </ul>
    </details>
  );
}
