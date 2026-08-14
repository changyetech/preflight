//! Preflight CLI —— 网络环境体检。
//!
//! 判级契约见 docs/verdict.md（Web 与 CLI 共同的判据），实现计划见
//! docs/plans/2026-08-12-cli-rust-rewrite.md。

mod config;
mod copy;
mod domain;
mod json;
mod lang;
mod probe;
mod render;
mod uninstall;
mod update;

use std::io::{IsTerminal, Write};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};

use config::{ConfigFile, Settings, Sources};
use copy::Text;
use lang::{Lang, LangError};

#[derive(Parser)]
#[command(
    name = "preflight",
    version,
    // GNU 惯例：版权信息进 `--version`（`-V` 仍是短版本号），日常报告输出保持干净。
    long_version = long_version(),
    about = "Check your network environment before using IP-sensitive tools"
)]
struct Cli {
    /// Interface language: en / zh-hans
    #[arg(long, global = true, value_name = "LANG")]
    lang: Option<String>,

    /// Machine-readable output
    #[arg(long)]
    json: bool,

    /// Include the explanation of every check
    #[arg(long, short)]
    verbose: bool,

    /// 不给子命令时跑体检——这是主路径，`config` 只是旁支。
    #[command(subcommand)]
    command: Option<Command>,
}

/// `--version` 的完整输出：版本号 + 版权行。年份取运行时当前年，不写死。
fn long_version() -> String {
    format!(
        "{}\n© {} Hangzhou Changye Network Technology Co., Ltd.",
        env!("CARGO_PKG_VERSION"),
        jiff::Zoned::now().year()
    )
}

#[derive(Subcommand)]
enum Command {
    /// View and change configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// List public DNS servers (optionally test reachability with --check)
    Dns {
        /// Test each server with a real DNS query
        #[arg(long)]
        check: bool,
    },
    /// Update preflight to the latest release
    Update,
    /// Remove preflight from this machine
    Uninstall {
        /// Also remove the config directory
        #[arg(long)]
        purge: bool,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Print the config file path actually in use
    Path,
    /// List the effective value of every key (secrets are never echoed)
    List,
    /// Set a value (the API key is read interactively and never echoed)
    Set {
        #[command(subcommand)]
        target: SetTarget,
    },
    /// Remove a key from the config file, restoring the built-in default
    Unset { key: ConfigKey },
    /// Show the effective value of a key (secrets are never echoed)
    Get { key: ConfigKey },
}

/// `config set` 的目标。刻意做成**子命令**而不是「键 + 可选值」的一对位置参数：
/// 只有这样才能在**解析层**就拒绝 `config set proxycheck-key <KEY>`——明文 key
/// 会进 shell history 与 `ps`，这条防线不能退到运行时。
/// 顺带的好处：每个键的取值类型与范围由 clap 校验，非法值根本进不了配置文件。
#[derive(Subcommand)]
enum SetTarget {
    /// Interface language: en / zh-hans
    Language { value: String },
    /// proxycheck API key — read interactively, never echoed
    ProxycheckKey,
    /// Network probe timeout, in seconds
    Timeout {
        #[arg(value_parser = clap::value_parser!(u64).range(1..=120))]
        value: u64,
    },
    /// Disable colored output: true / false
    NoColor {
        #[arg(action = clap::ArgAction::Set, value_parser = clap::value_parser!(bool))]
        value: bool,
    },
}

/// `config get` / `config unset` 的键，与配置文件白名单一一对应（docs/verdict.md §8）。
/// 判级阈值与检测项开关永远不在此列。
#[derive(Clone, Copy, ValueEnum)]
enum ConfigKey {
    #[value(name = "language")]
    Language,
    #[value(name = "proxycheck-key")]
    ProxycheckKey,
    #[value(name = "timeout")]
    Timeout,
    #[value(name = "no-color")]
    NoColor,
}

/// 退出码约定：
/// - `0` —— 体检完成。**不论风险档位**：用退出码表达风险会让脚本把「高风险」
///   和「工具挂了」混为一谈，而这两件事需要完全不同的处置。
/// - `1` —— 工具自身失败（配置不合法、语言无效等）。
/// - `2` —— 体检跑了，但一个贡献信号都没产出（结论为「数据不足」）。
///   报告仍然照常输出，只是它不构成一个可用的结论。
const EXIT_TOOL_FAILURE: i32 = 1;
const EXIT_NO_VERDICT: i32 = 2;

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("preflight: {err:#}");
            std::process::exit(EXIT_TOOL_FAILURE);
        }
    }
}

