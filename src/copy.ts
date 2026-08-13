// 语种注册表与文案聚合入口（规格第 7 节）。文案本身在 src/locales/ 下按语种分文件。
//
// 英文是源语言：`Copy` 类型由 en.ts 推导，其余语种按它对齐。
// 语种终态只有 en + zh-hans 两个，且均为完整 `Copy`（规格 §2 决策 9）：
// 类型强制译全，不再有字段级回落。

import { EN, type Copy } from "./locales/en";
import { ZH_HANS } from "./locales/zh-hans";

export type { Copy } from "./locales/en";

export type Lang = "en" | "zh-hans";

export type Locale = {
  id: Lang;
  /** URL 前缀。英文是根路径（规格第 7 节：默认英语）。 */
  path: string;
  /** `<html lang>` 的值，用 BCP 47 规范大小写，与 URL 里的小写前缀不同。 */
  htmlLang: string;
  /** 语言菜单里的自称，永远用该语言自己的写法，不随当前页语言变化。 */
  label: string;
};

/** 菜单顺序即此处顺序：英文（默认）在前。 */
export const LOCALES: readonly Locale[] = [
  { id: "en", path: "/", htmlLang: "en", label: "English" },
  {
    id: "zh-hans",
    path: "/zh-hans",
    htmlLang: "zh-Hans",
    label: "简体中文",
  },
];

export function localeOf(lang: Lang): Locale {
  // LOCALES 覆盖 Lang 的全部取值，找不到只可能是表被改坏了，属于开发期错误。
  const hit = LOCALES.find((locale) => locale.id === lang);
  if (!hit) throw new Error(`unknown lang: ${lang}`);
  return hit;
}

/**
 * 语言取自路径首段：`/zh-hans`，其余（含 `/` 与已删除的 `/en`）为英文。
 * 不做 Accept-Language 自动跳转（规格第 7 节）。
 */
export function langFromPathname(pathname: string): Lang {
  const segment = pathname.split("/")[1]?.toLowerCase();
  const hit = LOCALES.find((locale) => locale.id === segment);
  return hit ? hit.id : "en";
}

const TRANSLATIONS: Record<Lang, Copy> = {
  en: EN,
  "zh-hans": ZH_HANS,
};

export function getCopy(lang: Lang): Copy {
  return TRANSLATIONS[lang];
}

/** 默认文案是英文（规格第 7 节：默认语言为英语）。 */
export const COPY: Copy = EN;
export const COPY_ZH_HANS: Copy = getCopy("zh-hans");
