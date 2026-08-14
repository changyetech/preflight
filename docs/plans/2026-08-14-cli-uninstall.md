# `preflight uninstall` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 `preflight uninstall [--purge]` 子命令（自删除二进制 + 清理 install receipt，`--purge` 连配置目录），并更新用户文档。

**Architecture:** 新增 `cli/src/uninstall.rs` 承载全部卸载逻辑（receipt 路径推导为纯函数，可单元测试）；main.rs 只加 clap variant 与一行分发；文案按既有 copy 宏体系加 `UninstallText` 叶子，en / zh-hans 完整填写。自删除用 `self-replace` crate 统一两平台。

**Tech Stack:** Rust（edition 2024，rust-version 1.90）、clap 4 derive、anyhow、self-replace（唯一新依赖）。

**Spec:** [docs/specs/2026-08-14-cli-uninstall.md](../specs/2026-08-14-cli-uninstall.md)（先读一遍，验收标准在 §9）

**Branch:** `feat/cli-uninstall`（已存在，基于 fix/cli-install-path）

## Global Constraints

- 命令一律加 `rtk` 前缀（如 `rtk cargo test`）；cargo 命令在仓库根执行（workspace 会路由到 cli/）。
- 注释用简体中文，风格向现有文件看齐（说约束与为什么，不说下一行干什么）。
- 文案：en 是源语言，zh-hans 必须完整填写——漏字段是编译错误，这是刻意设计，不要用 `Default` 之类绕过。
- 不引 tokio；除 `self-replace` 外不新增任何依赖。
- 退出码沿用现有注册表：成功 `0`，失败走 `main()` 的 `Err` 路径 → `1`（`EXIT_TOOL_FAILURE`）。不新增退出码。
- stdout 只放命令自身结果；错误走 stderr（anyhow 冒泡到 main 统一打印）。
- 质量门：每个任务结束时 `make cli-fmt && make cli-lint` 必须干净。

---

### Task 1: receipt 路径推导（纯函数 + 单元测试）

**Files:**
- Create: `cli/src/uninstall.rs`
- Modify: `cli/src/main.rs`（仅加一行 `mod uninstall;`）

**Interfaces:**
- Consumes: 无（纯函数，参数是环境变量值）
- Produces: `uninstall::receipt_path(xdg_config_home: Option<&str>, home: Option<&str>, local_app_data: Option<&str>) -> Option<PathBuf>` — Task 2 的 `run()` 调用它

**背景（实现者必读）：** dist（cargo-dist）安装器会写一个 install receipt。其路径已与真实 installer.sh / installer.ps1 逐行核实（2026-08-14）：两平台都**先认 `XDG_CONFIG_HOME`**；未设时 Windows 落 `%LOCALAPPDATA%\preflight\`，Unix 落 `~/.config/preflight/`；文件名恒为 `preflight-receipt.json`。风格参照 `cli/src/config.rs` 的 `resolve_path`（纯函数 + 调用方读环境变量，测试不必改进程环境）。

- [ ] **Step 1: 写失败测试**

创建 `cli/src/uninstall.rs`：

```rust
//! `preflight uninstall`：删除二进制自身与 dist 安装器的 install receipt，
//! `--purge` 时连默认配置目录一起删。设计见 docs/specs/2026-08-14-cli-uninstall.md。

use std::path::{Path, PathBuf};