fn run() -> Result<i32> {
    let cli = Cli::parse();
    let system_locale = lang::system_tag();

    // Windows 上一次 update 留下的旧 exe 只能由下一次启动清理（update.rs）。
    #[cfg(windows)]
    update::remove_leftover();

    // `uninstall` / `update` 必须**先于配置文件加载**分发：配置文件是
    // deny_unknown_fields，一个拼错的键（或 `language = "ar"`）就会让每一条命令
    // 退出 1——包括专门删掉它的 uninstall，以及若某版本把解析弄出 bug 时唯一的
    // 修复通道 update。因此语言只从 `--lang` + 系统 locale 解析，拿默认 ConfigFile
    // 走同一条 Settings::resolve（超时因此恒为内置默认，对 update 的两个小请求
    // 足够）；`--lang` 本身非法仍照常报错，那是用户给的 flag，不是坏文件的锅。
    if matches!(
        cli.command,
        Some(Command::Uninstall { .. } | Command::Update)
    ) {
        let settings = Settings::resolve(
            &ConfigFile::default(),
            Sources {
                flag_lang: cli.lang.as_deref(),
                env_proxycheck_key: None,
                env_no_color: None,
                system_locale: system_locale.as_deref(),
            },
        )
        .map_err(|err| render_lang_error(err, system_locale.as_deref()))?;
        let text = copy::text(settings.lang);
        match cli.command {
            Some(Command::Uninstall { purge }) => uninstall::run(purge, &text)?,
            Some(Command::Update) => update::run(settings.timeout, &text)?,
            _ => unreachable!(),
        }
        return Ok(0);
    }

    let config_path = config::path_from_env();
    let file = config::load(config_path.as_deref())
        .map_err(|err| annotate_config_error(err, cli.lang.as_deref(), system_locale.as_deref()))?;

    let settings = Settings::resolve(
        &file,
        Sources {
            flag_lang: cli.lang.as_deref(),
            env_proxycheck_key: std::env::var("PROXYCHECK_API_KEY").ok().as_deref(),
            env_no_color: std::env::var("NO_COLOR").ok().as_deref(),
            system_locale: system_locale.as_deref(),
        },
    )
    .map_err(|err| render_lang_error(err, system_locale.as_deref()))?;

    let text = copy::text(settings.lang);

    match cli.command {
        Some(Command::Config { action }) => {
            run_config(
                action,
                &text,
                config_path.as_deref(),
                file,
                &settings,
                cli.lang.as_deref(),
            )?;
            Ok(0)
        }
        Some(Command::Dns { check }) => run_dns(&cli, &text, &settings, check),
        // 已在配置文件加载之前处理并返回。
        Some(Command::Uninstall { .. } | Command::Update) => unreachable!(),
        None => run_checkup(&cli, &text, &settings),
    }
}

