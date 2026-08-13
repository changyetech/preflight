# Preflight

面向「对 IP 网络环境敏感的工具」用户的网络环境体检——用之前先确认自己的 IP 环境。一个仓库产出两个实现，共享同一份[判级契约](docs/verdict.md)：

- **Preflight Web** —— 在线体检站，零门槛，能测 6 项
- **Preflight CLI** —— 本机命令行工具，覆盖全部 10 项

按 IP 判断访客的工具与服务对访问环境很敏感，网络环境配置至关重要：IPv6 悄悄泄露真实地址、DNS 走国内服务商暴露位置、出口 IP 风险过高触发风控、系统时区与出口 IP 对不上……开跑之前先扫一眼。

## 安装 CLI

```bash
brew install <owner>/tap/preflight
```

或者：

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/<owner>/preflight/releases/latest/download/preflight-installer.sh | sh
```

有 Rust 工具链的话也可以 `cargo install --git https://github.com/<owner>/preflight`。**不发布到 crates.io。**

> 仓库地址与应用名在第一个 `cli/v*` tag 之前是占位状态，见 [docs/plans/2026-08-12-cli-rust-rewrite.md](docs/plans/2026-08-12-cli-rust-rewrite.md) 的「未决事项」。

## 使用

```bash
preflight                 # 体检
preflight --verbose       # 附带每一项的说明
preflight --json          # 机器可读输出
preflight --lang en       # en / zh-hans
```

可选：注册一个免费的 proxycheck key，把日配额从 100 次提到 1000 次。

```bash
preflight config set proxycheck-key    # 交互式输入，不回显、不进 shell history
```

## 8 个检测项

| ID | 检测项 | Web | CLI |
|---|---|---|---|
| O1 | 出口 IP 与归属 | ✅ | ✅ |
| O2 | 系统时区一致性（对应图形界面应用） | ✅ | ✅ |
| O3 | IPv6 泄露 | ✅ | ✅ |
| O4 | IP 类型与风险 | 按需 | 自动 |
| C1 | 本机真实 IP（国内直连回显） | — | ✅ |
| C2 | 本地 DNS 与 DNS 泄露 | — | ✅ |
| C3 | 代理检测（环境变量 / 系统代理 / TUN） | — | ✅ |
| C4 | `$TZ` 时区一致性（命令行工具认的那个） | — | ✅ |

C1–C4 必须读取本机环境，网页结构性拿不到——这不是网页版偷懒，是能力边界（[ADR-0001](docs/adr/0001-web-as-cli-frontend-not-replacement.md)）。

**两端的结论在少数情形下会不同**，而且是刻意的：网页读不到 `$TZ`、读不到 TUN 状态，归属数据也来自不同的地理库。差异逐条登记在[判级契约第 5 节](docs/verdict.md)，不是 bug。

## 隐私

- **不存储任何检测结果**，两端都没有数据库、没有用户态（[ADR-0008](docs/adr/0008-privacy-informed-consent-upfront.md)）
- CLI **直连**第三方，不经过本站服务器——你的查询不会消耗网页版的共享配额，反过来也一样（[ADR-0012](docs/adr/0012-cli-direct-third-party-not-worker-api.md)）
- proxycheck key 只存在于你本机的配置文件（权限 `600`），绝不出现在输出、日志或任何请求之外的地方
- 代理检测只显示**开关状态**，不显示地址

## 开发

```bash
make help          # 全部 target
make check         # Web：fmt + lint + build + test
make check-cli     # CLI：fmt + clippy + build + test
make check-all     # 两边都跑
```

Web 是 React 19 + Vite + Cloudflare Worker，CLI 是 Rust。两端共享 `docs/verdict.md` 与 `docs/verdict-cases.json`——判级规则改一处，两边的 CI 同时变红。

**配置**（环境变量、Worker Secret、GitHub Secrets、CI、上线检查清单）见 **[docs/configuration.md](docs/configuration.md)**。

约定与架构见 [CLAUDE.md](CLAUDE.md)，术语见 [CONTEXT.md](CONTEXT.md)。

**本仓库只有 CLI 发 Release**（Web 走 Cloudflare 部署），否则安装命令里的 `releases/latest` 会被串掉。

## 前身

CLI 的前身是 Python 写的 `ai-ipcheck`，已归档、不再维护。Rust 版相对它有三处刻意的行为变更，理由见 [ADR-0010](docs/adr/0010-verdict-contract-normative-cli-full-implementation.md) 与
[实现计划](docs/plans/2026-08-12-cli-rust-rewrite--output.md)（其中的端点检测一项已按 [ADR-0013](docs/adr/0013-drop-vendor-endpoint-check.md) 整项移除）。

## License

MIT
