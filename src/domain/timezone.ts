// O2 系统时区一致性（判级契约 §2.4）。

import type { TimezoneResult } from "./types";

/**
 * 某个 IANA 时区在某一时刻的 UTC 偏移（分钟）。时区名不认识则返回 `null`。
 *
 * 用「把同一时刻按目标时区格式化，再当成 UTC 解回来」这个办法算偏移，而不是 `longOffset`——
 * 后者的可用性依赖较新的 Intl 实现，前者只要有 tzdata 就成立，且行为在浏览器与 workerd 里一致。
 */
function offsetMinutes(timeZone: string, at: Date): number | null {
  try {
    const parts = new Intl.DateTimeFormat("en-US", {
      timeZone,
      hour12: false,
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    }).formatToParts(at);

    const field = (type: Intl.DateTimeFormatPartTypes) =>
      Number(parts.find((part) => part.type === type)?.value);

    const asIfUtc = Date.UTC(
      field("year"),
      field("month") - 1,
      field("day"),
      // 24 时制在午夜可能给出 "24"，取模归到 0。
      field("hour") % 24,
      field("minute"),
      field("second"),
    );
    return Number.isNaN(asIfUtc)
      ? null
      : Math.round((asIfUtc - at.getTime()) / 60_000);
  } catch {
    // 时区名不合法时 Intl 会抛——那属于「无从比对」，不是「不一致」。
    return null;
  }
}

/**
 * 比对系统时区与出口 IP 时区，两侧都是 IANA 时区名。
 *
 * **先比名，名不同再比当前 UTC 偏移**（判级契约 §2.4，两端统一的算法）。
 * 只比名字会产生一类真实误报：`US/Pacific` 与 `America/Los_Angeles` 是同一个时区的
 * 两个 IANA 名，`Asia/Chongqing` 与 `Asia/Shanghai` 同理——只比名字会给设置完全正确的
 * 用户报一个中风险。风控看到的本来也是偏移，同偏移不构成破绽。
 *
 * 出口时区缺失、或任一时区名不认识 → `match: null`（无从比对），而不是 `false`：
 * 我们的数据缺口不该记成用户的异常。
 */
export function compareTimezone(
  browserTimezone: string,
  exitTimezone: string | null,
): TimezoneResult {
  const local = browserTimezone.trim();

  return {
    browserTimezone: local,
    exitTimezone,
    match: matches(local, exitTimezone),
  };
}

function matches(local: string, exit: string | null): boolean | null {
  if (exit === null) return null;

  const normalized = exit.trim();
  if (local === "" || normalized === "") return null;
  if (local.toLowerCase() === normalized.toLowerCase()) return true;

  const now = new Date();
  const localOffset = offsetMinutes(local, now);
  const exitOffset = offsetMinutes(normalized, now);
  if (localOffset === null || exitOffset === null) return null;

  return localOffset === exitOffset;
}

/** 浏览器时区跟随系统时区——这正是本项只覆盖图形界面应用、覆盖不到 `$TZ` 的原因（契约 §5.1）。 */
export function browserTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone;
}