fn run_checkup(cli: &Cli, text: &Text, settings: &Settings) -> Result<i32> {
    // 进度提示只往 **stderr** 打，且只在交互终端里打：stdout 恒为一份干净的报告，
    // `preflight | cat` 与 `--json` 的语义因此完全一致。
    let interactive = std::io::stderr().is_terminal();
    if interactive && !cli.json {
        eprint!("{}\r", text.values.checking);
        let _ = std::io::stderr().flush();
    }

    // 一次性渲染：全部探测结束后才输出。不做光标控制的渐进渲染——
    // 双渲染路径 + SIGWINCH + 终端兼容性引入的 bug 面比整个探测层还大。
    let report = probe::run(settings.timeout, settings.proxycheck_key.as_deref());

    if interactive && !cli.json {
        eprint!("\x1b[2K\r");
        let _ = std::io::stderr().flush();
    }

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&json::report(&report))?);
    } else {
        // 非 TTY 自动关色：重定向到文件不该满屏转义序列。
        let color = !settings.no_color && std::io::stdout().is_terminal();
        // 宽度跟随窗口，量的是 **stdout** 那一端：重定向到文件/管道时拿不到宽度，
        // 落回固定宽度——`preflight > report.txt` 的产物不该随当时的窗口大小变化。
        // 只在渲染前量一次，不监听 SIGWINCH：报告是一次性输出，不重排。
        let style = match terminal_size::terminal_size_of(std::io::stdout()) {
            Some((terminal_size::Width(columns), _)) => {
                render::Style::sized(color, columns as usize)
            }
            None => render::Style::new(color),
        };
        print!("{}", render::report(&report, text, &style, cli.verbose));
    }

    Ok(match report.verdict() {
        domain::verdict::Verdict::Insufficient => EXIT_NO_VERDICT,
        _ => 0,
    })
}

fn run_dns(cli: &Cli, text: &Text, settings: &Settings, check: bool) -> Result<i32> {
    let entries = domain::dns_servers::all();

    let results = if check {
        let interactive = std::io::stderr().is_terminal();
        if interactive && !cli.json {
            eprint!("{}\r", text.values.checking);
            let _ = std::io::stderr().flush();
        }
        let r = probe::dns_check::check_all(entries, settings.timeout);
        if interactive && !cli.json {
            eprint!("\x1b[2K\r");
            let _ = std::io::stderr().flush();
        }
        Some(r)
    } else {
        None
    };

    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json::dns_servers(entries, results.as_deref()))?
        );
    } else {
        let color = !settings.no_color && std::io::stdout().is_terminal();
        let style = match terminal_size::terminal_size_of(std::io::stdout()) {
            Some((terminal_size::Width(columns), _)) => {
                render::Style::sized(color, columns as usize)
            }
            None => render::Style::new(color),
        };
        print!(
            "{}",
            render::dns_table(entries, results.as_deref(), text, &style)
        );
    }

    Ok(0)
}