/// dist 安装器写入的 install receipt 路径。纯函数——调用方负责读环境变量，
/// 与 `config::resolve_path` 同一约定。
///
/// 与真实 installer.sh / installer.ps1 的写入逻辑逐行对齐（2026-08-14 核实）：
/// 两平台都先认 `XDG_CONFIG_HOME`；未设时 Windows 落 `%LOCALAPPDATA%`，
/// Unix 落 `~/.config`。返回 `None` 表示连基准目录都推导不出来，按「没有 receipt」处理。
// Task 3 的 run() 接线后，expect 会因 lint 不再触发而报警，届时**必须删除本行**——
// 这是刻意选 expect 而非 allow：接线后忘了摘会被编译器点名。
#[expect(dead_code)]
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
            Some(PathBuf::from("/home/u/.config/preflight/preflight-receipt.json"))
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
```

在 `cli/src/main.rs` 的模块声明区（`mod config;` 那一组，按字母序）加：

```rust
mod uninstall;
```

- [ ] **Step 2: 跑测试确认通过**

先写实现再跑（本任务的"失败态"是文件不存在时的编译错误，无从分步展示）：

```bash
rtk cargo test --package preflight uninstall
```

预期：3 个测试中 2 个通过（`#[cfg(windows)]` 那个在本机不编译）。

- [ ] **Step 3: fmt + clippy**

```bash
make cli-fmt && make cli-lint
```

预期：干净通过。`cli-lint` 是 `clippy -D warnings`，`receipt_path` 暂未被调用靠 Step 1 的 `#[expect(dead_code)]` 压住——Task 3 接线后编译器会反过来要求删掉它。

- [ ] **Step 4: Commit**

```bash
rtk git add cli/src/uninstall.rs cli/src/main.rs
rtk git commit -m "feat(cli): uninstall receipt 路径推导——与真实 installer 逐行对齐"
```

---

### Task 2: 文案 `UninstallText`（en + zh-hans）

**Files:**
- Modify: `cli/src/copy/mod.rs`
- Modify: `cli/src/copy/en.rs`
- Modify: `cli/src/copy/zh_hans.rs`

**Interfaces:**
- Consumes: 既有 `copy_leaf!` / `copy_node!` 宏
- Produces: `text.uninstall.removed / config_kept / config_kept_hint / reinstall_hint`、`text.errors.uninstall_remove` — Task 3 的 `run()` 使用这些字段名

- [ ] **Step 1: 改 mod.rs（这是"失败测试"——en/zh 少字段会编译不过）**

在 `cli/src/copy/mod.rs` 的 `DnsCommandText` 叶子之后加：

```rust
copy_leaf! {
    /// `preflight uninstall` 的文案。路径与重装命令是字面量，
    /// 由调用方裸拼接在前缀之后——标点与空格由取值自带（同 `ErrorText` 约定）。
    UninstallText {
        /// 每删掉一项打一行的前缀，路径由调用方追加。
        removed,
        /// 默认模式收尾：配置文件保留在哪，路径由调用方追加。
        config_kept,
        /// 紧随其后的第二行：不需要就手动删。
        config_kept_hint,
        /// 重装命令的前缀。命令本身是字面 CLI 语法，不随语种变化，写死在 uninstall.rs。
        reinstall_hint,
    }
}
```

`ErrorText` 叶子的 `lang_unknown,` 之后加：

```rust
        /// 卸载时某一项删不掉（权限等）。具体路径由调用方追加在冒号之后。
        uninstall_remove,
```

`Text` 根节点的 `dns_cmd: DnsCommandText,` 之后加：

```rust
        uninstall: UninstallText,
```

- [ ] **Step 2: 跑编译确认失败**

```bash
rtk cargo check --package preflight
```

预期：FAIL——`en.rs` 与 `zh_hans.rs` 的 `Text` 字面量缺 `uninstall` 字段、`ErrorText` 缺 `uninstall_remove`。

- [ ] **Step 3: 补齐两语种**

`cli/src/copy/en.rs`：`use super::{...}` 里加 `UninstallText`；`errors: ErrorText {...}` 里 `lang_unknown` 之后加：

```rust
        uninstall_remove: "Cannot remove",
```

`dns_cmd: DnsCommandText {...},` 字段之后加（保持与 mod.rs 的字段顺序一致）：

```rust
    uninstall: UninstallText {
        removed: "Removed: ",
        config_kept: "Config file kept: ",
        config_kept_hint: "Delete it manually if you no longer need it.",
        reinstall_hint: "To reinstall: ",
    },
```

