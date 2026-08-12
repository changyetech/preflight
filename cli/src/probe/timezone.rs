//! O2 系统时区一致性 与 C4 `$TZ` 时区一致性。
//!
//! **两条是不同的信号，不是一条的两种说法**（契约 5.1）：
//! - C4 比 `$TZ` 与出口 IP 时区 —— 命令行进程真正跑在这个时区里，**进综合结论**
//! - O2 比系统时区与出口 IP 时区 —— 对应图形界面应用，**只展示不进结论**
//!
//! 系统时区必须**绕开 `$TZ`** 读取，否则两条就成了同一条。

use jiff::Timestamp;
use jiff::tz::TimeZone;

/// 比对结果。`None` = 无从比对（任一侧缺时区名），**不算不一致**（契约 2.3）。
pub type Match = Option<bool>;

/// 比对两个 IANA 时区名。
///
/// 先比名，名不同再比**当前 UTC 偏移**——这是 `ai-ipcheck` 的行为，也是正确的：
/// `US/Pacific` 与 `America/Los_Angeles` 是同一个时区的两个 IANA 名，
/// 只比名字会把它判成「不一致」，给设置完全正确的用户报一个中风险。
///
/// 注意 ipcheck Web 目前**只比名字**（`src/domain/timezone.ts`），这是一处两端分歧。
pub fn compare(local: Option<&str>, exit: Option<&str>) -> Match {
    let (local, exit) = (local?.trim(), exit?.trim());
    if local.is_empty() || exit.is_empty() {
        return None;
    }
    if local.eq_ignore_ascii_case(exit) {
        return Some(true);
    }

    let now = Timestamp::now();
    let local_zone = TimeZone::get(local).ok()?;
    let exit_zone = TimeZone::get(exit).ok()?;
    Some(local_zone.to_offset(now) == exit_zone.to_offset(now))
}

/// 系统时区的 IANA 名，**不受 `$TZ` 影响**。
///
/// 读 `/etc/localtime` 软链是唯一可靠的做法：任何走标准库"当前时区"的 API 都会认 `$TZ`，
/// 那样 O2 就变成了 C4 的复制品。
pub fn system_timezone() -> Option<String> {
    #[cfg(unix)]
    {
        if let Ok(target) = std::fs::read_link("/etc/localtime")
            && let Some((_, name)) = target.to_string_lossy().split_once("zoneinfo/")
        {
            return Some(name.to_string());
        }
        if let Ok(name) = std::fs::read_to_string("/etc/timezone") {
            let name = name.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
        None
    }
    #[cfg(not(unix))]
    {
        windows_system_timezone()
    }
}

#[cfg(not(unix))]
fn windows_system_timezone() -> Option<String> {
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", "[System.TimeZoneInfo]::Local.Id"])
        .output()
        .ok()?;
    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!id.is_empty()).then_some(id)
}

/// 命令行进程实际会用的时区名：`$TZ` 优先，未设则继承系统时区。
pub fn cli_timezone() -> Option<String> {
    match std::env::var("TZ") {
        Ok(tz) if !tz.trim().is_empty() => Some(tz.trim().to_string()),
        _ => system_timezone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_names_match() {
        assert_eq!(
            compare(Some("Asia/Shanghai"), Some("Asia/Shanghai")),
            Some(true)
        );
    }

    #[test]
    fn iana_aliases_match_via_offset_not_name() {
        // 只比名字会把这两个判成不一致，给配置完全正确的用户报中风险。
        assert_eq!(
            compare(Some("US/Pacific"), Some("America/Los_Angeles")),
            Some(true)
        );
    }

    #[test]
    fn same_offset_different_zone_counts_as_match() {
        // 沿用 ai-ipcheck：风控看到的是偏移，同偏移不构成破绽。
        assert_eq!(
            compare(Some("Asia/Shanghai"), Some("Asia/Hong_Kong")),
            Some(true)
        );
    }

    #[test]
    fn genuinely_different_zones_do_not_match() {
        assert_eq!(
            compare(Some("Asia/Shanghai"), Some("America/Los_Angeles")),
            Some(false)
        );
    }

    #[test]
    fn missing_either_side_is_indeterminate_not_mismatch() {
        // 无从比对不等于不一致——检测不出来不该记成用户的异常。
        assert_eq!(compare(None, Some("Asia/Shanghai")), None);
        assert_eq!(compare(Some("Asia/Shanghai"), None), None);
        assert_eq!(compare(Some(""), Some("Asia/Shanghai")), None);
    }

    #[test]
    fn unknown_zone_name_is_indeterminate() {
        // 名字不认识时不能猜——`Windows 标准时间` 这种名字比不出偏移。
        assert_eq!(compare(Some("Totally/Bogus"), Some("Asia/Shanghai")), None);
    }
}
