# CLI 重写 · 发布链、网站与文档收尾

- 父计划：[2026-08-12-cli-rust-rewrite.md](./2026-08-12-cli-rust-rewrite.md)
- Depends on: `--output`

## 目标

把 CLI 发出去（二进制 + Homebrew + `cargo install --git`），网站挂上本仓库与一行安装命令，归档 `ai-ipcheck`，收尾仓库文档。

> 具体怎么配（仓库、Secrets、绑定、workflow 触发条件、检查清单）见 **[docs/configuration.md](../configuration.md)**——那份是持续维护的清单，本计划只记执行顺序。

## 前置（父计划的未决事项，必须先有）

GitHub 仓库 owner 与主仓库名 · `<owner>/homebrew-tap` 仓库 · `HOMEBREW_TAP_TOKEN` · `CLOUDFLARE_API_TOKEN` · **应用最终名的拍板**（死线是第一个 `cli/v*` tag）。

## 步骤

1. **应用定名**
   - 当前占位 `ipcheck`。若要改名，**必须在打第一个 tag 之前改完**：package 名、`[[bin]] name`、Homebrew formula 名、安装脚本 URL、网站文案、README 全部同步
   - → verify：全仓库 grep 占位名，确认无遗漏

2. **dist 配置**
   - `[workspace.metadata.dist]` 写在根 `Cargo.toml`
   - **tag 用 `cli/v0.1.0`（斜杠形式）**：dist 文档化的解析规则会忽略 `/` 之前非 package 名的前缀，因此 tag 前缀**与应用最终名解耦**
   - 目标平台：macOS（arm64 + x86_64）、Linux（x86_64 + arm64）、Windows（x86_64）
   - 可选 `tag-namespace = "cli"`（附带把 workflow 重命名为 `cli-release.yml`），但它是 **experimental**，**必须实测通过才留**
   - → verify：`dist plan --tag=cli/v0.1.0` 正确解析并列出全部目标

3. **`.github/workflows/web.yml`**
   - 触发：push main + `paths` 命中 `src/**`、`worker/**`、`index.html`、`en/**`、`vite.config.ts`、`wrangler.jsonc`、**`docs/verdict-cases.json`**
   - 步骤：`pnpm build` → `wrangler deploy -c dist/ipcheck/wrangler.json`（**不要**对根 `wrangler.jsonc` 直接 deploy）
   - → verify：改动 `cli/**` **不触发**本工作流；改动 `src/**` 触发

4. **`cli-release.yml`**
   - 由 dist 生成，只由 tag `cli/v*` 触发
   - → verify：与 `web.yml`、`cli.yml` 三者触发条件零交集

5. **Homebrew tap**
   - 建 `<owner>/homebrew-tap` 仓库（Homebrew 硬性要求 tap 是独立仓库）
   - `HOMEBREW_TAP_TOKEN` 存为主仓库 secret
   - → verify：一次真实发版后 `brew install <owner>/tap/ipcheck` 可用

6. **安装命令与网站落点**
   - 首屏并列两条：
     - `brew install <owner>/tap/ipcheck`
     - `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/<owner>/ipcheck/releases/latest/download/ipcheck-installer.sh | sh`
   - `cargo install --git` 放文档、**不上首屏**（要求 Rust 工具链，不是「一行安装」的受众）
   - **约束：本仓库只有 CLI 发 Release**，否则 `latest` 会被将来的 Web release 串掉。写进 README
   - 落点：`src/locales/en.ts` 加安装区块即可让 5 个语种都有内容（字段级回落），再补 `zh-hans.ts`
   - → verify：5 个语种页面均可见安装区块；`latest` URL 实测可下载

7. **README.md（新建）**
   - 本仓库目前没有 README，而网站和 `ai-ipcheck` 归档页都要指过来
   - 同时介绍两个产物：ipcheck Web（是什么、地址）与 ipcheck CLI（装什么、9 项能测什么、与网页版的差别）
   - → verify：从归档页与网站点进来都能立刻知道该装什么

8. **`ai-ipcheck` 归档**
   - 原仓库 README 指向本仓库，然后归档（只读）
   - **PyPI 上的 `ai-ipcheck` 不作处理**——不发新版、不 yank
   - → verify：归档页首屏可见新仓库链接

9. **仓库文档收尾**
   - `CLAUDE.md`：结构树加 `cli/`；Tech Stack 加 Rust 与 CLI；Build & Test 加 `cli-*` / `check-cli` / `check-all`；**修掉已漂移的「`index.html` / `en/index.html` 两个入口」**（实际已是 5 语种、英文在根路径）
   - `docs/specs/2026-08-10-preflight-web.md` §3：**掏空改为引用** [docs/verdict.md](../verdict.md)——同一套判级规则不能同时住在一份 normative 和一份 descriptive 文档里
   - → verify：`grep -rn "两个入口" CLAUDE.md` 无命中；spec §3 不再自述判级规则

10. **删除 `refs/ipcheck/`**
    - `--output` 的平价验收通过**之后**才删——在那之前它是唯一的正确性 oracle
    - → verify：删除后 `make check-all` 仍通过；无文档链接指向 `refs/`

## 验收标准

1. `dist plan --tag=cli/v0.1.0` 正确解析；三条 workflow 触发条件零交集
2. 一次真实发版后，Homebrew 与 installer 两条命令实测可用
3. 网站首屏可见安装命令，5 语种均有内容
4. README 存在，同时覆盖 Web 与 CLI
5. `ai-ipcheck` 仓库已归档且指向本仓库
6. `CLAUDE.md` 漂移已修，spec §3 已改为引用
7. `refs/ipcheck/` 已删除且无悬空链接
