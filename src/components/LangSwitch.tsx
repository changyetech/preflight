// 右上角手动语言切换：纯路径链接，不写 localStorage / cookie，不做 Accept-Language 自动跳转（规格第 7 节）。

import { useCopy } from "../i18n";

export function LangSwitch({ lang }: { lang: "zh" | "en" }) {
  const COPY = useCopy();
  const href = lang === "en" ? "/" : "/en";

  return (
    <a className="lang-switch" href={href}>
      {COPY.nav.switchTo}
    </a>
  );
}