`cli/src/copy/zh_hans.rs` 同构：`use` 加 `UninstallText`；errors 加：

```rust
        uninstall_remove: "无法删除",
```

尾部加：

```rust
    uninstall: UninstallText {
        removed: "已删除：",
        config_kept: "配置文件已保留：",
        config_kept_hint: "如不再需要，请手动删除。",
        reinstall_hint: "重新安装：",
    },
```

- [ ] **Step 4: 跑测试确认通过**

```bash
rtk cargo test --package preflight copy
```

预期：PASS（含既有的 `zh_hans_is_a_distinct_full_translation`）。

- [ ] **Step 5: fmt + clippy + Commit**

```bash
make cli-fmt && make cli-lint
rtk git add cli/src/copy/
rtk git commit -m "feat(cli): uninstall 文案——en/zh-hans 完整两份"
```

---

### Task 3: 子命令接线 + `run()` 实现（集成测试先行）

**Files:**
- Create: `cli/tests/uninstall.rs`
- Modify: `cli/src/uninstall.rs`（加 `run()`）
- Modify: `cli/src/main.rs`（clap variant + 分发 + 解析测试）
- Modify: `cli/Cargo.toml`（加 `self-replace`）

**Interfaces:**
- Consumes: Task 1 的 `receipt_path`、Task 2 的 `text.uninstall.*` / `text.errors.uninstall_remove`、既有 `config::resolve_path`
- Produces: `uninstall::run(purge: bool, text: &Text) -> anyhow::Result<()>`；clap 的 `Command::Uninstall { purge: bool }`

- [ ] **Step 1: 写失败的集成测试**

创建 `cli/tests/uninstall.rs`：

```rust
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
        let root = std::env::temp_dir().join(format!(
            "preflight-uninstall-{case}-{}",
            std::process::id()
        ));
        // 上一次失败运行的残留会让断言失真，先清。
        std::fs::remove_dir_all(&root).ok();
        let bin_dir = root.join("bin");
        let config_dir = root.join("cfg").join("preflight");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();

        let exe = bin_dir.join(if cfg!(windows) { "preflight.exe" } else { "preflight" });
        std::fs::copy(env!("CARGO_BIN_EXE_preflight"), &exe).unwrap();

        std::fs::write(config_dir.join("preflight-receipt.json"), "{}").unwrap();
        std::fs::write(config_dir.join("config.toml"), "timeout = 20\n").unwrap();
        Self { root, exe, config_dir }
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
```

- [ ] **Step 2: 跑测试确认失败**

```bash
rtk cargo test --package preflight --test uninstall
```

预期：FAIL——`unrecognized subcommand 'uninstall'`（退出码非 0，三个用例全挂）。

- [ ] **Step 3: 加依赖**

```bash
cd cli && cargo add self-replace && cd ..
```

然后在 `cli/Cargo.toml` 给这行依赖加注释（对齐仓库风格）：

```toml
# 卸载时删除运行中的二进制自身：Unix 等价于 unlink，Windows 上处理
# 「运行中 exe 不能删自己」。uv / rye 同款，两平台一个代码路径。
self-replace = "1.5.5"
```

（版本号以 `cargo add` 实际写入的为准，不要手动回退。）

- [ ] **Step 4: clap variant 与分发**

`cli/src/main.rs` 的 `enum Command` 里，`Dns` variant 之后加：

```rust
    /// Remove preflight from this machine
    Uninstall {
        /// Also remove the config directory
        #[arg(long)]
        purge: bool,
    },
```

`run()` 的 `match cli.command` 里，`Some(Command::Dns { .. })` 分支之后加：

```rust
        Some(Command::Uninstall { purge }) => {
            uninstall::run(purge, &text)?;
            Ok(0)
        }
```

`main.rs` 的 `mod tests` 里加解析测试：

```rust
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
```

- [ ] **Step 5: 实现 `run()`**

