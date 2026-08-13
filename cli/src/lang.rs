//! 语种与语言选择。
//!
//! 解析顺序：`--lang` > 配置文件 `language` > 系统 locale > `en`。
//!
//! 与 Preflight Web 的一处**刻意分歧**：Web 不做 Accept-Language 自动跳转（规格第 7 节），
//! 因为 URL 必须是语言的唯一真相，且浏览器给的 Accept-Language 常与用户真实偏好脱节。
//! CLI 没有 URL，这条理由整个不适用；而 shell 里的 `LC_ALL`/`LANG` 是用户自己环境的一部分，
//! 可信度远高于浏览器默认值。理由见 docs/verdict.md 第 5 节。

use std::fmt;

/// CLI 支持的语种。终态收缩为 en / zh-hans（ADR-0016）——
/// 繁体中文、俄语、阿拉伯语均不再支持；阿拉伯语额外受终端 BiDi 渲染不一致所限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    ZhHans,
}

impl Lang {
    pub const fn as_str(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::ZhHans => "zh-hans",
        }
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LangError {
    /// 显式请求了一个不支持的语种（包括已删除的 zh-hant / ru / ar）。
    Unknown(String),
}

/// 解析**显式请求**的语种（`--lang` 或配置文件）。认不出就报错——用户明确要了什么，
/// 我们给别的东西是不行的。
pub fn parse_explicit(value: &str) -> Result<Lang, LangError> {
    normalize(value).ok_or_else(|| LangError::Unknown(value.to_string()))
}

/// 从**系统 locale** 推断语种。系统 locale 是提示而非请求，认不出一律回落英文——
/// 包括已删除的语种：用户没有要求过它们，不该因为系统 locale 落在那上面就报错退出。
pub fn from_system_tag(tag: &str) -> Lang {
    normalize(tag).unwrap_or(Lang::En)
}

/// 纯函数形式的解析顺序，便于测试——调用方负责读环境。
pub fn resolve(
    flag: Option<&str>,
    config: Option<&str>,
    system: Option<&str>,
) -> Result<Lang, LangError> {
    if let Some(v) = flag {
        return parse_explicit(v);
    }
    if let Some(v) = config {
        return parse_explicit(v);
    }
    Ok(system.map(from_system_tag).unwrap_or(Lang::En))
}

/// 读系统 locale：**先按 POSIX 顺序读环境变量**，再退回平台 API。
///
/// 不能只用 `sys_locale::get_locale()`——它在 macOS 上读的是 CoreFoundation 的系统 locale，
/// 完全不看 `LC_ALL`/`LANG`。而本工具的用户就是会在 shell 里手设这些变量的那批人
/// （`ai-ipcheck` 的用户尤其如此），只信 CF 会让他们设了中文却看到英文。
/// 平台 API 仍然保留：Windows 上没有这些变量，macOS 图形环境启动的终端也可能没有。
pub fn system_tag() -> Option<String> {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(var) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    sys_locale::get_locale()
}

/// 把各种写法的语言标记归一到我们支持的两种。
///
/// 认得 `zh-hans` / `zh_CN.UTF-8` / `en_US` 这类形态。`zh-hant` / `zh-tw` / `zh-hk` / `zh-mo`
/// 这些繁体标记**故意**归到「认不出」——繁体中文已随语种收缩删除，不再悄悄降级成简体。
fn normalize(raw: &str) -> Option<Lang> {
    let tag = raw.trim().to_ascii_lowercase();
    // 砍掉 `.UTF-8` 之类的编码后缀与 `@modifier`。
    let tag = tag.split(['.', '@']).next().unwrap_or("");
    if tag.is_empty() {
        return None;
    }

    let (primary, rest) = match tag.split_once(['-', '_']) {
        Some((p, r)) => (p, r),
        None => (tag, ""),
    };

    match primary {
        // 繁体的判据是文字系统或地区，不是「不是简体就是繁体」——
        // `zh` 单独出现（无地区）按简体处理，这是中文用户的多数情形；
        // 带繁体标记的一律认不出，回落到 en（系统 locale）或报错（显式请求）。
        "zh" => {
            if is_traditional(rest) {
                None
            } else {
                Some(Lang::ZhHans)
            }
        }
        "en" => Some(Lang::En),
        // `C` / `POSIX` 是"无 locale"，按英文处理。
        "c" | "posix" => Some(Lang::En),
        _ => None,
    }
}

fn is_traditional(rest: &str) -> bool {
    rest.split(['-', '_'])
        .any(|part| matches!(part, "hant" | "tw" | "hk" | "mo"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_wins_over_config_and_system() {
        let got = resolve(Some("zh-hans"), Some("en"), Some("en_US.UTF-8"));
        assert_eq!(got, Ok(Lang::ZhHans));
    }

    #[test]
    fn config_wins_over_system() {
        let got = resolve(None, Some("zh-hans"), Some("en_US.UTF-8"));
        assert_eq!(got, Ok(Lang::ZhHans));
    }

    #[test]
    fn system_locale_is_used_when_nothing_explicit() {
        assert_eq!(resolve(None, None, Some("zh_CN.UTF-8")), Ok(Lang::ZhHans));
        assert_eq!(resolve(None, None, Some("en_US.UTF-8")), Ok(Lang::En));
    }

    #[test]
    fn english_is_the_final_fallback() {
        assert_eq!(resolve(None, None, None), Ok(Lang::En));
        assert_eq!(resolve(None, None, Some("C")), Ok(Lang::En));
        // 认不出的系统 locale 不报错，回落英文。
        assert_eq!(resolve(None, None, Some("ja_JP.UTF-8")), Ok(Lang::En));
    }

    #[test]
    fn unrecognized_zh_variant_is_detected_by_script_or_region() {
        // 无地区的 `zh` 按简体处理。
        assert_eq!(from_system_tag("zh"), Lang::ZhHans);
    }

    #[test]
    fn deleted_locales_fall_back_instead_of_failing_as_system_locale() {
        // 用户没**要求**这些已删除的语种，系统 locale 落在它们上面时不该报错退出。
        assert_eq!(resolve(None, None, Some("zh-Hant-TW")), Ok(Lang::En));
        assert_eq!(resolve(None, None, Some("zh_TW.UTF-8")), Ok(Lang::En));
        assert_eq!(resolve(None, None, Some("zh_HK")), Ok(Lang::En));
        assert_eq!(resolve(None, None, Some("ru_RU.UTF-8")), Ok(Lang::En));
        assert_eq!(resolve(None, None, Some("ar_SA.UTF-8")), Ok(Lang::En));
    }

    #[test]
    fn explicit_unsupported_language_is_refused() {
        assert_eq!(
            parse_explicit("ja"),
            Err(LangError::Unknown("ja".to_string()))
        );
        // 已删除的语种：显式请求时报错，而不是像系统 locale 那样静默回落。
        assert_eq!(
            parse_explicit("zh-hant"),
            Err(LangError::Unknown("zh-hant".to_string()))
        );
        assert_eq!(
            parse_explicit("ru"),
            Err(LangError::Unknown("ru".to_string()))
        );
        assert_eq!(
            parse_explicit("ar"),
            Err(LangError::Unknown("ar".to_string()))
        );
    }
}
