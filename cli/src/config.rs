//! 配置：来源优先级、白名单、以及 key 的存取。
//!
//! 优先级恒为 **命令行 flag > 环境变量 > 配置文件 > 内置默认**。
//!
//! 允许的键是**白名单**（docs/verdict.md 第 8 节）：`language`、`proxycheck_key`、
//! `timeout`、`no_color`。判级阈值与检测项开关一律禁止——用户能配阈值，判级契约就作废了。
//! 未知键**报错退出**而不是静默忽略：静默忽略会让拼错的键表现成"配了但没生效"，
//! 这是最难查的一类问题。

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::lang::{self, Lang, LangError};

/// 网络探测的默认超时。
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// 配置文件的形状。`deny_unknown_fields` 就是那条白名单——多写一个键，
/// toml 会连键名一起报出来。
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    /// `language` 存字符串而不是 `Lang`：这样非法值由我们自己给出解释，
    /// 而不是 serde 抛一句泛泛的枚举错误（阿拉伯语需要一段说明，见 lang.rs）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxycheck_key: Option<String>,
    /// 网络探测超时，秒。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_color: Option<bool>,
}

/// 解析出配置文件路径。纯函数——调用方负责读环境变量，测试才不必改进程环境。
///
/// `IPCHECK_CONFIG` > 平台约定目录。返回 `None` 表示连家目录都找不到，
/// 此时按"没有配置文件"处理，而不是报错。
pub fn resolve_path(
    ipcheck_config: Option<&str>,
    xdg_config_home: Option<&str>,
    home: Option<&str>,
    appdata: Option<&str>,
) -> Option<PathBuf> {
    if let Some(explicit) = ipcheck_config.filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(explicit));
    }

    if cfg!(windows) {
        return appdata
            .filter(|v| !v.is_empty())
            .map(|base| Path::new(base).join("ipcheck").join("config.toml"));
    }

    // XDG 风格：`~/.config/ipcheck/config.toml`。刻意不用各平台"正统"路径——
    // macOS 的 `~/Library/Application Support/` 对 CLI 用户既不好找也不好手写。
    if let Some(base) = xdg_config_home.filter(|v| !v.is_empty()) {
        return Some(Path::new(base).join("ipcheck").join("config.toml"));
    }
    home.filter(|v| !v.is_empty()).map(|base| {
        Path::new(base)
            .join(".config")
            .join("ipcheck")
            .join("config.toml")
    })
}

/// 从进程环境解析路径。
pub fn path_from_env() -> Option<PathBuf> {
    resolve_path(
        std::env::var("IPCHECK_CONFIG").ok().as_deref(),
        std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
        std::env::var("APPDATA").ok().as_deref(),
    )
}

/// 读配置文件。文件不存在是正常情形，返回默认值。
pub fn load(path: Option<&Path>) -> Result<ConfigFile> {
    let Some(path) = path else {
        return Ok(ConfigFile::default());
    };
    if !path.exists() {
        return Ok(ConfigFile::default());
    }

    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read config: {}", path.display()))?;
    parse(&raw).with_context(|| format!("parse config: {}", path.display()))
}

/// 解析配置内容。独立成函数是为了让白名单行为可以脱离文件系统测试。
pub fn parse(raw: &str) -> Result<ConfigFile> {
    Ok(toml::from_str(raw)?)
}

/// 写配置文件。**权限置 600**——里面可能有 proxycheck key。
///
/// 注意：这是把结构体重新序列化，用户手写的注释会丢。`config set` 是便利路径，
/// 想保留注释就直接编辑文件。
pub fn save(path: &Path, config: &ConfigFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create config dir: {}", parent.display()))?;
    }

    let body = toml::to_string_pretty(config)?;
    std::fs::write(path, body).with_context(|| format!("write config: {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("chmod 600: {}", path.display()))?;
    }

    Ok(())
}

/// 全部来源合并后的最终配置。
pub struct Settings {
    pub lang: Lang,
    pub proxycheck_key: Option<String>,
    pub timeout: Duration,
    pub no_color: bool,
}

/// `Debug` 手写：**key 绝不出现在任何输出里**，包括 `dbg!`、日志与 panic backtrace
/// （docs/verdict.md 第 9 节）。派生的 `Debug` 会把它原样打出来。
impl fmt::Debug for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings")
            .field("lang", &self.lang)
            .field(
                "proxycheck_key",
                &if self.proxycheck_key.is_some() {
                    "<set>"
                } else {
                    "<unset>"
                },
            )
            .field("timeout", &self.timeout)
            .field("no_color", &self.no_color)
            .finish()
    }
}

/// 各来源的原始输入。纯数据，便于测试优先级而不必改进程环境。
pub struct Sources<'a> {
    pub flag_lang: Option<&'a str>,
    pub env_proxycheck_key: Option<&'a str>,
    pub env_no_color: Option<&'a str>,
    pub system_locale: Option<&'a str>,
}