fn run_config(
    action: ConfigAction,
    text: &Text,
    config_path: Option<&std::path::Path>,
    mut file: ConfigFile,
    settings: &Settings,
    flag_lang: Option<&str>,
) -> Result<()> {
    // `list` 打的是**合并后的生效值**（flag > 环境变量 > 配置文件 > 默认），
    // 不是配置文件原文——用户想问的是「现在到底用的什么」。不依赖配置文件路径，
    // 放在路径守卫之前。
    if let ConfigAction::List = action {
        println!("language = {}", settings.lang);
        println!(
            "proxycheck_key = {}",
            if settings.proxycheck_key.is_some() {
                text.config.list_value_set
            } else {
                text.config.list_value_unset
            }
        );
        println!("timeout = {}", settings.timeout.as_secs());
        println!("no_color = {}", settings.no_color);
        return Ok(());
    }

    let Some(path) = config_path else {
        bail!(
            "{}: HOME / XDG_CONFIG_HOME / APPDATA",
            text.errors.config_read
        );
    };

    match action {
        // 已在上方的路径守卫之前处理并返回。
        ConfigAction::List => unreachable!(),

        ConfigAction::Path => {
            println!("{}: {}", text.config.path_label, path.display());
        }

        // `get` 与 `list` 同口径，打的都是**生效值**（flag > 环境变量 > 配置文件 > 默认）。
        ConfigAction::Get { key } => match key {
            ConfigKey::Language => println!("{}", settings.lang),
            ConfigKey::Timeout => println!("{}", settings.timeout.as_secs()),
            ConfigKey::NoColor => println!("{}", settings.no_color),
            // 只报状态，绝不回显 key 本身。
            ConfigKey::ProxycheckKey => println!(
                "{}",
                if settings.proxycheck_key.is_some() {
                    text.config.key_state_set
                } else {
                    text.config.key_state_unset
                }
            ),
        },

        ConfigAction::Set { target } => match target {
            SetTarget::Language { value } => {
                // 先校验再落盘：非法的 language 会让**之后的每一条命令**都启动不了。
                lang::parse_explicit(&value).map_err(|err| lang_error(text, err))?;
                file.language = Some(value);
                save_config(path, &file, text, ConfigKey::Language, flag_lang)?;
            }

            SetTarget::Timeout { value } => {
                file.timeout = Some(value);
                save_config(path, &file, text, ConfigKey::Timeout, flag_lang)?;
            }

            SetTarget::NoColor { value } => {
                file.no_color = Some(value);
                save_config(path, &file, text, ConfigKey::NoColor, flag_lang)?;
            }

            SetTarget::ProxycheckKey => {
                // 交互式、不回显。刻意不提供 `--proxycheck-key <KEY>` 明文 flag：
                // 那会把 secret 写进 shell history，也会出现在 `ps` 的进程列表里。
                let entered = rpassword::prompt_password(format!("{} ", text.config.key_prompt))
                    .context("read key from terminal")?;
                let entered = entered.trim().to_string();

                if entered.is_empty() {
                    println!("{}", text.config.key_empty);
                    return Ok(());
                }

                file.proxycheck_key = Some(entered);
                config::save(path, &file)
                    .map_err(|err| err.context(text.errors.config_write.to_string()))?;
                // key 有自己的成功文案（要顺带说配额从 100 涨到 1000）。
                println!("{}", text.config.key_saved);
                warn_if_overridden(text, ConfigKey::ProxycheckKey, flag_lang);
            }
        },

        ConfigAction::Unset { key } => {
            // 幂等：本来就没设也照常写一次，用户要的是"之后按默认走"这个结果。
            match key {
                ConfigKey::Language => file.language = None,
                ConfigKey::ProxycheckKey => file.proxycheck_key = None,
                ConfigKey::Timeout => file.timeout = None,
                ConfigKey::NoColor => file.no_color = None,
            }
            config::save(path, &file)
                .map_err(|err| err.context(text.errors.config_write.to_string()))?;
            println!("{}", text.config.unset_saved);
            warn_if_overridden(text, key, flag_lang);
        }
    }

    Ok(())
}

/// 落盘并报告结果。
fn save_config(
    path: &std::path::Path,
    file: &ConfigFile,
    text: &Text,
    key: ConfigKey,
    flag_lang: Option<&str>,
) -> Result<()> {
    config::save(path, file).map_err(|err| err.context(text.errors.config_write.to_string()))?;
    println!("{}", text.config.set_saved);
    warn_if_overridden(text, key, flag_lang);
    Ok(())
}

/// 写进去了，但当前生效的是更高优先级的来源——不提示的话，用户会以为"配了没生效"。
/// 提示走 **stderr**：stdout 上只留命令自身的结果。
fn warn_if_overridden(text: &Text, key: ConfigKey, flag_lang: Option<&str>) {
    let source = match key {
        ConfigKey::Language => flag_lang.map(|_| "--lang"),
        ConfigKey::ProxycheckKey => std::env::var("PROXYCHECK_API_KEY")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(|_| "PROXYCHECK_API_KEY"),
        // NO_COLOR 的约定：存在且非空即生效，不看具体值。
        ConfigKey::NoColor => std::env::var("NO_COLOR")
            .ok()
            .filter(|v| !v.is_empty())
            .map(|_| "NO_COLOR"),
        // timeout 没有更高优先级的来源。
        ConfigKey::Timeout => None,
    };

    if let Some(source) = source {
        eprintln!("{}{source}", text.config.override_notice);
    }
}

