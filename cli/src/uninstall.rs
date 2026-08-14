//! `preflight uninstall`：删除二进制自身与 dist 安装器的 install receipt，
//! `--purge` 时连默认配置目录一起删。设计见 docs/specs/2026-08-14-cli-uninstall.md。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::copy::Text;

/// 重装命令是字面 CLI 语法，不随语种变化（与 docs/cli-guide.md §1 同一条命令）。
#[cfg(windows)]
const REINSTALL: &str = "powershell -c \"irm https://github.com/changyetech/preflight/releases/latest/download/preflight-installer.ps1 | iex\"";
#[cfg(not(windows))]
const REINSTALL: &str = "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/changyetech/preflight/releases/latest/download/preflight-installer.sh | sh";

/// 执行卸载。顺序刻意：receipt → （--purge 的）配置目录 → 二进制自身。
/// 二进制放最后，前面任一步失败时它还在，用户修好权限即可重跑（重跑幂等：
/// 已删的项不存在即跳过）。
pub fn run(purge: bool, text: &Text) -> Result<()> {
    let xdg = std::env::var("XDG_CONFIG_HOME").ok();
    let home = std::env::var("HOME").ok();

    let receipt = receipt_path(
        xdg.as_deref(),
        home.as_deref(),
        std::env::var("LOCALAPPDATA").ok().as_deref(),
    );
    // 默认配置路径：复用 config.rs 的推导但**不看 PREFLIGHT_CONFIG**——
    // 用户显式指定的文件不碰，--purge 只清默认目录（spec §3）。
    let config_file = crate::config::resolve_path(
        None,
        xdg.as_deref(),
        home.as_deref(),
        std::env::var("APPDATA").ok().as_deref(),
    );

    // receipt 不存在则静默跳过——从源码 `make cli-release` 安装的用户没有它。
    if let Some(path) = receipt.as_deref().filter(|p| p.exists()) {
        std::fs::remove_file(path)
            .with_context(|| format!("{}: {}", text.errors.uninstall_remove, path.display()))?;
        println!("{}{}", text.uninstall.removed, path.display());
    }

    if purge {
        // 配置目录与 receipt 目录可能是同一个（Unix）也可能不同（Windows：
        // 配置在 %APPDATA%，receipt 在 %LOCALAPPDATA%），去重后都清。
        let mut dirs: Vec<&Path> = Vec::new();
        if let Some(dir) = config_file.as_deref().and_then(Path::parent) {
            dirs.push(dir);
        }
        if let Some(dir) = receipt.as_deref().and_then(Path::parent)
            && !dirs.contains(&dir)
        {
            dirs.push(dir);
        }
        for dir in dirs {
            if dir.exists() {
                std::fs::remove_dir_all(dir).with_context(|| {
                    format!("{}: {}", text.errors.uninstall_remove, dir.display())
                })?;
                println!("{}{}", text.uninstall.removed, dir.display());
            }
        }
    }

    let exe = std::env::current_exe().context("locate current executable")?;
    self_replace::self_delete()
        .with_context(|| format!("{}: {}", text.errors.uninstall_remove, exe.display()))?;
    println!("{}{}", text.uninstall.removed, exe.display());

    // 收尾提示：配置还在哪（仅默认模式且文件确实存在时）、怎么重装。
    if !purge && let Some(path) = config_file.as_deref().filter(|p| p.exists()) {
        println!("{}{}", text.uninstall.config_kept, path.display());
        println!("{}", text.uninstall.config_kept_hint);
    }
    println!("{}{}", text.uninstall.reinstall_hint, REINSTALL);
    Ok(())
}

/// dist 安装器写入的 install receipt 路径。纯函数——调用方负责读环境变量，
/// 与 `config::resolve_path` 同一约定。
///
/// 与真实 installer.sh / installer.ps1 的写入逻辑逐行对齐（2026-08-14 核实）：
/// 两平台都先认 `XDG_CONFIG_HOME`；未设时 Windows 落 `%LOCALAPPDATA%`，
/// Unix 落 `~/.config`。返回 `None` 表示连基准目录都推导不出来，按「没有 receipt」处理。
pub fn receipt_path(
    xdg_config_home: Option<&str>,
    home: Option<&str>,
    local_app_data: Option<&str>,
) -> Option<PathBuf> {
    let base = if let Some(dir) = xdg_config_home.filter(|v| !v.is_empty()) {
        PathBuf::from(dir)
    } else if cfg!(windows) {
        PathBuf::from(local_app_data.filter(|v| !v.is_empty())?)
    } else {
        Path::new(home.filter(|v| !v.is_empty())?).join(".config")
    };
    Some(base.join("preflight").join("preflight-receipt.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_config_home_wins_on_every_platform() {
        // installer.sh 与 installer.ps1 都先认 XDG_CONFIG_HOME，两平台一致。
        assert_eq!(
            receipt_path(Some("/x/cfg"), Some("/home/u"), Some("C:\\lad")),
            Some(PathBuf::from("/x/cfg/preflight/preflight-receipt.json"))
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn unix_falls_back_to_dot_config() {
        assert_eq!(
            receipt_path(None, Some("/home/u"), None),
            Some(PathBuf::from(
                "/home/u/.config/preflight/preflight-receipt.json"
            ))
        );
        // 空串按未设置处理，与 config::resolve_path 同一约定。
        assert_eq!(receipt_path(Some(""), None, None), None);
    }

    #[test]
    #[cfg(windows)]
    fn windows_falls_back_to_local_app_data() {
        let got = receipt_path(None, None, Some("C:\\Users\\u\\AppData\\Local")).unwrap();
        assert!(got.ends_with("preflight\\preflight-receipt.json"));
    }
}