impl Settings {
    pub fn resolve(file: &ConfigFile, sources: Sources<'_>) -> Result<Self, LangError> {
        let lang = lang::resolve(
            sources.flag_lang,
            file.language.as_deref(),
            sources.system_locale,
        )?;

        // key：环境变量 > 配置文件。空串按"未设置"处理——`PROXYCHECK_API_KEY=` 是
        // 脚本里常见的"我清掉了它"，不该被当成一个空 key 送去查询。
        let proxycheck_key = sources
            .env_proxycheck_key
            .filter(|v| !v.trim().is_empty())
            .map(str::to_string)
            .or_else(|| {
                file.proxycheck_key
                    .as_deref()
                    .filter(|v| !v.trim().is_empty())
                    .map(str::to_string)
            });

        // NO_COLOR 的约定：只要存在且非空就生效，不看具体值。
        let no_color = sources.env_no_color.map(|v| !v.is_empty()).unwrap_or(false)
            || file.no_color.unwrap_or(false);

        Ok(Self {
            lang,
            proxycheck_key,
            timeout: file
                .timeout
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_TIMEOUT),
            no_color,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources<'a>() -> Sources<'a> {
        Sources {
            flag_lang: None,
            env_proxycheck_key: None,
            env_no_color: None,
            system_locale: None,
        }
    }

    #[test]
    fn unknown_key_is_an_error_and_names_the_key() {
        // 静默忽略会让拼错的键表现成"配了但没生效"。
        let err = parse("langauge = \"zh-hans\"\n").unwrap_err();
        assert!(
            err.to_string().contains("langauge"),
            "错误信息必须点名那个键，实际是：{err}"
        );
    }

    #[test]
    fn verdict_thresholds_are_not_configurable() {
        // 判级阈值可配 = 判级契约作废。这条测试锁的是那条红线。
        for raw in [
            "risk_threshold = 80\n",
            "[checks]\nipv6 = false\n",
            "high_risk_score = 50\n",
        ] {
            assert!(parse(raw).is_err(), "这份配置本该被拒绝：{raw}");
        }
    }

    #[test]
    fn whitelisted_keys_all_parse() {
        let config = parse(
            "language = \"zh-hans\"\nproxycheck_key = \"k\"\ntimeout = 20\nno_color = true\n",
        )
        .unwrap();
        assert_eq!(config.language.as_deref(), Some("zh-hans"));
        assert_eq!(config.timeout, Some(20));
        assert_eq!(config.no_color, Some(true));
    }

    #[test]
    fn missing_file_is_not_an_error() {
        assert!(load(None).is_ok());
        assert!(load(Some(Path::new("/nonexistent/ipcheck/config.toml"))).is_ok());
    }

    #[test]
    fn explicit_config_path_wins() {
        let got = resolve_path(Some("/tmp/custom.toml"), Some("/x"), Some("/home/u"), None);
        assert_eq!(got, Some(PathBuf::from("/tmp/custom.toml")));
    }

    #[test]
    #[cfg(not(windows))]
    fn unix_uses_xdg_style_paths() {
        assert_eq!(
            resolve_path(None, Some("/x/cfg"), Some("/home/u"), None),
            Some(PathBuf::from("/x/cfg/ipcheck/config.toml"))
        );
        assert_eq!(
            resolve_path(None, None, Some("/home/u"), None),
            Some(PathBuf::from("/home/u/.config/ipcheck/config.toml"))
        );
    }

    #[test]
    fn env_key_wins_over_file_key() {
        let file = ConfigFile {
            proxycheck_key: Some("from-file".into()),
            ..Default::default()
        };
        let settings = Settings::resolve(
            &file,
            Sources {
                env_proxycheck_key: Some("from-env"),
                ..sources()
            },
        )
        .unwrap();
        assert_eq!(settings.proxycheck_key.as_deref(), Some("from-env"));
    }

    #[test]
    fn empty_env_key_falls_back_to_file() {
        let file = ConfigFile {
            proxycheck_key: Some("from-file".into()),
            ..Default::default()
        };
        let settings = Settings::resolve(
            &file,
            Sources {
                env_proxycheck_key: Some(""),
                ..sources()
            },
        )
        .unwrap();
        assert_eq!(settings.proxycheck_key.as_deref(), Some("from-file"));
    }

    #[test]
    fn flag_lang_beats_file_lang() {
        let file = ConfigFile {
            language: Some("zh-hans".into()),
            ..Default::default()
        };
        let settings = Settings::resolve(
            &file,
            Sources {
                flag_lang: Some("en"),
                ..sources()
            },
        )
        .unwrap();
        assert_eq!(settings.lang, Lang::En);
    }

    #[test]
    fn timeout_falls_back_to_default() {
        let settings = Settings::resolve(&ConfigFile::default(), sources()).unwrap();
        assert_eq!(settings.timeout, DEFAULT_TIMEOUT);
    }

    #[test]
    fn no_color_env_is_presence_based() {
        let on = Settings::resolve(
            &ConfigFile::default(),
            Sources {
                env_no_color: Some("0"), // 值无所谓，存在且非空即生效
                ..sources()
            },
        )
        .unwrap();
        assert!(on.no_color);

        let off = Settings::resolve(
            &ConfigFile::default(),
            Sources {
                env_no_color: Some(""),
                ..sources()
            },
        )
        .unwrap();
        assert!(!off.no_color);
    }

    #[test]
    fn debug_never_leaks_the_key() {
        let settings = Settings::resolve(
            &ConfigFile::default(),
            Sources {
                env_proxycheck_key: Some("super-secret-value"),
                ..sources()
            },
        )
        .unwrap();
        let rendered = format!("{settings:?}");
        assert!(!rendered.contains("super-secret-value"), "{rendered}");
        assert!(rendered.contains("<set>"));
    }

    #[test]
    fn saved_config_is_owner_only_and_roundtrips() {
        let dir = std::env::temp_dir().join(format!("ipcheck-cfg-{}", std::process::id()));
        let path = dir.join("config.toml");
        let config = ConfigFile {
            proxycheck_key: Some("secret".into()),
            ..Default::default()
        };
        save(&path, &config).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "配置文件里可能有 key，必须是 600");
        }

        let reloaded = load(Some(&path)).unwrap();
        assert_eq!(reloaded.proxycheck_key.as_deref(), Some("secret"));
        // 没设置的键不该被写成 null 之类的噪音。
        assert!(reloaded.language.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }
}
