//! `preflight update`：自更新到 GitHub Release 最新版。
//! 机制：拉 `dist-manifest.json` 比版本，有新版则下载并执行**官方 installer**——
//! receipt、安装路径、PATH 行全由 installer 维护，与首次安装同一条路。
//! 设计见 docs/specs/2026-08-14-cli-update.md。

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::copy::Text;

/// Release 资产的下载基址。`releases/latest` 恒指向 CLI 最新版，依赖
/// 「本仓库只有 CLI 发 GitHub Release」这条红线（docs/deployment.md §0）。
/// 刻意不走 api.github.com——匿名限流每小时 60 次，这个端点没有，且与安装
/// 通道同域名，代理环境行为一致。
const DEFAULT_BASE: &str = "https://github.com/changyetech/preflight/releases/latest/download";

#[cfg(windows)]
const INSTALLER: &str = "preflight-installer.ps1";
#[cfg(not(windows))]
const INSTALLER: &str = "preflight-installer.sh";

pub fn run(timeout: Duration, text: &Text) -> Result<()> {
    // receipt 守卫在联网之前：源码安装没有官方更新路径——installer 会把新版装进
    // ~/.local/bin，与旧二进制在 PATH 上互相遮蔽（spec §4）。
    let receipt = crate::uninstall::receipt_path(
        std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
        std::env::var("LOCALAPPDATA").ok().as_deref(),
    );
    if !receipt.is_some_and(|path| path.exists()) {
        bail!("{}", text.errors.update_no_receipt);
    }

    // 测试接缝：集成测试用它指向本地假服务，不写入用户文档（spec §3）。
    let base = std::env::var("PREFLIGHT_UPDATE_BASE").unwrap_or_else(|_| DEFAULT_BASE.to_string());
    let agent = crate::probe::http::agent(timeout);

    let manifest = fetch(&agent, &format!("{base}/dist-manifest.json"))
        .with_context(|| text.errors.update_fetch.to_string())?;
    let latest = latest_version(&manifest)
        .ok_or_else(|| anyhow::anyhow!("{}: dist-manifest.json", text.errors.update_fetch))?;

    let current = env!("CARGO_PKG_VERSION");
    let Some(remote) = parse_version(&latest) else {
        bail!("{}: {latest}", text.errors.update_fetch);
    };
    // 远端更旧（Release 被撤到旧版）也报「已是最新」——不做降级。
    if remote <= parse_version(current).expect("CARGO_PKG_VERSION 恒为 x.y.z") {
        println!("{}{current}", text.update.up_to_date);
        return Ok(());
    }

    println!(
        "{}{current}{}{latest}",
        text.update.updating_prefix, text.update.updating_connector
    );

    let script_body = fetch(&agent, &format!("{base}/{INSTALLER}"))
        .with_context(|| text.errors.update_fetch.to_string())?;
    // 带 pid 的唯一文件名：两个 update 进程并发时不互相覆盖对方正在执行的脚本。
    // 扩展名必须保留——PowerShell 的 -File 只认 .ps1。
    let script = std::env::temp_dir().join(format!("{}-{INSTALLER}", std::process::id()));
    std::fs::write(&script, script_body).context("write installer script")?;

    // rename-aside：把自己改名让出原路径。Windows 是硬需求（运行中的 exe 锁写，
    // installer 的 Copy-Item 会直接失败，而 rename 是允许的）；Unix 上当前
    // installer.sh 用 `mv` 落盘本无冲突，但那是 dist 的实现细节，哪天改成 `cp`
    // 就是 Linux 上的 ETXTBSY——统一做，两平台一个代码路径，且天然获得失败回滚。
    let exe = std::env::current_exe().context("locate current executable")?;
    let aside = aside_path(&exe);
    std::fs::rename(&exe, &aside).context("set the current executable aside")?;

    // stdio 继承：installer 自己的下载进度直接给用户看。
    let status = installer_command(&script).status();
    let _ = std::fs::remove_file(&script);

    match status {
        Ok(status) if status.success() => {
            if exe.exists() {
                // Unix 上 unlink 运行中的 inode 无害；Windows 上仍被本进程锁定，
                // 删除失败则留待下次启动清理（`remove_leftover`）。
                let _ = std::fs::remove_file(&aside);
            } else {
                // installer 成功但落到了别处（用户挪过二进制）——改回来，新旧并存，
                // 不删任何东西（spec §5.4）。
                let _ = std::fs::rename(&aside, &exe);
            }
            println!("{}{latest}", text.update.updated);
            Ok(())
        }
        outcome => {
            // 回滚：installer 失败时旧二进制是唯一已知可用的版本，不能丢。
            let _ = std::fs::remove_file(&exe);
            let _ = std::fs::rename(&aside, &exe);
            match outcome {
                Ok(status) => bail!("{}: {status}", text.errors.update_run),
                Err(err) => Err(err).context(text.errors.update_run.to_string()),
            }
        }
    }
}

