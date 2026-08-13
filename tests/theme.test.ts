// 主题偏好推导的纯函数测试（规格 §2 决策 1 / §4.3）。
// 只测 src/theme.ts 里不依赖 React 的部分——`useTheme` 钩子接的是 DOM/浏览器 API，
// 交给结构化验收（brief 验证条款 3）里点名要覆盖的六种场景在这里逐一断言。

import { afterEach, describe, expect, it, vi } from "vitest";

import {
  normalizeThemePref,
  readStoredThemePref,
  resolveTheme,
  writeStoredThemePref,
} from "../src/theme";

describe("resolveTheme", () => {
  it("light 偏好恒为 light，不受系统明暗影响", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("light", false)).toBe("light");
  });

  it("dark 偏好恒为 dark，不受系统明暗影响", () => {
    expect(resolveTheme("dark", true)).toBe("dark");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("system 偏好 + 系统深色 → dark", () => {
    expect(resolveTheme("system", true)).toBe("dark");
  });

  it("system 偏好 + 系统浅色 → light", () => {
    expect(resolveTheme("system", false)).toBe("light");
  });
});

describe("normalizeThemePref", () => {
  it("认识 light/dark，原样收窄", () => {
    expect(normalizeThemePref("light")).toBe("light");
    expect(normalizeThemePref("dark")).toBe("dark");
  });

  it("null/undefined/非法字符串一律落到 system 档", () => {
    expect(normalizeThemePref(null)).toBe("system");
    expect(normalizeThemePref(undefined)).toBe("system");
    expect(normalizeThemePref("auto")).toBe("system");
    expect(normalizeThemePref("")).toBe("system");
  });
});

describe("readStoredThemePref", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("localStorage 为空（键不存在）→ system 档", () => {
    vi.stubGlobal("localStorage", { getItem: () => null });
    expect(readStoredThemePref()).toBe("system");
  });

  it("localStorage 存了合法偏好 → 原样读回", () => {
    vi.stubGlobal("localStorage", { getItem: () => "dark" });
    expect(readStoredThemePref()).toBe("dark");
  });

  it("localStorage 访问抛异常（如隐私模式）→ 不崩，落到 system 档", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("access denied");
      },
    });
    expect(() => readStoredThemePref()).not.toThrow();
    expect(readStoredThemePref()).toBe("system");
  });

  it("localStorage 全局本身不存在（如本测试运行环境）→ 不崩，落到 system 档", () => {
    vi.stubGlobal("localStorage", undefined);
    expect(() => readStoredThemePref()).not.toThrow();
    expect(readStoredThemePref()).toBe("system");
  });
});

describe("writeStoredThemePref", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("正常写入时把值转交给 localStorage.setItem", () => {
    const setItem = vi.fn();
    vi.stubGlobal("localStorage", { setItem });
    writeStoredThemePref("dark");
    expect(setItem).toHaveBeenCalledWith("ipcheck-theme", "dark");
  });

  it("写入抛异常时静默忽略，不影响调用方", () => {
    vi.stubGlobal("localStorage", {
      setItem: () => {
        throw new Error("access denied");
      },
    });
    expect(() => writeStoredThemePref("light")).not.toThrow();
  });
});
