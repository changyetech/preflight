# 配置清单

- 状态：**约定文档**——本仓库跑起来、部署出去所需的全部配置。新增任何配置项必须登记在此
- 适用：Preflight Web（Cloudflare Worker + 静态资源）与 Preflight CLI（Rust）

> **红线：本文出现的所有 secret 都不入库。** 仓库里只有占位（`.env.example`）与引用（`${{ secrets.* }}`），真值走 `wrangler secret put`、GitHub Secrets 或用户本机的配置文件（[ADR-0008](./adr/0008-privacy-informed-consent-upfront.md)）。

> 本文回答**「有哪些配置项、值从哪来」**。「按什么顺序部署、出事怎么退回来」在 **[deployment.md](./deployment.md)**。

---

## 0. 总览

| 配置项 | 类型 | 配在哪 | 不配的后果 |
|---|---|---|---|
| `VITE_TURNSTILE_SITE_KEY` | 公开值 | 构建期环境变量（本地 `.env` / CI 用 GitHub Variable，§3.2） | 网页版 O4（IP 风险）不可用，卡片直接说明原因 |
| `TURNSTILE_SECRET_KEY` | **secret** | Worker Secret | `/api/risk` 全部返回 403 / 2010 |
| `PROXYCHECK_API_KEY` | **secret** | Worker Secret | `/api/risk` 全部返回 500 / 5001 |
| `QUOTA` | 绑定 | `wrangler.jsonc` | 部署失败 |
| `RISK_RATE_LIMITER` | 绑定 | `wrangler.jsonc` | 部署失败 |
| `ASSETS` | 绑定 | `wrangler.jsonc` | 静态资源 404 |
| `CLOUDFLARE_API_TOKEN` | **secret** | GitHub Secret | `web.yml` 部署步骤失败 |
| `CLOUDFLARE_ACCOUNT_ID` | 标识符 | GitHub Secret | token 关联多个账号时 `web.yml` 部署失败 |
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

`pnpm deploy`，或合进 `main` 让 `web.yml` 自动部署。**不要**直接对根 `wrangler.jsonc` 执行 `wrangler deploy`——那会绕过 Vite 产物、改用 wrangler 自己的打包。根配置是**输入**，真正部署用的是 `@cloudflare/vite-plugin` 生成的 `dist/preflight/wrangler.json`。

完整步骤、部署后验证与回滚见 [deployment.md §2](./deployment.md)；自定义域名与 observability 这两项待人工完成的收尾见 [deployment.md §4](./deployment.md)。

---

## 3. GitHub

### 3.1 只需要一个仓库

| 仓库 | 用途 | 可见性 |
|---|---|---|
| `changyetech/preflight` | 主仓库，Web 与 CLI 都在这儿 | **必须 public**，理由见下 |

**必须 public**：installer 脚本与预编译二进制都是主仓库的 Release 资产，转私有会让三条安装通道一起断——`installer.sh` 连脚本本身都取不到（401）、`cargo install --git` 要 git 凭证、Releases 页面直接看不见。

**没有 Homebrew tap 仓库。** `brew install <owner>/<tap>/<formula>` 会被解析成 `github.com/<owner>/homebrew-<tap>`——`homebrew-` 前缀是命名硬规则，formula 不可能住在主仓库里，所以走 Homebrew 就必然多一个仓库。当前不值这个维护面，`installers` 里只留了 shell 与 powershell。要加回来见 [deployment.md §5](./deployment.md)。

### 3.2 Secrets 与 Variables

| Secret | 给谁用 | 权限 |
|---|---|---|
| `CLOUDFLARE_API_TOKEN` | `web.yml` 的部署步骤 | Workers 部署权限 |
| `CLOUDFLARE_ACCOUNT_ID` | 同上 | 不是凭证，是账号标识符 |

| Variable（Settings → Secrets and variables → Actions → **Variables**） | 给谁用 | 说明 |
|---|---|---|
| `VITE_TURNSTILE_SITE_KEY` | `web.yml` 的 build 步骤 | **构建期**变量，Vite 静态内联进前端 bundle。公开值（本来就会出现在 bundle 里），所以走 Variables 而非 Secrets。**在 Cloudflare Dashboard 给 Worker 设运行时变量是无效的**——前端读不到，且每次 `wrangler deploy` 还会把 Dashboard 手工加的变量清掉 |