/// 上次 update 留下的旧二进制。Windows 上运行中的 exe 删不掉自己，由**下一次**
/// 任意命令启动时清理；失败忽略（可能正有旧进程在跑）。
#[cfg(windows)]
pub fn remove_leftover() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(aside_path(&exe));
    }
}

/// rename-aside 的目标路径：同目录、原文件名 + `.old`（`preflight.exe` → `preflight.exe.old`）。
fn aside_path(exe: &Path) -> PathBuf {
    let mut name = exe.file_name().unwrap_or_default().to_os_string();
    name.push(".old");
    exe.with_file_name(name)
}

#[cfg(not(windows))]
fn installer_command(script: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg(script);
    cmd
}

#[cfg(windows)]
fn installer_command(script: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script);
    cmd
}

/// GET 并读成字符串。与 `probe::http::get_text` 不同，这里的失败要报给用户
/// （带 URL 与原因）——探测层「塌缩成 None」的约定在此不适用。
fn fetch(agent: &ureq::Agent, url: &str) -> Result<String> {
    let mut response = agent.get(url).call().with_context(|| url.to_string())?;
    if response.status() != 200 {
        bail!("{url}: HTTP {}", response.status());
    }
    response
        .body_mut()
        .read_to_string()
        .with_context(|| url.to_string())
}

/// 从 dist-manifest.json 里取 preflight 的版本号。
/// schema 与 cli/v0.2.0 的真实资产核实（2026-08-14）：`releases[].{app_name, app_version}`。
fn latest_version(manifest: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(manifest).ok()?;
    value
        .get("releases")?
        .as_array()?
        .iter()
        .find(|release| release.get("app_name").and_then(|n| n.as_str()) == Some("preflight"))?
        .get("app_version")?
        .as_str()
        .map(str::to_owned)
}

/// 解析 `x.y.z`（容忍 `v` 前缀）。dist 版本恒为纯三段；解析不出来由调用方报错，不猜。
fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let version = version.trim();
    let mut parts = version.strip_prefix('v').unwrap_or(version).split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parses_three_numeric_parts_only() {
        assert_eq!(parse_version("0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_version("v1.10.3"), Some((1, 10, 3)));
        assert_eq!(parse_version(" 1.2.3 "), Some((1, 2, 3)));
        // 预发布号、缺段、多段、空串都不猜——调用方按失败处理。
        assert_eq!(parse_version("1.2.3-rc.1"), None);
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn newer_means_strictly_greater_tuple() {
        // 元组比较即语义比较：先 major 再 minor 再 patch。
        assert!(parse_version("0.3.0") > parse_version("0.2.9"));
        assert!(parse_version("1.0.0") > parse_version("0.99.99"));
        assert!(parse_version("0.2.0") <= parse_version("0.2.0"));
        assert!(parse_version("0.1.9") <= parse_version("0.2.0"));
    }

    #[test]
    fn latest_version_reads_the_real_manifest_shape() {
        // 与 cli/v0.2.0 真实资产同形的最小样本。
        let manifest = r#"{
            "dist_version": "0.32.0",
            "announcement_tag": "cli/v0.2.0",
            "releases": [{ "app_name": "preflight", "app_version": "0.2.0" }]
        }"#;
        assert_eq!(latest_version(manifest), Some("0.2.0".to_string()));
    }

    #[test]
    fn latest_version_rejects_foreign_or_broken_manifests() {
        // 别的 app、缺字段、非 JSON——都返回 None，由调用方报错。
        assert_eq!(
            latest_version(r#"{"releases":[{"app_name":"other","app_version":"9.9.9"}]}"#),
            None
        );
        assert_eq!(latest_version(r#"{"releases":[]}"#), None);
        assert_eq!(latest_version("{}"), None);
        assert_eq!(latest_version("not json"), None);
    }

    #[test]
    fn aside_path_appends_old_to_the_full_file_name() {
        // Windows 上必须是 `preflight.exe.old` 而不是 `preflight.old`——
        // 清理逻辑与生成逻辑共用本函数，命名错配就永远清不掉。
        assert_eq!(
            aside_path(Path::new("/bin/preflight.exe")),
            PathBuf::from("/bin/preflight.exe.old")
        );
        assert_eq!(
            aside_path(Path::new("/bin/preflight")),
            PathBuf::from("/bin/preflight.old")
        );
    }
}
