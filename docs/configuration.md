# 配置清单

- 状态：**约定文档**——本仓库跑起来、部署出去所需的全部配置。新增任何配置项必须登记在此
- 适用：ipcheck Web（Cloudflare Worker + 静态资源）与 ipcheck CLI（Rust）

> **红线：本文出现的所有 secret 都不入库。** 仓库里只有占位（`.env.example`）与引用（`${{ secrets.* }}`），真值走 `wrangler secret put`、GitHub Secrets 或用户本机的配置文件（[ADR-0008](./adr/0008-privacy-informed-consent-upfront.md)）。

---

## 0. 总览

| 配置项 | 类型 | 配在哪 | 不配的后果 |
|---|---|---|---|
| `VITE_TURNSTILE_SITE_KEY` | 公开值 | 构建期环境变量 | 网页版 O4（IP 风险）不可用，卡片直接说明原因 |
| `TURNSTILE_SECRET_KEY` | **secret** | Worker Secret | `/api/risk` 全部返回 403 / 2010 |
| `PROXYCHECK_API_KEY` | **secret** | Worker Secret | `/api/risk` 全部返回 500 / 5001 |
| `QUOTA` | 绑定 | `wrangler.jsonc` | 部署失败 |
| `RISK_RATE_LIMITER` | 绑定 | `wrangler.jsonc` | 部署失败 |
| `ASSETS` | 绑定 | `wrangler.jsonc` | 静态资源 404 |
| `CLOUDFLARE_API_TOKEN` | **secret** | GitHub Secret | `web.yml` 部署步骤失败 |
| `HOMEBREW_TAP_TOKEN` | **secret** | GitHub Secret | CLI 发版时推 tap 失败 |
| CLI 的 `proxycheck_key` | **secret** | 用户本机 | CLI 配额 100 次/天而非 1000 |

**只有 CLI 是零配置可用的**——不配任何东西也能跑完 10 项，只是 proxycheck 走无 key 的 100 次/天。Web 侧则必须配齐 Turnstile 与 proxycheck 才有完整功能。

---

## 1. 本地开发

### 1.1 最小可跑集

```bash
pnpm install
cp .env.example .env      # 全部留空也能跑：O1–O3 正常，O4 会说明未配置
make dev                  # http://localhost:5173
```

`.env` 里的值全空时：网页能开、O1–O3 正常工作，**O4 明确显示「未配置」而不是静默失败**——这是刻意的，宁可说清楚也不发一个注定被拒的请求。

### 1.2 要连通 O4 需要三个值

| 变量 | 从哪来 |
|---|---|
| `VITE_TURNSTILE_SITE_KEY` | Cloudflare Dashboard → Turnstile → 新建站点，取 **Site Key**（公开值，会进前端 bundle） |
| `TURNSTILE_SECRET_KEY` | 同一个站点的 **Secret Key** |
| `PROXYCHECK_API_KEY` | <https://proxycheck.io> 注册（免费档 1,000 次/天，见 [proxycheck.md §2.2](./proxycheck.md)） |

开发期 `TURNSTILE_SECRET_KEY` 与 `PROXYCHECK_API_KEY` 从 `.env` 读即可（`@cloudflare/vite-plugin` 会把它们注入 workerd）；**生产环境必须走 Worker Secret**，见 §2.2。

> Turnstile 提供了固定的测试用 key（Dashboard 的文档里有「always passes / always blocks」几组），本地调试比申请真站点方便。

### 1.3 测试

```bash
make build      # 必须先构建：tests/i18n-routing.test.ts 依赖 env.ASSETS 读 dist/client 的真实产物
make test
```

测试用的 secret 是 `vitest.config.ts` 里写死的占位值（`test-proxycheck-key` 等），**所有第三方调用在测试里都被 stub**，这两个值不会被真的发出去。

### 1.4 CLI 本地开发

```bash
make check-cli    # fmt + clippy + build + test
```

CLI **不需要任何配置**即可开发与运行。可选的 `proxycheck_key` 见 §5。

---

## 2. Cloudflare 部署

### 2.1 绑定（已在 `wrangler.jsonc` 里，无需手工建）