在 `cli/src/uninstall.rs` 顶部补 use、**删除 `receipt_path` 上的 `#[expect(dead_code)]`**（留着编译器会报「expect 未兑现」），并加实现：

```rust
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
        if let Some(dir) = receipt.as_deref().and_then(Path::parent) {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
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
    if !purge {
        if let Some(path) = config_file.as_deref().filter(|p| p.exists()) {
            println!("{}{}", text.uninstall.config_kept, path.display());
            println!("{}", text.uninstall.config_kept_hint);
        }
    }
    println!("{}{}", text.uninstall.reinstall_hint, REINSTALL);
    Ok(())
}
```

- [ ] **Step 6: 跑测试确认通过**

```bash
rtk cargo test --package preflight
```

预期：全部 PASS（含 3 个集成用例、Task 1 单元测试、既有全量测试）。

- [ ] **Step 7: fmt + clippy + Commit**

```bash
make cli-fmt && make cli-lint
rtk git add cli/Cargo.toml Cargo.lock cli/src/ cli/tests/
rtk git commit -m "feat(cli): preflight uninstall 子命令——自删除+receipt清理，--purge 连配置目录"
```

---

### Task 4: 文档更新 + 全量验收

**Files:**
- Modify: `docs/cli-guide.md`（§1 加卸载小节；§3 命令总览加一行）

**Interfaces:**
- Consumes: Task 3 已落地的命令行为
- Produces: 无（文档终点）

- [ ] **Step 1: §3 命令总览加一行**

`docs/cli-guide.md` §3 的命令代码块里，`preflight config <ACTION>` 之后加：

```
preflight uninstall [--purge]        # 卸载（--purge 连配置一起删）
```

- [ ] **Step 2: §1 末尾（「验证安装」代码块之后、§2 之前）加卸载小节**

````markdown
### 1.1 卸载

```bash
preflight uninstall           # 删除二进制与 install receipt，配置保留
preflight uninstall --purge   # 连配置目录一起删除
```

手动清理（二进制已损坏、或想逐项确认时）：

| 对象 | Linux / macOS | Windows |
|---|---|---|
| 二进制 | 安装器按 `$XDG_BIN_HOME` → `$XDG_DATA_HOME/../bin` → `~/.local/bin` 顺序落盘，通常在 `~/.local/bin/preflight` | PowerShell 安装器的对应目录 |
| install receipt | `${XDG_CONFIG_HOME:-~/.config}/preflight/preflight-receipt.json` | `%LOCALAPPDATA%\preflight\preflight-receipt.json` |
| 配置文件 | `${XDG_CONFIG_HOME:-~/.config}/preflight/config.toml` | `%APPDATA%\preflight\config.toml` |

安装器若曾向 shell rc（`~/.profile` 等）追加过 PATH 行，`uninstall` 不会去改它——留着无害，介意就手动删除该行。
````

- [ ] **Step 3: 对照 spec §9 验收清单逐项核对**

逐条核对 [spec §9](../specs/2026-08-14-cli-uninstall.md)：前五项由 Task 3 的测试覆盖；「删除失败退出码 1」由 `run()?` 冒泡到 `main()` 的 `EXIT_TOOL_FAILURE` 保证（架构既有行为，集成测试不模拟权限失败——tempdir 内制造只读目录在 CI 的 root 环境下不可靠）。

- [ ] **Step 4: 全量质量门 + Windows 目标编译**

```bash
make check-cli
rustup target add x86_64-pc-windows-msvc
rtk cargo check --package preflight --target x86_64-pc-windows-msvc
```

预期：check-cli 全绿；Windows 目标 `cargo check` 通过（只查编译不链接，无需 MSVC 工具链）。

- [ ] **Step 5: Commit**

```bash
rtk git add docs/cli-guide.md
rtk git commit -m "docs(cli-guide): 补卸载章节——uninstall 用法与手动清理兜底"
```
