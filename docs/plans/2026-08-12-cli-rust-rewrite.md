# 用 Rust 在本仓库重写 ipcheck CLI（父计划）

- 契约：[docs/verdict.md](../verdict.md) · [docs/verdict-cases.json](../verdict-cases.json)
- 决策：[ADR-0010](../adr/0010-verdict-contract-normative-cli-full-implementation.md) · [ADR-0011](../adr/0011-capability-boundary-divergence.md) · [ADR-0012](../adr/0012-cli-direct-third-party-not-worker-api.md)
- 术语：[CONTEXT.md](../../CONTEXT.md)

## 目标

在本仓库内用 Rust 实现 ipcheck CLI，取代已归档的 `ai-ipcheck`。CLI 是判级契约的**全集实现**（9 项全测），与 ipcheck Web 共享 `docs/verdict.md` 与 `docs/verdict-cases.json`。CLI 直连第三方，不消耗网页版的共享配额。

## 范围

**做**：Rust workspace 与 crate、配置系统、i18n、9 项探测、呈现层与 `--json`、两端共吃 golden 向量、CI 三条流水线、多平台发布（二进制 + Homebrew + `cargo install --git`）、网站与文档收尾、`ai-ipcheck` 归档。

**不做**：
- 不发 crates.io
- 不补 `ai-ipcheck` 缺失的 Windows／Linux 系统代理检测（保持平价，留作后续独立改动）
- 不删 147 黑名单检测（仍检测、仍告警，只是不再进综合结论）
- 不给 CLI 加 `ar` 语种（[verdict.md](../verdict.md) 记明理由）
- 不动 ipcheck Web 的功能，除两处：新增 golden 向量测试、`en.ts`/`zh-hans.ts` 加安装区块

## 子计划与执行顺序

```
foundation ──┬──> domain ──┐
             │             ├──> output ──> release
             └──> probes ──┘
```

| 子计划 | 内容 | 依赖 |
|---|---|---|
| [--foundation](./2026-08-12-cli-rust-rewrite--foundation.md) | workspace、配置系统、i18n 框架、Makefile、`cli.yml` | — |
| [--domain](./2026-08-12-cli-rust-rewrite--domain.md) | 判级契约的两端实现 + golden 向量参数化测试 | foundation |
| [--probes](./2026-08-12-cli-rust-rewrite--probes.md) | 9 项探测 | foundation |
| [--output](./2026-08-12-cli-rust-rewrite--output.md) | 呈现层、`--json`、与 `ai-ipcheck` 的平价验收 | domain, probes |
| [--release](./2026-08-12-cli-rust-rewrite--release.md) | dist、tap、`web.yml`、网站、README、归档、删 `refs/` | output |

`domain` 与 `probes` 互不依赖，可并行。

## 全局验收标准

1. `make check`（Web）与 `make check-cli`（Rust）各自独立通过；`make check` **不需要 Rust 工具链**
2. `docs/verdict-cases.json` 的 25 条用例在**两端**全绿；故意改动契约阈值后**两端同时**变红
3. `ipcheck` 与 `ai-ipcheck` 在同一网络环境下，9 项的**探测值一致**、综合结论一致——`docs/verdict-cases.json` 中标记 `aiIpcheckDiverges` 的场景与黑名单命中场景除外
4. proxycheck key 不出现在任何输出、日志、`--json`、panic backtrace 中
5. 配置文件出现白名单外的键时**非零退出**并指明键名
6. `dist plan --tag=cli/v0.1.0` 能正确解析，且不触发 Web 部署工作流
7. 网站首屏可见安装命令，链接指向本仓库

## 未决事项（阻塞 --release，不阻塞其余）

| 事项 | 阻塞什么 |
|---|---|
| GitHub 仓库 owner 与主仓库名 | `web.yml`、安装命令 URL、README |
| `<owner>/homebrew-tap` 仓库 | Homebrew 分发 |
| `HOMEBREW_TAP_TOKEN` secret | tap 推送 |
| `CLOUDFLARE_API_TOKEN` secret | `web.yml` 自动部署 |
| **应用最终名**（当前占位 `ipcheck`） | 死线是第一个 `cli/v*` tag——之后改名要走 formula rename + 安装脚本 URL 变更 + 已装用户命令失效 |
