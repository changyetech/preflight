// 语言 Provider：按路径决定的 lang 注入对应文案（context 与 hook 在 i18n.ts）。

import type { ReactNode } from "react";

import { getCopy, type Lang } from "./copy";
import { CopyContext } from "./i18n";

export function CopyProvider({
  lang,
  children,
}: {
  lang: Lang;
  children?: ReactNode;
}) {
  return (
    <CopyContext.Provider value={getCopy(lang)}>
      {children}
    </CopyContext.Provider>
  );
}
