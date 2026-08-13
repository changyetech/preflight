// 语种注册表（规格第 7 节）。
//
// 这里锁的是「多语化本身」的不变量：路径→语言的映射、语种取值域的大小与唯一性。
// 字段级回落机制随收缩一起删除（规格 §2 决策 9），不再是本文件的断言对象；
// RTL（`dir` 字段）随 ar 删除而移除（规格 §3），同样不再断言。

import { describe, expect, it } from "vitest";

import { LOCALES, langFromPathname, localeOf } from "../src/copy";

describe("路径决定语言", () => {
  it.each(LOCALES.map((locale) => [locale.path, locale.id] as const))(
    "%s → %s",
    (path, id) => {
      expect(langFromPathname(path)).toBe(id);
    },
  );

  it.each(["/fr", "/zh", "/", "/en", "/zh-hant", "/ru", "/ar"])(
    "%s 落回默认语言英文，不猜测、不跳转（含已删除的 /en、/zh-hant、/ru、/ar）",
    (path) => {
      expect(langFromPathname(path)).toBe("en");
    },
  );

  it("语种前缀大小写不敏感（用户手敲 /ZH-Hans 也要落到简体）", () => {
    expect(langFromPathname("/ZH-Hans")).toBe("zh-hans");
  });
});

describe("语种注册表", () => {
  it("语种取值域收缩为两个：en + zh-hans（规格 §3）", () => {
    expect(LOCALES.map((locale) => locale.id)).toEqual(["en", "zh-hans"]);
  });

  it("每个语种的 path / htmlLang / label 都唯一，菜单里不会出现两个同名项", () => {
    for (const key of ["path", "htmlLang", "label"] as const) {
      const values = LOCALES.map((locale) => locale[key]);
      expect(new Set(values).size).toBe(values.length);
    }
  });

  it("localeOf 对每个 Lang 都取得到", () => {
    for (const locale of LOCALES) {
      expect(localeOf(locale.id).path).toBe(locale.path);
    }
  });
});
