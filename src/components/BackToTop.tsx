// 回顶按钮：滚过一屏后浮现在右下角。
//
// 用 <button> 而非 `#top` 锚点：回顶是纯视图操作，不该往地址栏塞 hash、也不该多一条历史记录。

import { useEffect, useState } from "react";

import { useCopy } from "../i18n";

/** 向上箭头。内联 SVG，与 LangSwitch 的地球图标一致，不为一个图标多一次请求。 */
function ArrowUpIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M12 19V5" />
      <path d="m5 12 7-7 7 7" />
    </svg>
  );
}

export function BackToTop() {
  const COPY = useCopy();
  const [shown, setShown] = useState(false);

  useEffect(() => {
    // 阈值定成固定 320px（约首屏结论区之后）：按视口高算的话，
    // 桌面大屏要滚上近千像素才出现，来得太晚。
    const onScroll = () => setShown(window.scrollY > 320);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <button
      type="button"
      className={shown ? "back-to-top is-shown" : "back-to-top"}
      aria-label={COPY.nav.backToTop}
      title={COPY.nav.backToTop}
      onClick={() => {
        const reduce = window.matchMedia(
          "(prefers-reduced-motion: reduce)",
        ).matches;
        window.scrollTo({ top: 0, behavior: reduce ? "auto" : "smooth" });
      }}
    >
      <ArrowUpIcon />
    </button>
  );
}