| 绑定 | 类型 | 用途 |
|---|---|---|
| `ASSETS` | Static Assets | `/api/*` 之外的所有路径 |
| `QUOTA` | Durable Object（SQLite） | proxycheck 日配额守卫，上限 **1,000 次/天**（[ADR-0002](./adr/0002-no-kv-for-rate-limiting.md)）。只存「日期 + 当日计数」两个标量，不含 IP |
| `RISK_RATE_LIMITER` | Rate Limiting | 单 IP 限流，10 次/60 秒，**只作用于 `/api/risk`** |

DO 的 migration（`v1` / `new_sqlite_classes`）已在配置里，首次部署自动执行。

### 2.2 Worker Secret（必须手工设，不入库）

```bash
wrangler secret put PROXYCHECK_API_KEY
wrangler secret put TURNSTILE_SECRET_KEY
```

这两个值**不进仓库、不进响应、不进日志**。`VITE_TURNSTILE_SITE_KEY` 是公开值，走构建期环境变量而不是 secret——它本来就会出现在前端 bundle 里。

### 2.3 部署

```bash
pnpm deploy    # = build + wrangler deploy -c dist/ipcheck/wrangler.json
```

**不要**直接对根 `wrangler.jsonc` 执行 `wrangler deploy`——那会绕过 Vite 产物、改用 wrangler 自己的打包。根配置是**输入**，真正部署用的是 `@cloudflare/vite-plugin` 生成的 `dist/ipcheck/wrangler.json`。

### 2.4 待人工完成的两项

**自定义域名**：`wrangler.jsonc` 里的 `routes` 目前是注释状态。启用步骤写在该文件的注释里（确认 zone 已托管 → 取消注释并填实际 `zone_name` → `pnpm deploy` → `curl -sI` 验证证书）。

**Observability**：当前**显式关闭**。原因是官方文档未给出 Workers Logs 是否默认记录客户端 IP 的明确结论，也没找到按字段排除 IP 的配置项——在结论确认前，关掉才符合 [ADR-0008](./adr/0008-privacy-informed-consent-upfront.md) 的零留存承诺。**核实清楚之前不要打开。**

---

## 3. GitHub

### 3.1 需要两个仓库

| 仓库 | 用途 | 可见性 |
|---|---|---|
| `<owner>/ipcheck` | 主仓库 | **必须 public**——`cargo install --git` 与 Homebrew tap 对私有仓库都要凭证，一行安装命令就不成立了 |
| `<owner>/homebrew-tap` | Homebrew formula | public。Homebrew 硬性要求 tap 是**独立仓库** |

### 3.2 Secrets

| Secret | 给谁用 | 权限 |
|---|---|---|
| `CLOUDFLARE_API_TOKEN` | `web.yml` 的部署步骤 | Workers 部署权限 |
| `HOMEBREW_TAP_TOKEN` | dist 生成的 release workflow | 对 tap 仓库有 write |

### 3.3 三条 workflow，触发条件零交集

| workflow | 触发 | 干什么 |
|---|---|---|
| `web.yml` | push main + paths（`src/**`、`worker/**`、`docs/verdict-cases.json` 等） | lint → build → test → 部署 |
| `cli.yml` | PR/push + paths（`cli/**`、`Cargo.*`、`docs/verdict-cases.json`） | fmt → clippy → test |
| dist 生成的 release workflow | tag `cli/v*` | 多平台交叉编译 → Release → 推 tap |

`docs/verdict-cases.json` **刻意同时出现在前两条的 paths 里**——它是判级契约的可执行形式，改它必须两端同时验证（[verdict.md §7](./verdict.md)）。

> `web.yml` 的 paths 在 `push` 下写了一遍；`cli.yml` 因为同时挂 `push` 与 `pull_request`，**同一份 paths 重复了两次**。GitHub Actions 不支持 YAML 锚点，这个重复是被迫的——**改一处必须改两处**。

---

## 4. 发布 CLI

前置：§3 的两个仓库与 `HOMEBREW_TAP_TOKEN` 就位，**且应用最终名已定**。

```bash
dist init                        # 生成 release workflow（首次）
dist plan --tag=cli/v0.2.0       # 必须先实测解析通过
git tag cli/v0.2.0 && git push --tags
```

