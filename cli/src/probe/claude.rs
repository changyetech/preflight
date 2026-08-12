//! C5 Claude 端点检测。
//!
//! **黑名单命中不进综合结论**（ADR-0010）：仍然检测、仍然告警，但不改变档位。
//! 名单是 CC 2.1.197 解出的冻结快照，水印已在 2.1.198 从二进制移除，无法再刷新——
//! 让一份不可更新的陈旧数据驱动「高风险」定论，误报只会随时间单向累积。

use std::path::PathBuf;

/// 端点的三态。
#[derive(Debug, Clone, PartialEq)]
pub enum Endpoint {
    /// 未设 base url，且本机没装过 Claude Code——没什么可说的，不误报。
    NotInstalled,
    /// 官方直连：未设 base url，或 host == api.anthropic.com。
    Official,
    /// 国产大模型：不经 Anthropic，无封号风险。
    Domestic { host: String },
    /// 第三方中转：提示数据泄露与封号风险。
    Relay {
        host: String,
        /// 命中的黑名单条目。**不进综合结论**，只作告警。
        blacklisted: Option<&'static str>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub endpoint: Endpoint,
    /// base url 的来源：环境变量 或 settings.json。仅用于呈现。
    pub source: Option<&'static str>,
}

/// 国产大模型关键词。启发式，宁可漏判为有风险，也不把中转误判成国产。
const DOMESTIC_HINTS: [&str; 24] = [
    "deepseek",
    "moonshot",
    "kimi",
    "minimax",
    "xaminim",
    "zhipu",
    "bigmodel",
    "glm",
    "baichuan",
    "stepfun",
    "01ai",
    "lingyiwanwu",
    "dashscope",
    "qwen",
    "tongyi",
    "volces",
    "volcengine",
    "doubao",
    "hunyuan",
    "wenxin",
    "ernie",
    "iflytek",
    "spark",
    "sensenova",
];

/// 147 项域名黑名单：Anthropic 反蒸馏水印的 known 名单，CC 2.1.197 解出的**冻结快照**。
const BLACKLIST: &str = include_str!("blacklist-147.txt");

pub fn blacklist_entries() -> impl Iterator<Item = &'static str> {
    BLACKLIST
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// host 精确或**后缀**命中名单则返回命中项（复刻水印的 known 匹配）。
pub fn blacklist_hit(host: &str) -> Option<&'static str> {
    let host = host.to_ascii_lowercase();
    blacklist_entries().find(|entry| host == *entry || host.ends_with(&format!(".{entry}")))
}

pub fn is_domestic(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    DOMESTIC_HINTS.iter().any(|hint| host.contains(hint))
}

/// 从 base url 取 host（含端口）。
pub fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let authority = rest.split(['/', '?', '#']).next()?;
    // 去掉 userinfo。
    let authority = authority
        .rsplit_once('@')
        .map(|(_, a)| a)
        .unwrap_or(authority);
    let authority = authority.trim();
    (!authority.is_empty()).then(|| authority.to_ascii_lowercase())
}

/// 纯函数形式的分类，便于测试——调用方负责读环境与文件。
pub fn classify(base_url: Option<&str>, claude_installed: bool) -> Endpoint {
    let Some(url) = base_url.filter(|u| !u.trim().is_empty()) else {
        // 没设 base url：先看本机装没装过 Claude Code，没装就别报「官方直连」——
        // 那会让一个根本不用 Claude 的人以为我们检测过他的配置。
        return if claude_installed {
            Endpoint::Official
        } else {
            Endpoint::NotInstalled
        };
    };

    let Some(host) = host_of(url) else {
        return Endpoint::Relay {
            host: url.to_string(),
            blacklisted: None,
        };
    };

    if host == "api.anthropic.com" {
        return Endpoint::Official;
    }
    if is_domestic(&host) {
        return Endpoint::Domestic { host };
    }

    let blacklisted = blacklist_hit(&host);
    Endpoint::Relay { host, blacklisted }
}

fn config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR")
        && !dir.trim().is_empty()
    {
        return Some(PathBuf::from(dir));
    }
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".claude"))
}

/// 读 `ANTHROPIC_BASE_URL`：shell 环境变量优先，再读 `~/.claude/settings*.json` 的 `env`。
fn read_base_url() -> (Option<String>, Option<&'static str>) {
    if let Ok(url) = std::env::var("ANTHROPIC_BASE_URL")
        && !url.trim().is_empty()
    {
        return (Some(url), Some("env"));
    }

    let Some(dir) = config_dir() else {
        return (None, None);
    };
    for name in ["settings.json", "settings.local.json"] {
        let Ok(raw) = std::fs::read_to_string(dir.join(name)) else {
            continue;
        };
        if let Some(url) = base_url_from_settings(&raw) {
            return (Some(url), Some("settings.json"));
        }
    }
    (None, None)
}

