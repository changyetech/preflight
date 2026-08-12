// 语种注册表与文案聚合入口（规格第 7 节）。文案本身在 src/locales/ 下按语种分文件。
//
// 英文是源语言：`Copy` 类型由 en.ts 推导，其余语种按它对齐。
// 尚未译全的语种（zh-hant / ru / ar）只写已翻好的字段，其余在 getCopy 里逐字段回落英文——
// 回落发生在字段粒度而非整份文件，因此补译可以一条一条来，不必凑齐一整份才能上线。

import { AR } from "./locales/ar";
import { EN, type Copy, type PartialCopy } from "./locales/en";
import { RU } from "./locales/ru";
import { ZH_HANS } from "./locales/zh-hans";
import { ZH_HANT } from "./locales/zh-hant";

export type { Copy } from "./locales/en";

export type Lang = "en" | "zh-hans" | "zh-hant" | "ru" | "ar";

export type Locale = {
  id: Lang;
  /** URL 前缀。英文是根路径（规格第 7 节：默认英语）。 */
  path: string;
  /** `<html lang>` 的值，用 BCP 47 规范大小写，与 URL 里的小写前缀不同。 */
  htmlLang: string;
  dir: "ltr" | "rtl";
  /** 语言菜单里的自称，永远用该语言自己的写法，不随当前页语言变化。 */
  label: string;
};

/** 菜单顺序即此处顺序：英文（默认）在前，其余按目标受众规模排。 */
export const LOCALES: readonly Locale[] = [
  { id: "en", path: "/", htmlLang: "en", dir: "ltr", label: "English" },
  {
    id: "zh-hans",
    path: "/zh-hans",
    htmlLang: "zh-Hans",
    dir: "ltr",
    label: "简体中文",
  },
  {
    id: "zh-hant",
    path: "/zh-hant",
    htmlLang: "zh-Hant",
    dir: "ltr",
    label: "繁體中文",
  },
  { id: "ru", path: "/ru", htmlLang: "ru", dir: "ltr", label: "Русский" },
  { id: "ar", path: "/ar", htmlLang: "ar", dir: "rtl", label: "العربية" },
];

export function localeOf(lang: Lang): Locale {
  // LOCALES 覆盖 Lang 的全部取值，找不到只可能是表被改坏了，属于开发期错误。
  const hit = LOCALES.find((locale) => locale.id === lang);
  if (!hit) throw new Error(`unknown lang: ${lang}`);
  return hit;
}

/**
 * 语言取自路径首段：`/zh-hans`、`/zh-hant`、`/ru`、`/ar`，其余（含 `/` 与别名 `/en`）为英文。
 * 不做 Accept-Language 自动跳转（规格第 7 节）。
 */
export function langFromPathname(pathname: string): Lang {
  const segment = pathname.split("/")[1]?.toLowerCase();
  const hit = LOCALES.find((locale) => locale.id === segment);
  return hit ? hit.id : "en";
}

/** 把译文补丁按字段合并到英文源上：给了的用译文，没给的用英文。 */
function merge<T>(base: T, patch: unknown): T {
  if (typeof patch !== "object" || patch === null) return base;

  const out = { ...base } as Record<string, unknown>;
  for (const [key, value] of Object.entries(patch)) {
    const fallback = out[key];
    out[key] =
      typeof value === "object" && value !== null
        ? merge(fallback, value)
        : value;
  }
  return out as T;
}

const TRANSLATIONS: Record<Lang, Copy | PartialCopy> = {
  en: EN,
  "zh-hans": ZH_HANS,
  "zh-hant": ZH_HANT,
  ru: RU,
  ar: AR,
};

/** 每份文案只合并一次：Provider 每次渲染都调 getCopy，不缓存会把整棵树的 props 打成新对象。 */
const RESOLVED = new Map<Lang, Copy>();

export function getCopy(lang: Lang): Copy {
  const cached = RESOLVED.get(lang);
  if (cached) return cached;

  const resolved = lang === "en" ? EN : merge(EN as Copy, TRANSLATIONS[lang]);
  RESOLVED.set(lang, resolved);
  return resolved;
}

/** 默认文案是英文（规格第 7 节：默认语言为英语）。 */
export const COPY: Copy = EN;
export const COPY_ZH_HANS: Copy = getCopy("zh-hans");