/// 配置读不出来时，语言还没解析成功——退回"忽略配置文件"的解析结果来选文案，
/// 好让中文用户看到中文的报错。
fn annotate_config_error(
    err: anyhow::Error,
    flag_lang: Option<&str>,
    system_locale: Option<&str>,
) -> anyhow::Error {
    let text = fallback_text(flag_lang, system_locale);
    err.context(text.errors.config_parse.to_string())
}

fn render_lang_error(err: LangError, system_locale: Option<&str>) -> anyhow::Error {
    let text = fallback_text(None, system_locale);
    lang_error(&text, err)
}

/// 不支持的语种：错误信息必须列出受支持的取值，只说"不支持"等于让用户去猜。
fn lang_error(text: &Text, err: LangError) -> anyhow::Error {
    match err {
        LangError::Unknown(value) => anyhow::anyhow!(
            "{}: {value} ({} / {})",
            text.errors.lang_unknown,
            Lang::En,
            Lang::ZhHans,
        ),
    }
}

/// 语言本身出问题时的兜底文案：只信任还没出错的来源。
fn fallback_text(flag_lang: Option<&str>, system_locale: Option<&str>) -> Text {
    let lang = flag_lang
        .and_then(|v| lang::parse_explicit(v).ok())
        .or_else(|| system_locale.map(lang::from_system_tag))
        .unwrap_or(Lang::En);
    copy::text(lang)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn bare_invocation_is_the_checkup_not_a_subcommand() {
        // `preflight` 裸敲必须直接体检，不能要求用户记一个子命令。
        let cli = Cli::try_parse_from(["preflight"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn there_is_no_plaintext_key_flag() {
        // 明文 flag 会进 shell history 与 `ps`。这条测试锁住那个"顺手加一个"的冲动。
        assert!(Cli::try_parse_from(["preflight", "--proxycheck-key", "secret"]).is_err());
        assert!(
            Cli::try_parse_from(["preflight", "config", "set", "proxycheck-key", "secret"])
                .is_err()
        );
    }

    #[test]
    fn config_set_only_accepts_whitelisted_keys() {
        assert!(Cli::try_parse_from(["preflight", "config", "set", "proxycheck-key"]).is_ok());
        assert!(Cli::try_parse_from(["preflight", "config", "set", "risk-threshold"]).is_err());
        // 判级阈值与检测项开关永远不可配（docs/verdict.md §8）。
        assert!(
            Cli::try_parse_from(["preflight", "config", "set", "risk-threshold", "80"]).is_err()
        );
    }

    #[test]
    fn config_set_covers_every_whitelisted_key() {
        for args in [
            ["config", "set", "language", "zh-hans"],
            ["config", "set", "timeout", "20"],
            ["config", "set", "no-color", "true"],
        ] {
            let argv = ["preflight"].into_iter().chain(args);
            assert!(Cli::try_parse_from(argv).is_ok(), "{args:?} 应被接受");
        }
    }

    #[test]
    fn non_secret_keys_require_a_value() {
        // 少给值时报错，而不是把键当值悄悄写进去。
        assert!(Cli::try_parse_from(["preflight", "config", "set", "language"]).is_err());
        assert!(Cli::try_parse_from(["preflight", "config", "set", "timeout"]).is_err());
        assert!(Cli::try_parse_from(["preflight", "config", "set", "no-color"]).is_err());
    }

    #[test]
    fn timeout_is_bounded_and_no_color_is_a_bool() {
        // 0 秒必然失败，几千秒看起来像卡死——两头都挡在解析层，非法值进不了配置文件。
        assert!(Cli::try_parse_from(["preflight", "config", "set", "timeout", "0"]).is_err());
        assert!(Cli::try_parse_from(["preflight", "config", "set", "timeout", "121"]).is_err());
        assert!(Cli::try_parse_from(["preflight", "config", "set", "timeout", "1"]).is_ok());
        assert!(Cli::try_parse_from(["preflight", "config", "set", "timeout", "120"]).is_ok());
        assert!(Cli::try_parse_from(["preflight", "config", "set", "no-color", "yes"]).is_err());
    }

    #[test]
    fn config_get_and_unset_accept_every_whitelisted_key() {
        for key in ["language", "proxycheck-key", "timeout", "no-color"] {
            assert!(Cli::try_parse_from(["preflight", "config", "get", key]).is_ok());
            assert!(Cli::try_parse_from(["preflight", "config", "unset", key]).is_ok());
        }
        assert!(Cli::try_parse_from(["preflight", "config", "unset", "risk-threshold"]).is_err());
    }

    #[test]
    fn config_list_is_a_valid_subcommand() {
        assert!(Cli::try_parse_from(["preflight", "config", "list"]).is_ok());
    }

    #[test]
    fn dns_subcommand_is_accepted() {
        assert!(Cli::try_parse_from(["preflight", "dns"]).is_ok());
        let cli = Cli::try_parse_from(["preflight", "dns"]).unwrap();
        match cli.command {
            Some(Command::Dns { check }) => assert!(!check),
            _ => panic!("expected Dns command"),
        }
    }

    #[test]
    fn dns_check_flag_is_accepted() {
        let cli = Cli::try_parse_from(["preflight", "dns", "--check"]).unwrap();
        match cli.command {
            Some(Command::Dns { check }) => assert!(check),
            _ => panic!("expected Dns command"),
        }
    }

    #[test]
    fn update_subcommand_is_accepted_and_takes_no_flags() {
        let cli = Cli::try_parse_from(["preflight", "update"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Update)));
        // 刻意没有 --check 等旁支（spec 非目标）——加了记得先回 spec。
        assert!(Cli::try_parse_from(["preflight", "update", "--check"]).is_err());
    }

    #[test]
    fn uninstall_subcommand_and_purge_flag_are_accepted() {
        let cli = Cli::try_parse_from(["preflight", "uninstall"]).unwrap();
        match cli.command {
            Some(Command::Uninstall { purge }) => assert!(!purge),
            _ => panic!("expected Uninstall command"),
        }
        let cli = Cli::try_parse_from(["preflight", "uninstall", "--purge"]).unwrap();
        match cli.command {
            Some(Command::Uninstall { purge }) => assert!(purge),
            _ => panic!("expected Uninstall command"),
        }
    }

    #[test]
    fn long_version_carries_the_copyright_line() {
        // `--version` 走 long_version，除版本号外还要带版权行（`-V` 仍是短版本号）。
        // 不用 unwrap_err：那需要 `Cli: Debug`，为一条测试给整个 CLI 派生 Debug 不值。
        let Err(err) = Cli::try_parse_from(["preflight", "--version"]) else {
            panic!("--version 应走 DisplayVersion 的错误路径");
        };
        let copyright = format!(
            "© {} Hangzhou Changye Network Technology Co., Ltd.",
            jiff::Zoned::now().year()
        );
        assert!(
            err.to_string().contains(&copyright),
            "--version 输出应包含版权行「{copyright}」：{err}"
        );
    }

    #[test]
    fn lang_flag_is_accepted_before_and_after_subcommands() {
        assert!(Cli::try_parse_from(["preflight", "--lang", "zh-hans"]).is_ok());
        assert!(Cli::try_parse_from(["preflight", "config", "path", "--lang", "zh-hans"]).is_ok());
    }

    #[test]
    fn unsupported_lang_error_lists_supported_values() {
        // 显式给了不支持的语言（含已删除的 zh-hant / ru / ar）时，
        // 错误信息必须列出受支持的取值，而不是只说"不支持"就完事。
        let err = render_lang_error(LangError::Unknown("ar".to_string()), None);
        let message = err.to_string();
        assert!(message.contains("en"), "错误信息应包含 en：{message}");
        assert!(
            message.contains("zh-hans"),
            "错误信息应包含 zh-hans：{message}"
        );
    }
}
