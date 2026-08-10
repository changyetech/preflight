// O2 系统时区一致性（规格 2.2）。

import type { TimezoneResult } from "./types";

/**
 * 比对浏览器时区与出口 IP 时区，两侧都是 IANA 时区名。
 *
 * 出口时区缺失时返回 `match: null`（无法比对），而不是 `false`：
 * 边缘没给出时区是我们的数据缺口，不该记成用户的异常。
 */
export function compareTimezone(
  browserTimezone: string,
  exitTimezone: string | null,
): TimezoneResult {
  const normalize = (tz: string) => tz.trim().toLowerCase();

  return {
    browserTimezone: browserTimezone.trim(),
    exitTimezone,
    match:
      exitTimezone === null
        ? null
        : normalize(browserTimezone) === normalize(exitTimezone),
  };
}

/** 浏览器时区跟随系统时区——这正是本项只覆盖 Claude 桌面版、覆盖不到 CC CLI 的原因（规格 2.2）。 */
export function browserTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone;
}