`release.yml` **不需要任何 secret**——它只往本仓库发 Release，用 GitHub 自动注入的 `GITHUB_TOKEN` 就够了。（加回 Homebrew 通道的话会多出一个 `HOMEBREW_TAP_TOKEN`，见 [deployment.md §5](./deployment.md)。）

`CLOUDFLARE_ACCOUNT_ID` **必须显式给**。不给的话 wrangler 会去问 API「这个 token 能访问哪些账号」，恰好一个时能自动推断出来，多于一个就在非交互的 CI 里直接报错退出——也就是说不配它，CI 能不能跑取决于账号数量哪天变没变。

它按理不算 secret（没有 token，拿到 account ID 什么也做不了），仍走 Secret 而不是写进 `wrangler.jsonc`：主仓库是 public，没必要公开账号指纹；而且 `wrangler.jsonc` 同时被 vitest 读，能不动就不动。

> **secret 没设时 GitHub 会把 `${{ secrets.X }}` 展开成空字符串，而不是不注入这个变量。** wrangler 判断环境变量用的是 `variableName in process.env`（不是判真假），空串照样算「已设」——于是它会拿着空的 account id 去调 API，报出来的错跟真正的原因毫无关系。`web.yml` 因此在部署前有一步 `Check deploy secrets` 显式挡空值。**新增任何走 `env:` 注入的 secret，都要一并加进那步**，否则又会退回到「错误信息指向错的地方」。

### 3.3 三条 workflow，触发条件零交集

| workflow | 触发 | 干什么 |
|---|---|---|
| `web.yml` | push main + paths（`src/**`、`worker/**`、`docs/verdict-cases.json` 等） | lint → build → test → 部署 |
| `cli.yml` | PR/push + paths（`cli/**`、`Cargo.*`、`docs/verdict-cases.json`） | fmt → clippy → test |
| `release.yml`（**dist 生成，不要手改**） | tag `cli/v*` | 多平台交叉编译 → 生成 installer → 发 GitHub Release |

`docs/verdict-cases.json`、`docs/country-codes.json`、`docs/dns-servers.json` 三份**刻意同时出现在前两条的 paths 里**——它们是两端共吃的数据，改了必须两端同时验证（[verdict.md §7](./verdict.md)）。

`release.yml` 的 `on:` 只有 `push.tags`。dist 默认还会挂 `on: pull_request` 做 dry-run，那会与 `cli.yml` 的 PR 触发重叠，所以配置里用 `pr-run-mode = "skip"` 关掉了——**零交集是靠这个配置项维持的**，重跑 `dist init` 前先确认它还在。

> `web.yml` 的 paths 在 `push` 下写了一遍；`cli.yml` 因为同时挂 `push` 与 `pull_request`，**同一份 paths 重复了两次**。GitHub Actions 不支持 YAML 锚点，这个重复是被迫的——**改一处必须改两处**。

---

## 4. 发布 CLI 用到的配置

发布**步骤**见 [deployment.md §3](./deployment.md)。这里只登记配置项本身。

| 配置项 | 在哪 | 值 |
|---|---|---|
| `repository` | `Cargo.toml` 的 `[workspace.package]` | `https://github.com/changyetech/preflight`。dist 靠它生成 installer 的下载 URL，**缺了 `dist init` 直接报错** |
| `homepage` | 同上 | 同上。当前非必需（它是给 Homebrew formula 用的），加回 Homebrew 通道时会变成必需，留着省事 |
| `installers` | `[workspace.metadata.dist]` | `["shell", "powershell"]`。**没有 `homebrew`**，理由见 §3.1 |
| `pr-run-mode` | 同上 | `"skip"`。维持三条 workflow 零交集，见 §3.3 |

`[workspace.metadata.dist]` 与 `[profile.dist]` 两段**由 `dist init` 整段重写**，手写注释会被吞——说明都放在 `Cargo.toml` 文件顶部。改完必须重跑 `dist init --yes` 才会同步到 `.github/workflows/release.yml`。

**约束：本仓库只有 CLI 发 GitHub Release**（Web 走 Cloudflare 部署）。安装命令里的 `releases/latest` 依赖这条——Web 一旦也发 Release，那个 URL 就会被串掉。

---

## 5. CLI 的终端用户配置

CLI 零配置可用。以下都是可选项。

