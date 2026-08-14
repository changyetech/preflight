//! `preflight uninstall`：删除二进制自身与 dist 安装器的 install receipt，
//! `--purge` 时连默认配置目录一起删。设计见 docs/specs/2026-08-14-cli-uninstall.md。

use std::path::{Path, PathBuf};

/// dist 安装器写入的 install receipt 路径。纯函数——调用方负责读环境变量，
/// 与 `config::resolve_path` 同一约定。
///
/// 与真实 installer.sh / installer.ps1 的写入逻辑逐行对齐（2026-08-14 核实）：
/// 两平台都先认 `XDG_CONFIG_HOME`；未设时 Windows 落 `%LOCALAPPDATA%`，
/// Unix 落 `~/.config`。返回 `None` 表示连基准目录都推导不出来，按「没有 receipt」处理。
#[allow(dead_code)]
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
