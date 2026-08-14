//! `preflight uninstall` 集成测试：把真实二进制拷进临时目录里跑（它会删除自己），
//! receipt 与配置经 XDG_CONFIG_HOME 指进同一临时空间，不碰真实环境。

use std::path::PathBuf;
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
        Command::new(&self.exe)
            .args(args)
            // 断言认英文文案，语言固定，不随宿主机环境漂移。
            .args(["--lang", "en"])
            .env("XDG_CONFIG_HOME", self.root.join("cfg"))
            .env("HOME", self.root.join("home"))
            .env_remove("PREFLIGHT_CONFIG")
            .output()
            .unwrap()
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