**配置文件**：`~/.config/preflight/config.toml`（Unix）／`%APPDATA%\preflight\config.toml`（Windows）。`PREFLIGHT_CONFIG` 可覆盖路径，`preflight config path` 打印实际读的是哪个。`preflight config list` 打印全部键**合并后的生效值**（flag > 环境变量 > 配置文件 > 默认；secret 只报是否已设置，绝不回显）。

**允许的键是白名单**（[verdict.md §8](./verdict.md)），**未知键报错退出**——静默忽略会让拼错的键表现成「配了但没生效」，那是最难查的一类问题：

| 键 | 说明 |
|---|---|
| `language` | `en` / `zh-hans`（其余值报错并说明支持的语种） |
| `proxycheck_key` | 把配额从 100 次/天 提到 1,000 |
| `timeout` | 网络探测超时（秒），默认 10 |
| `no_color` | 关闭彩色输出 |

**禁止**：任何判级阈值、任何检测项开关。用户能配阈值，判级契约就作废了。

**读写**：白名单里的每个键都可以经 `config set` 写入、`config unset` 移回默认，也可以直接编辑配置文件（`config set` 会重新序列化，手写的注释会丢）。

```bash
preflight config set language zh-hans  # en / zh-hans
preflight config set timeout 20        # 1–120 秒，超出范围直接报错
preflight config set no-color true     # true / false
preflight config set proxycheck-key    # 交互式、不回显，写入后权限置 600
preflight config get timeout           # 打印生效值（secret 只报是否已设置）
preflight config unset timeout         # 移除该键，恢复内置默认
```

非法值在**写入前**就被拒绝，配置文件里不会留下半个不合法的配置。写入成功但当前被更高优先级来源（`--lang`、`PROXYCHECK_API_KEY`、`NO_COLOR`）覆盖时，stderr 会提示一行——否则用户会以为「配了没生效」。

`proxycheck-key` 刻意**只有交互式**一条路径：`config set proxycheck-key <KEY>` 在参数解析层就被拒绝，明文 key 会进 shell history，也会出现在 `ps` 的进程列表里。脚本／CI 场景用环境变量 `PROXYCHECK_API_KEY`。

**其他环境变量**：`NO_COLOR`（存在且非空即生效）、`--lang` 之外的语言来源依次是 config → `LC_ALL` / `LC_MESSAGES` / `LANG` → `en`。

---

## 6. 配置就位检查清单

**首次部署与发版的操作清单在 [deployment.md §1](./deployment.md)**，这里只核对本文登记的配置项有没有配全。

**本地开发**
- [ ] `pnpm install` 且 `cp .env.example .env`
- [ ] `make build && make test` 通过
- [ ] `make check-cli` 通过

**Cloudflare**
- [ ] `PROXYCHECK_API_KEY` / `TURNSTILE_SECRET_KEY` 两个 Worker Secret 已设（§2.2）
- [ ] observability 保持关闭，直到 IP 留存问题核实清楚（[deployment.md §4](./deployment.md)）

**GitHub**
- [ ] 主仓库 public（`installer.sh` 与二进制都是它的 Release 资产）
- [ ] `CLOUDFLARE_API_TOKEN` / `CLOUDFLARE_ACCOUNT_ID` 两个 Secret 已设（§3.2）
- [ ] `VITE_TURNSTILE_SITE_KEY` 已设为 repo **Variable**——CI 构建期注入，Worker 运行时设置无效（§3.2）
- [ ] 改 `cli/**` 不触发 `web.yml`，改 `src/**` 不触发 `cli.yml`
- [ ] 改 `docs/verdict-cases.json`、`country-codes.json`、`dns-servers.json` 任一，**前两条都触发**

**CLI 发布**
- [ ] `Cargo.toml` 的 `repository` / `installers` / `pr-run-mode` 都在（§4）
- [ ] `dist plan --tag=cli/v0.2.0` 解析通过

---

## 7. 绝不入库的东西

`PROXYCHECK_API_KEY` · `TURNSTILE_SECRET_KEY` · `CLOUDFLARE_API_TOKEN` · 用户的 `~/.config/preflight/config.toml`

`.env` 已在 `.gitignore` 里。CLI 侧还有一层：`Settings` 手写了 `Debug` 实现，key 在调试输出里显示为 `<set>` 而不是原值——派生的 `Debug` 会在 panic backtrace 里把它原样打出来。
