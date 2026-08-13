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
        "{}\n© {} Preflight",
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
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Print the config file path actually in use
    Path,
    /// List the effective value of every key (secrets are never echoed)
    List,
    /// Set a value interactively (input is not echoed)
    Set { key: SettableKey },
    /// Show whether a value is set (secrets are never echoed)
    Get { key: SettableKey },
}

/// 可通过 `config set` 写入的键。这里只放 secret——其余键直接编辑配置文件即可，
/// 多一条写入路径就多一处优先级歧义。
#[derive(Clone, Copy, ValueEnum)]
enum SettableKey {
    #[value(name = "proxycheck-key")]
    ProxycheckKey,
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
            run_config(action, &text, config_path.as_deref(), file, &settings)?;
            Ok(0)
        }
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

fn run_config(
    action: ConfigAction,
    text: &Text,
    config_path: Option<&std::path::Path>,
    mut file: ConfigFile,
    settings: &Settings,
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

        ConfigAction::Get { key } => match key {
            // 只报状态，绝不回显 key 本身。
            SettableKey::ProxycheckKey => println!(
                "{}",
                if settings.proxycheck_key.is_some() {
                    text.config.key_state_set
                } else {
                    text.config.key_state_unset
                }
            ),
        },

        ConfigAction::Set { key } => match key {
            SettableKey::ProxycheckKey => {
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
                println!("{}", text.config.key_saved);
            }
        },
    }

    Ok(())
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
    }

    #[test]
    fn config_list_is_a_valid_subcommand() {
        assert!(Cli::try_parse_from(["preflight", "config", "list"]).is_ok());
    }

    #[test]
    fn long_version_carries_the_copyright_line() {
        // `--version` 走 long_version，除版本号外还要带版权行（`-V` 仍是短版本号）。
        // 不用 unwrap_err：那需要 `Cli: Debug`，为一条测试给整个 CLI 派生 Debug 不值。
        let Err(err) = Cli::try_parse_from(["preflight", "--version"]) else {
            panic!("--version 应走 DisplayVersion 的错误路径");
        };
        let copyright = format!("© {} Preflight", jiff::Zoned::now().year());
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
