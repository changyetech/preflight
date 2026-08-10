// 语言上下文：把 COPY 从静态导入换成运行时可切换的资源（规格第 7 节）。
// 不写 localStorage / cookie——语言完全由路径决定，不持久化任何用户偏好（CLAUDE.md 隐私约束）。

import { createContext, useContext, type ReactNode } from "react";

import { COPY, getCopy, type Copy, type Lang } from "./copy";

const CopyContext = createContext<Copy>(COPY);

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

export function useCopy(): Copy {
  return useContext(CopyContext);
}

/** `/en` 是唯一的英文入口，其余路径一律中文——不做 Accept-Language 自动跳转（规格第 7 节）。 */
export function langFromPathname(pathname: string): Lang {
  return pathname === "/en" || pathname.startsWith("/en/") ? "en" : "zh";
}