**tag 用斜杠形式 `cli/v0.2.0`**：dist 文档化的解析规则会忽略 `/` 之前非 package 名的前缀，因此 tag 前缀与应用最终叫什么**完全解耦**。

`Cargo.toml` 的 `[workspace.metadata.dist]` 里 `tap = "OWNER/homebrew-tap"` 目前是**占位**，定了 owner 要改。

**约束：本仓库只有 CLI 发 GitHub Release**（Web 走 Cloudflare 部署）。安装命令里的 `releases/latest` 依赖这条——Web 一旦也发 Release，那个 URL 就会被串掉。

---

## 5. CLI 的终端用户配置

CLI 零配置可用。以下都是可选项。

**配置文件**：`~/.config/ipcheck/config.toml`（Unix）／`%APPDATA%\ipcheck\config.toml`（Windows）。`IPCHECK_CONFIG` 可覆盖路径，`ipcheck config path` 打印实际读的是哪个。`ipcheck config list` 打印全部键**合并后的生效值**（flag > 环境变量 > 配置文件 > 默认；secret 只报是否已设置，绝不回显）。

**允许的键是白名单**（[verdict.md §8](./verdict.md)），**未知键报错退出**——静默忽略会让拼错的键表现成「配了但没生效」，那是最难查的一类问题：

| 键 | 说明 |
|---|---|
| `language` | `en` / `zh-hans`（其余值报错并说明支持的语种） |
| `proxycheck_key` | 把配额从 100 次/天 提到 1,000 |
| `timeout` | 网络探测超时（秒），默认 10 |
| `no_color` | 关闭彩色输出 |

**禁止**：任何判级阈值、任何检测项开关。用户能配阈值，判级契约就作废了。

**设置 key**：

```bash
ipcheck config set proxycheck-key    # 交互式、不回显，写入后权限置 600
```

刻意**不提供** `--proxycheck-key <KEY>` 明文 flag——那会把 secret 写进 shell history，也会出现在 `ps` 的进程列表里。脚本／CI 场景用环境变量 `PROXYCHECK_API_KEY`。

**其他环境变量**：`NO_COLOR`（存在且非空即生效）、`--lang` 之外的语言来源依次是 config → `LC_ALL` / `LC_MESSAGES` / `LANG` → `en`。

---

## 6. 上线检查清单

**本地开发**
- [ ] `pnpm install` 且 `cp .env.example .env`
- [ ] `make build && make test` 通过
- [ ] `make check-cli` 通过

**Cloudflare**
- [ ] `wrangler secret put PROXYCHECK_API_KEY`
- [ ] `wrangler secret put TURNSTILE_SECRET_KEY`
- [ ] 构建环境有 `VITE_TURNSTILE_SITE_KEY`
- [ ] `pnpm deploy` 成功，`/api/geo` 返回 200
- [ ] 自定义域名（`wrangler.jsonc` 的 `routes`）已启用并验证证书
- [ ] observability 保持关闭，直到 IP 留存问题核实清楚

**GitHub**
- [ ] 主仓库 public，`<owner>/homebrew-tap` 已建
- [ ] `CLOUDFLARE_API_TOKEN` / `HOMEBREW_TAP_TOKEN` 已设
- [ ] 改 `cli/**` 不触发 `web.yml`，改 `src/**` 不触发 `cli.yml`
- [ ] 改 `docs/verdict-cases.json` **两条都触发**

**CLI 发布**
- [ ] 应用最终名已定（这是最后的免费改名窗口）
- [ ] `Cargo.toml` 里的 `tap` 占位已替换
- [ ] `dist plan --tag=cli/v0.2.0` 解析通过
- [ ] 发版后 `brew install <owner>/tap/ipcheck` 与 installer 一行命令实测可用

---

## 7. 绝不入库的东西

`PROXYCHECK_API_KEY` · `TURNSTILE_SECRET_KEY` · `CLOUDFLARE_API_TOKEN` · `HOMEBREW_TAP_TOKEN` · 用户的 `~/.config/ipcheck/config.toml`

`.env` 已在 `.gitignore` 里。CLI 侧还有一层：`Settings` 手写了 `Debug` 实现，key 在调试输出里显示为 `<set>` 而不是原值——派生的 `Debug` 会在 panic backtrace 里把它原样打出来。