/// 从 settings.json 里取 `env.ANTHROPIC_BASE_URL`。
pub fn base_url_from_settings(raw: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let url = value.get("env")?.get("ANTHROPIC_BASE_URL")?.as_str()?;
    (!url.trim().is_empty()).then(|| url.to_string())
}

fn claude_installed() -> bool {
    if let Some(dir) = config_dir()
        && dir.is_dir()
    {
        return true;
    }
    if let Ok(home) = std::env::var("HOME")
        && PathBuf::from(home).join(".claude.json").exists()
    {
        return true;
    }
    which_claude()
}

fn which_claude() -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join("claude").is_file())
}

pub fn probe() -> Detection {
    let (base_url, source) = read_base_url();
    Detection {
        endpoint: classify(base_url.as_deref(), claude_installed()),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blacklist_snapshot_has_147_entries() {
        // 快照是冻结的。条目数变了说明有人动了名单——那需要一个理由。
        assert_eq!(blacklist_entries().count(), 147);
    }

    #[test]
    fn blacklist_matches_exactly_or_by_suffix() {
        assert_eq!(blacklist_hit("yunwu.ai"), Some("yunwu.ai"));
        assert_eq!(blacklist_hit("api.yunwu.ai"), Some("yunwu.ai"));
        assert_eq!(blacklist_hit("YUNWU.AI"), Some("yunwu.ai"));
        // 后缀匹配必须以点分隔，不能是字符串包含。
        assert_eq!(blacklist_hit("notyunwu.ai"), None);
        assert_eq!(blacklist_hit("api.anthropic.com"), None);
    }

    #[test]
    fn extracts_host_with_port_from_a_base_url() {
        assert_eq!(
            host_of("https://api.anthropic.com").as_deref(),
            Some("api.anthropic.com")
        );
        assert_eq!(
            host_of("https://api.anthropic.com/v1").as_deref(),
            Some("api.anthropic.com")
        );
        assert_eq!(
            host_of("http://127.0.0.1:8080/v1").as_deref(),
            Some("127.0.0.1:8080")
        );
        assert_eq!(
            host_of("https://user:pw@relay.example.com/x").as_deref(),
            Some("relay.example.com")
        );
    }

    #[test]
    fn unset_base_url_is_official_only_when_claude_is_installed() {
        assert_eq!(classify(None, true), Endpoint::Official);
        // 没装过 Claude Code 就别声称检测过他的配置。
        assert_eq!(classify(None, false), Endpoint::NotInstalled);
        assert_eq!(classify(Some("   "), false), Endpoint::NotInstalled);
    }

    #[test]
    fn official_host_is_recognised_with_or_without_path() {
        assert_eq!(
            classify(Some("https://api.anthropic.com"), true),
            Endpoint::Official
        );
        assert_eq!(
            classify(Some("https://api.anthropic.com/v1/"), true),
            Endpoint::Official
        );
        // 后缀伪装不能被当成官方。
        assert!(matches!(
            classify(Some("https://api.anthropic.com.evil.example"), true),
            Endpoint::Relay { .. }
        ));
    }

    #[test]
    fn domestic_models_are_not_flagged_as_relays() {
        // 不经 Anthropic ⇒ 没有封号风险，不该报中转告警。
        assert_eq!(
            classify(Some("https://api.deepseek.com"), true),
            Endpoint::Domestic {
                host: "api.deepseek.com".into()
            }
        );
        assert!(is_domestic("open.bigmodel.cn"));
        assert!(is_domestic("ark.cn-beijing.volces.com"));
    }

    #[test]
    fn third_party_relay_is_flagged_and_blacklist_is_reported_separately() {
        let hit = classify(Some("https://api.yunwu.ai/v1"), true);
        assert_eq!(
            hit,
            Endpoint::Relay {
                host: "api.yunwu.ai".into(),
                blacklisted: Some("yunwu.ai"),
            }
        );

        let clean = classify(Some("https://relay.example.com"), true);
        assert_eq!(
            clean,
            Endpoint::Relay {
                host: "relay.example.com".into(),
                blacklisted: None,
            }
        );
    }

    #[test]
    fn reads_base_url_out_of_settings_json() {
        let raw =
            r#"{"env":{"ANTHROPIC_BASE_URL":"https://relay.example.com","TZ":"Asia/Shanghai"}}"#;
        assert_eq!(
            base_url_from_settings(raw).as_deref(),
            Some("https://relay.example.com")
        );
        assert_eq!(base_url_from_settings(r#"{"env":{}}"#), None);
        assert_eq!(base_url_from_settings("{}"), None);
        assert_eq!(base_url_from_settings("not json"), None);
    }
}
