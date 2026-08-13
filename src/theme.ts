// 主题偏好 → 生效主题的推导逻辑（规格 §2 决策 1 / §4.3）。
// 纯函数部分不依赖 DOM，可直接单测；`useTheme` 只负责把它接到 React 与浏览器 API 上。

import { useCallback, useEffect, useState } from "react";

/** 用户偏好，三态：浅色 / 深色 / 跟随系统。 */
export type ThemePref = "light" | "dark" | "system";
/** 实际生效的主题，二态——CSS 只认这个（`:root[data-theme]`）。 */
export type ResolvedTheme = "light" | "dark";

/** ADR-0016：主题偏好是 ADR-0008「不持久化」立场的显式例外，纯外观、无隐私载荷。 */
export const THEME_STORAGE_KEY = "preflight-theme";

/** 把任意值收窄为合法的三态偏好；不认识的值一律落到 system（规格 §4.3）。 */
export function normalizeThemePref(value: unknown): ThemePref {
  return value === "light" || value === "dark" ? value : "system";
}

/** 偏好 + 系统是否深色 → 实际生效主题。 */
export function resolveTheme(
  pref: ThemePref,
  systemPrefersDark: boolean,
): ResolvedTheme {
  if (pref === "light") return "light";
  if (pref === "dark") return "dark";
  return systemPrefersDark ? "dark" : "light";
}

/** 容错读取偏好：隐私模式等场景下 localStorage 访问可能直接抛异常，不能让整页脚本挂掉。 */
export function readStoredThemePref(): ThemePref {
  try {
    return normalizeThemePref(localStorage.getItem(THEME_STORAGE_KEY));
  } catch {
    return "system";
  }
}

/** 容错写入偏好；写入失败（隐私模式等）时静默忽略，偏好只是当次会话生效。 */
export function writeStoredThemePref(pref: ThemePref): void {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, pref);
  } catch {
    // 忽略：写不进去不影响当次会话的主题切换
  }
}

/** 把双属性机制写到 `<html>` 上——`data-theme-pref` 记偏好，`data-theme` 记解析结果，CSS 只读后者。 */
function applyThemeAttrs(pref: ThemePref, resolved: ResolvedTheme): void {
  const el = document.documentElement;
  el.setAttribute("data-theme-pref", pref);
  el.setAttribute("data-theme", resolved);
}

/**
 * 主题状态钩子：初始值取自 localStorage（与 head 内联脚本读同一个键，避免闪白后再次跳变），
 * `system` 档下监听 `prefers-color-scheme` 变化以实时跟手。
 */
export function useTheme(): {
  pref: ThemePref;
  resolved: ResolvedTheme;
  setPref: (pref: ThemePref) => void;
} {
  const [pref, setPrefState] = useState<ThemePref>(() => readStoredThemePref());
  const [resolved, setResolved] = useState<ResolvedTheme>(() =>
    resolveTheme(
      pref,
      window.matchMedia("(prefers-color-scheme: dark)").matches,
    ),
  );

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const sync = () => {
      const next = resolveTheme(pref, mq.matches);
      setResolved(next);
      applyThemeAttrs(pref, next);
    };
    sync();
    // 只有 system 档需要跟系统明暗变化实时联动；light/dark 是用户明确选定，不该被系统信号覆盖。
    if (pref !== "system") return;
    mq.addEventListener("change", sync);
    return () => mq.removeEventListener("change", sync);
  }, [pref]);

  const setPref = useCallback((next: ThemePref) => {
    writeStoredThemePref(next);
    setPrefState(next);
  }, []);

  return { pref, resolved, setPref };
}
