// 语言上下文：把 COPY 从静态导入换成运行时可切换的资源（规格第 7 节）。
// 不写 localStorage / cookie——语言完全由路径决定，不持久化任何用户偏好（CLAUDE.md 隐私约束）。
// Provider 组件在 CopyProvider.tsx——组件与 hook 分文件，Fast Refresh 才能生效。

import { createContext, useContext } from "react";

import { COPY, type Copy } from "./copy";

export const CopyContext = createContext<Copy>(COPY);

export function useCopy(): Copy {
  return useContext(CopyContext);
}
