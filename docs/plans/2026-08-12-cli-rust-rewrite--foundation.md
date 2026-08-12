# CLI 重写 · 骨架、配置与 i18n

- 父计划：[2026-08-12-cli-rust-rewrite.md](./2026-08-12-cli-rust-rewrite.md)
- Depends on: —

## 目标

建立 Rust workspace 与 crate 骨架、配置系统、i18n 框架、Makefile 与 CI 集成，使 `--domain` 与 `--probes` 能并行推进。

## 范围

**做**：workspace 布局、依赖基线、`config` 子命令、语言解析、文案骨架、`cli.yml`、Makefile target。
**不做**：任何探测逻辑、任何判级逻辑、任何渲染（只需一个能跑通的 `main`）。

## 步骤

1. **workspace 布局**
   - 根 `Cargo.toml`：纯 workspace（`[workspace] members = ["cli"]`、`resolver = "2"`），**不含 `[package]`**
   - `cli/Cargo.toml`：`name = "ipcheck"`（占位，见父计划未决事项）、`edition`、`[[bin]] name = "ipcheck"`
   - `.gitignore` 加 `/target`
   - → verify：`cargo build` 通过；`pnpm build` 不受影响

2. **依赖基线**（只引这些，新增依赖需在本计划留痕）
   - `ureq`（HTTP）+ `rustls` + **`platform-verifier`**——用**系统信任库**是硬要求：目标用户全都开着代理，编译期根证书（`rustls-webpki-roots`）会让做 TLS 中间人的代理下所有探测直接失败
   - `serde` / `serde_json`（`--json` 与第三方响应解析）
   - `clap`（derive，子命令用 `Option<Subcommand>`，`None` 时跑体检）
   - `toml`（配置文件）
   - `anyhow`（CLI 不是库，不需要 `thiserror` 的具名错误类型）
   - → verify：`cargo tree` 中无 `openssl-sys`

   **已落地（相对本步初稿的偏离，登记在此）**：
   - TLS 信任源从 `rustls-native-certs` 改为 ureq 的 `platform-verifier` feature（`rustls-platform-verifier`）。目标不变——用系统信任库而非编译期根证书——但这是 ureq 原生支持的路径，且它直接调用 OS 验证器（macOS 走 Security.framework，已在依赖树中确认），会认企业与用户自加的根证书。ureq 以 `--no-default-features` 引入，关掉了默认的 `rustls-webpki-roots`
   - 新增 `rpassword`：交互式读 key 且不回显，没有它就只能提供明文 flag
   - 新增 `sys-locale`：Windows 与无环境变量时的系统 locale 兜底（见步骤 5 的偏离）

3. **配置系统**
   - 优先级：**flag > 环境变量 > 配置文件 > 内置默认**
   - 路径：`~/.config/ipcheck/config.toml`（Unix）／`%APPDATA%\ipcheck\config.toml`（Windows），`IPCHECK_CONFIG` 覆盖
   - **键白名单**（[verdict.md §8](../verdict.md)）：`language`、`proxycheck_key`、`timeout`、`no_color`。**未知键报错退出**并指明键名，不静默忽略
   - → verify：单测覆盖三级优先级各一条；未知键用例断言非零退出码与错误信息含键名

4. **`config` 子命令**
   - `ipcheck config path` — 打印实际读取的配置文件路径
   - `ipcheck config set proxycheck-key` — **交互式、不回显**读入，写入配置文件，权限置 `600`
   - `ipcheck config get proxycheck-key` — 只显示「已设置／未设置」，**不回显 key 本身**
   - **不提供** `--proxycheck-key <KEY>` 明文 flag（会进 shell history 与 `ps`）
   - 环境变量 `PROXYCHECK_API_KEY` 作为脚本／CI 路径
   - → verify：单测断言写入后文件权限为 `600`；断言 `config get` 输出不含 key 内容

5. **i18n 框架**
   - `copy!` 声明宏：从单份 struct 定义同时生成 `Copy`（字段 `&'static str`）与 `PartialCopy`（字段 `Option<&'static str>`）及 merge 实现（`unwrap_or(en.field)`）
   - `cli/src/copy/en.rs` 为**源语言且必须完整**；`zh_hans.rs` 完整；`zh_hant.rs`／`ru.rs` 为 partial 空壳
   - 语言解析：`--lang` > config `language` > `LC_ALL`／`LC_MESSAGES`／`LANG`（Windows 读系统 locale）> `en`
     - **偏离登记**：不能只用 `sys_locale::get_locale()`——它在 macOS 上读 CoreFoundation 的系统 locale，**完全不看 `LC_ALL`／`LANG`**，中文 shell 环境会拿到英文。实现改为**先按 POSIX 顺序自己读这三个环境变量**，读不到再退回 `sys-locale`（覆盖 Windows 与图形环境启动的终端）
     - **显式请求与系统提示的处理不同**：`--lang ar` / config `language = "ar"` 是**报错**（用户明确要了什么，给别的东西不行）；而系统 locale 是 `ar` 时**回落英文不报错**（用户没要求过阿拉伯语，不该因为系统语言就退出）
   - 合法语种：`en`／`zh-hans`／`zh-hant`／`ru`。**`ar` 报错并说明原因**，不静默回落
   - `zh-hant`／`ru` 生效时打一行提示「部分未翻译，缺失项显示英文」
   - → verify：单测覆盖解析顺序、`ar` 报错、partial 回落到 EN
   - 注：本步只建骨架与少量文案；9 项检测文案随 `--probes`／`--output` 填充

6. **Makefile**
   - 新增 `cli-build` / `cli-test` / `cli-lint`（clippy）/ `cli-fmt` / `check-cli`（= fmt + lint + build + test）
   - 新增 `check-all`（= `check` + `check-cli`）
   - **`make check` 保持只管 Web**——改一行 CSS 不该等 Rust 编译
   - `clean` 加 `target`
   - → verify：`make check-cli` 通过；`make check` 在无 Rust 工具链的环境下仍可跑

7. **`.github/workflows/cli.yml`**
   - 触发：PR / push，`paths` 命中 `cli/**`、`Cargo.toml`、`Cargo.lock`、**`docs/verdict-cases.json`**
   - 步骤：`cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`
   - → verify：改动 `docs/verdict-cases.json` 会触发本工作流（`--domain` 完成后端到端验证）
   - 注：`web.yml` 与 `cli-release.yml` 属于 `--release`

## 验收标准

1. `cargo build` 与 `make check-cli` 通过，`cargo tree` 无 `openssl-sys`
2. 配置三级优先级、未知键报错、`600` 权限、`config get` 不回显 key —— 均有测试
3. 语言解析四级顺序、`ar` 报错、partial 回落 —— 均有测试
4. `make check` 不依赖 Rust 工具链
5. `cli.yml` 的 paths 包含 `docs/verdict-cases.json`
