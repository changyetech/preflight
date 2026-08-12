// 语种注册表与译文回落（规格第 7 节）。
//
// 这里锁的是「多语化本身」的三条不变量：路径→语言的映射、未译字段回落英文、
// 以及各语种文案的结构与英文源完全同构（漏译在编译期就报错，这里补一道运行时保险）。

import { describe, expect, it } from "vitest";

import {
  COPY,
  LOCALES,
  getCopy,
  langFromPathname,
  localeOf,
  type Lang,
} from "../src/copy";

describe("路径决定语言", () => {
  it.each(LOCALES.map((locale) => [locale.path, locale.id] as const))(
    "%s → %s",
    (path, id) => {
      expect(langFromPathname(path)).toBe(id);
    },
  );

  it("/en 是英文别名，与根路径同语言", () => {
    expect(langFromPathname("/en")).toBe("en");
  });

  it("未注册路径落回默认语言英文，不猜测、不跳转", () => {
    expect(langFromPathname("/fr")).toBe("en");
    expect(langFromPathname("/zh")).toBe("en");
    expect(langFromPathname("/")).toBe("en");
  });

  it("语种前缀大小写不敏感（用户手敲 /ZH-Hans 也要落到简体）", () => {
    expect(langFromPathname("/ZH-Hans")).toBe("zh-hans");
  });
});

describe("语种注册表", () => {
  it("每个语种的 path / htmlLang / label 都唯一，菜单里不会出现两个同名项", () => {
    for (const key of ["path", "htmlLang", "label"] as const) {
      const values = LOCALES.map((locale) => locale[key]);
      expect(new Set(values).size).toBe(values.length);
    }
  });

  it("阿拉伯语是唯一的 RTL 语种", () => {
    const rtl = LOCALES.filter((locale) => locale.dir === "rtl");

    expect(rtl.map((locale) => locale.id)).toEqual(["ar"]);
  });

  it("localeOf 对每个 Lang 都取得到", () => {
    for (const locale of LOCALES) {
      expect(localeOf(locale.id).path).toBe(locale.path);
    }
  });
});

describe("译文回落", () => {
  /** 递归比较结构：键集合一致、叶子都是非空字符串。 */
  function assertSameShape(actual: unknown, expected: unknown, path: string) {
    if (typeof expected === "string") {
      expect(typeof actual, path).toBe("string");
      expect((actual as string).length, path).toBeGreaterThan(0);
      return;
    }

    const a = actual as Record<string, unknown>;
    const e = expected as Record<string, unknown>;
    expect(Object.keys(a).sort(), path).toEqual(Object.keys(e).sort());
    for (const key of Object.keys(e)) {
      assertSameShape(a[key], e[key], `${path}.${key}`);
    }
  }

  it.each(LOCALES.map((locale) => locale.id))(
    "%s 的文案结构与英文源同构，且没有空字符串",
    (id: Lang) => {
      assertSameShape(getCopy(id), COPY, id);
    },
  );

  it("未译语种整份回落英文源（ru 目前一条未译）", () => {
    expect(getCopy("ru")).toEqual(COPY);
  });

  it("已译语种不回落（简体中文站点标题与英文不同）", () => {
    expect(getCopy("zh-hans").site.title).not.toBe(COPY.site.title);
  });

  it("回落是字段级的：补一条译文不影响其余字段仍取英文", () => {
    // 直接验证合并语义，而不是等某个语种真的补译——补译顺序不该影响这条保证。
    const copy = getCopy("zh-hans");

    expect(copy.actions.installCommand).toBe(COPY.actions.installCommand);
    expect(copy.actions.retry).not.toBe(COPY.actions.retry);
  });
});
