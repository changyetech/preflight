//! `preflight update` 集成测试：把真实二进制拷进临时目录里跑，
//! `PREFLIGHT_UPDATE_BASE` 指向测试内起的本地假 HTTP 服务——全离线，
//! 不碰真实 GitHub，也不碰真实环境（XDG_CONFIG_HOME 指进临时空间）。
//!
//! 只跑 Unix：用例要执行假 installer.sh（CI 是 ubuntu）。Windows 的
//! rename/清理路径由 release 的 msvc 目标兜住编译（spec §9）。
#![cfg(unix)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 极简 HTTP 服务：按路径返回预置响应，其余 404。只求能喂饱 ureq 的 GET。
struct Server {
    base: String,
    hits: Arc<AtomicUsize>,
}

impl Server {
    fn serve(routes: Vec<(&'static str, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                counter.fetch_add(1, Ordering::SeqCst);
                // 读到头部结束即可，GET 没有 body。
                let mut request = Vec::new();
                let mut byte = [0u8; 1];
                while !request.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) > 0 {
                    request.push(byte[0]);
                }
                let request = String::from_utf8_lossy(&request);
                let path = request.split_whitespace().nth(1).unwrap_or("");
                let response = match routes.iter().find(|(route, _)| *route == path) {
                    Some((_, body)) => format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    ),
                    None => {
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            .to_string()
                    }
                };
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Self { base, hits }
    }

    fn manifest(version: &str) -> String {
        format!(r#"{{"releases":[{{"app_name":"preflight","app_version":"{version}"}}]}}"#)
    }
}

/// 每个用例独立的临时空间：二进制副本 + 伪造的 receipt（与 uninstall 测试同款）。
struct Sandbox {
    root: PathBuf,
    exe: PathBuf,
}

impl Sandbox {
    fn new(case: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("preflight-update-{case}-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        let bin_dir = root.join("bin");
        let config_dir = root.join("cfg").join("preflight");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();

        let exe = bin_dir.join("preflight");
        std::fs::copy(env!("CARGO_BIN_EXE_preflight"), &exe).unwrap();
        std::fs::write(config_dir.join("preflight-receipt.json"), "{}").unwrap();
        Self { root, exe }
    }

    fn run(&self, server: &Server) -> std::process::Output {
        Command::new(&self.exe)
            // 断言认英文文案，语言固定，不随宿主机环境漂移。
            .args(["update", "--lang", "en"])
            .env("XDG_CONFIG_HOME", self.root.join("cfg"))
            .env("HOME", self.root.join("home"))
            .env("LOCALAPPDATA", self.root.join("cfg"))
            .env("PREFLIGHT_UPDATE_BASE", &server.base)
            // 假 installer 用它定位落盘目录（真 installer 也认这个变量）。
            .env("XDG_BIN_HOME", self.root.join("bin"))
            .output()
            .unwrap()
    }

    fn aside(&self) -> PathBuf {
        self.root.join("bin").join("preflight.old")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

#[test]
fn up_to_date_reports_and_touches_nothing() {
    let sb = Sandbox::new("same-version");
    // 远端与本地同版本——不该下载 installer，更不该动二进制。
    let server = Server::serve(vec![(
        "/dist-manifest.json",
        Server::manifest(env!("CARGO_PKG_VERSION")),
    )]);

    let out = sb.run(&server);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(concat!("Already up to date: v", env!("CARGO_PKG_VERSION"))),
        "stdout: {stdout}"
    );
    assert!(sb.exe.exists());
    assert!(!sb.aside().exists());
    assert_eq!(server.hits.load(Ordering::SeqCst), 1, "只该拉一次 manifest");
}

#[test]
fn newer_release_runs_the_installer_and_replaces_the_binary() {
    let sb = Sandbox::new("upgrade");
    // 假 installer 模仿真 installer 的落盘方式：写临时文件后 mv 到位。
    let installer = "#!/bin/sh\n\
        printf 'updated-binary' > \"$XDG_BIN_HOME/preflight.new\"\n\
        mv \"$XDG_BIN_HOME/preflight.new\" \"$XDG_BIN_HOME/preflight\"\n";
    let server = Server::serve(vec![
        ("/dist-manifest.json", Server::manifest("9.9.9")),
        ("/preflight-installer.sh", installer.to_string()),
    ]);

    let out = sb.run(&server);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(concat!(
            "Updating v",
            env!("CARGO_PKG_VERSION"),
            " → v9.9.9"
        )),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("Updated to v9.9.9"), "stdout: {stdout}");
    assert_eq!(
        std::fs::read(&sb.exe).unwrap(),
        b"updated-binary",
        "原路径上应是 installer 放下的新二进制"
    );
    assert!(!sb.aside().exists(), "成功后 .old 不该残留（Unix）");
}

#[test]
fn failed_installer_rolls_the_old_binary_back() {
    let sb = Sandbox::new("installer-fails");
    let original = std::fs::read(&sb.exe).unwrap();
    let server = Server::serve(vec![
        ("/dist-manifest.json", Server::manifest("9.9.9")),
        ("/preflight-installer.sh", "#!/bin/sh\nexit 1\n".to_string()),
    ]);

    let out = sb.run(&server);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("The installer did not complete"),
        "stderr: {stderr}"
    );
    // 旧二进制是唯一已知可用的版本——必须原样回到原路径。
    assert_eq!(std::fs::read(&sb.exe).unwrap(), original);
    assert!(!sb.aside().exists());
}

#[test]
fn no_receipt_refuses_before_any_network_request() {
    let sb = Sandbox::new("no-receipt");
    std::fs::remove_file(
        sb.root
            .join("cfg")
            .join("preflight")
            .join("preflight-receipt.json"),
    )
    .unwrap();
    let server = Server::serve(vec![(
        "/dist-manifest.json",
        Server::manifest(env!("CARGO_PKG_VERSION")),
    )]);

    let out = sb.run(&server);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not installed by the official installer"),
        "stderr: {stderr}"
    );
    // 守卫在联网之前（spec §4）：一个请求都不该发出去。
    assert_eq!(server.hits.load(Ordering::SeqCst), 0);
}

#[test]
fn unreachable_manifest_fails_with_the_fetch_error() {
    let sb = Sandbox::new("manifest-404");
    let server = Server::serve(vec![]); // 一切 404

    let out = sb.run(&server);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Cannot check for updates"),
        "stderr: {stderr}"
    );
    assert!(sb.exe.exists(), "失败不该动二进制");
}
