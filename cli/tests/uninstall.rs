//! `preflight uninstall` 集成测试：把真实二进制拷进临时目录里跑（它会删除自己），
//! receipt 与配置经 XDG_CONFIG_HOME 指进同一临时空间，不碰真实环境。

use std::path::{Path, PathBuf};
use std::process::Command;

/// 每个用例独立的临时空间：二进制副本 + 伪造的 receipt 与配置。
struct Sandbox {
    root: PathBuf,
    exe: PathBuf,
    config_dir: PathBuf,
}

impl Sandbox {
    fn new(case: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("preflight-uninstall-{case}-{}", std::process::id()));
        // 上一次失败运行的残留会让断言失真，先清。
        std::fs::remove_dir_all(&root).ok();
        let bin_dir = root.join("bin");
        let config_dir = root.join("cfg").join("preflight");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();

        let exe = bin_dir.join(if cfg!(windows) {
            "preflight.exe"
        } else {
            "preflight"
        });
        std::fs::copy(env!("CARGO_BIN_EXE_preflight"), &exe).unwrap();

        std::fs::write(config_dir.join("preflight-receipt.json"), "{}").unwrap();
        std::fs::write(config_dir.join("config.toml"), "timeout = 20\n").unwrap();
        Self {
            root,
            exe,
            config_dir,
        }
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        self.command(args)
            .env_remove("PREFLIGHT_CONFIG")
            .output()
            .unwrap()
    }

    /// 同 `run`，但显式设置 `PREFLIGHT_CONFIG`——只有验证「显式配置路径不被删」的
    /// 用例需要它，默认路径一律走 `run`。
    fn run_with_explicit_config(&self, args: &[&str], config: &Path) -> std::process::Output {
        self.command(args)
            .env("PREFLIGHT_CONFIG", config)
            .output()
            .unwrap()
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(&self.exe);
        cmd.args(args)
            // 断言认英文文案，语言固定，不随宿主机环境漂移。
            .args(["--lang", "en"])
            .env("XDG_CONFIG_HOME", self.root.join("cfg"))
            .env("HOME", self.root.join("home"))
            // Windows 的 config 走 %APPDATA%、receipt 走 %LOCALAPPDATA%，不覆盖就会删到真实目录。
            .env("APPDATA", self.root.join("cfg"))
            .env("LOCALAPPDATA", self.root.join("cfg"));
        cmd
    }

    fn write_config(&self, body: &str) {
        std::fs::write(self.config_dir.join("config.toml"), body).unwrap();
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

#[test]
fn uninstall_removes_binary_and_receipt_but_keeps_config() {
    let sb = Sandbox::new("default");
    let out = sb.run(&["uninstall"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(!sb.exe.exists(), "二进制应被删除");
    assert!(
        !sb.config_dir.join("preflight-receipt.json").exists(),
        "receipt 应被删除"
    );
    assert!(
        sb.config_dir.join("config.toml").exists(),
        "默认模式必须保留配置"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Removed: "), "输出应含删除清单：{stdout}");
    assert!(
        stdout.contains("Config file kept: "),
        "输出应提示配置仍在：{stdout}"
    );
    assert!(
        stdout.contains("preflight-installer"),
        "输出应含重装命令：{stdout}"
    );
}

#[test]
fn uninstall_purge_also_removes_config_dir() {
    let sb = Sandbox::new("purge");
    let out = sb.run(&["uninstall", "--purge"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(!sb.exe.exists(), "二进制应被删除");
    assert!(!sb.config_dir.exists(), "--purge 应删除整个配置目录");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Config file kept: "),
        "--purge 后不该再提示配置仍在：{stdout}"
    );
}

#[test]
fn uninstall_survives_a_config_file_the_loader_rejects() {
    // 配置文件是白名单（deny_unknown_fields），一个拼错的键会让别的命令全部退出 1。
    // uninstall 正是用来把它删掉的那条路，必须先于配置加载分发，否则用户无路可走。
    let sb = Sandbox::new("broken-config");
    sb.write_config("nope = 1\n");

    let out = sb.run(&["uninstall"]);
    assert!(
        out.status.success(),
        "坏配置不该挡住 uninstall，stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!sb.exe.exists(), "坏配置下也必须删掉二进制");
}

#[test]
fn purge_never_deletes_the_directory_of_an_explicit_config_path() {
    // `resolve_path` 的 PREFLIGHT_CONFIG 分支原样返回用户给的路径（不追加 `preflight`
    // 这一段）。uninstall 若拿它推导「配置目录」，--purge 就会 remove_dir_all 掉
    // 用户的 dotfiles 目录——这条用例锁住那个 `None`。
    let sb = Sandbox::new("explicit-config");
    let dotfiles = sb.root.join("dotfiles");
    std::fs::create_dir_all(&dotfiles).unwrap();
    let explicit = dotfiles.join("preflight.toml");
    std::fs::write(&explicit, "timeout = 20\n").unwrap();

    let out = sb.run_with_explicit_config(&["uninstall", "--purge"], &explicit);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(explicit.exists(), "PREFLIGHT_CONFIG 指定的文件不该被删");
    assert!(dotfiles.exists(), "它所在的目录更不该被 remove_dir_all");
    assert!(!sb.config_dir.exists(), "--purge 仍要删掉默认配置目录");
}

#[test]
fn uninstall_without_receipt_or_config_still_succeeds() {
    // 从源码安装的用户没有 receipt；重复卸载也必须幂等（spec §5）。
    let sb = Sandbox::new("bare");
    std::fs::remove_dir_all(&sb.config_dir).unwrap();
    let out = sb.run(&["uninstall"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!sb.exe.exists(), "没有 receipt 也要能删除二进制");
}
