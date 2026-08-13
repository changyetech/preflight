// 右上角主题菜单：浅色/深色/跟随系统三态下拉（规格 §2 决策 1）。
//
// 三态需要「当前生效项打勾」这种比链接列表更强的语义，照原型走 button + role="menu"
// 的可达性模式（aria-haspopup/aria-expanded/role=menuitem/aria-current），自己管
// 开合状态、点外部与 Esc 关闭。状态推导（偏好 → 生效主题）是纯函数，放在 ../theme
// 里单独测试，这里只接 DOM。

import { useEffect, useRef, useState } from "react";

import { useCopy } from "../i18n";
import { useTheme, type ThemePref } from "../theme";

const OPTIONS: readonly ThemePref[] = ["light", "dark", "system"];

/** 太阳图标：浅色。 */
function SunIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      width="14"
      height="14"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="4.2" />
      <path d="M12 2.5v2.6M12 18.9v2.6M4.22 4.22l1.84 1.84M17.94 17.94l1.84 1.84M2.5 12h2.6M18.9 12h2.6M4.22 19.78l1.84-1.84M17.94 6.06l1.84-1.84" />
    </svg>
  );
}

/** 月牙图标：深色。 */
function MoonIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      width="14"
      height="14"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M20.2 14.3A8.4 8.4 0 0 1 9.7 3.8a8.4 8.4 0 1 0 10.5 10.5z" />
    </svg>
  );
}

/** 显示器图标：跟随系统。 */
function SystemIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      width="14"
      height="14"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect x="2.8" y="3.8" width="18.4" height="12.6" rx="1.8" />
      <path d="M8.6 20.2h6.8M12 16.4v3.8" />
    </svg>
  );
}

/** 勾选标记：当前生效档的提示，用 visibility 占位（见样式）而非 display，避免行宽跳动。 */
function CheckIcon() {
  return (
    <svg
      className="theme-menu-chk"
      viewBox="0 0 24 24"
      width="14"
      height="14"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M4.5 12.5l5 5 10-10" />
    </svg>
  );
}

const ICONS = { light: SunIcon, dark: MoonIcon, system: SystemIcon };

export function ThemeSwitch() {
  const COPY = useCopy();
  const { pref, setPref } = useTheme();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  // 点击菜单外部或按 Esc 关闭——菜单是低频弹层，不必用全局单例管理器（当前只有这一个）。
  useEffect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("click", onDocClick);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("click", onDocClick);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  const TriggerIcon = ICONS[pref];
  const label = `${COPY.nav.theme.label}：${COPY.nav.theme[pref]}`;

  return (
    <div className="theme-menu" ref={rootRef}>
      <button
        type="button"
        className="theme-switch"
        aria-haspopup="true"
        aria-expanded={open}
        aria-label={label}
        title={label}
        onClick={() => setOpen((o) => !o)}
      >
        <TriggerIcon />
      </button>

      <div className="theme-menu-list" role="menu" hidden={!open}>
        {OPTIONS.map((opt) => {
          const Icon = ICONS[opt];
          return (
            <button
              key={opt}
              type="button"
              role="menuitem"
              aria-current={opt === pref ? "true" : undefined}
              onClick={() => {
                setPref(opt);
                setOpen(false);
              }}
            >
              <span className="theme-menu-item-label">
                <Icon />
                {COPY.nav.theme[opt]}
              </span>
              <CheckIcon />
            </button>
          );
        })}
      </div>
    </div>
  );
}
