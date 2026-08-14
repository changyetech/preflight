# 部署手册

- 状态：**约定文档**——按什么顺序把两个产物送出去，以及出事了怎么退回来
- 适用：Preflight Web（Cloudflare Worker + 静态资源）与 Preflight CLI（GitHub Release）

> 本文回答**「怎么操作」**。「有哪些配置项、值从哪来」在 **[configuration.md](./configuration.md)**——同一套步骤不重复写两份，两边互相链接。

---

## 0. 两条发布链，互不相干

| | Preflight Web | Preflight CLI |
|---|---|---|
| 触发 | push `main` 且命中 `web.yml` 的 paths | 推 tag `cli/v*` |
| workflow | `.github/workflows/web.yml` | `.github/workflows/release.yml`（**dist 生成**） |
| 去哪 | Cloudflare Workers | 本仓库的 GitHub Release |
| 版本号 | 无——永远只有「当前部署」 | `cli/Cargo.toml` 的 `version` |
| 回滚 | `wrangler rollback` | 发新的补丁版本 |

**红线：本仓库只有 CLI 发 GitHub Release。** 安装命令里的 `releases/latest` 依赖这条——Web 哪天也发 Release，那个 URL 立刻被串掉。

第三条 workflow `cli.yml` 只是质量门（fmt / clippy / test），不部署任何东西。三者触发条件零交集，见 [configuration.md §3.3](./configuration.md)。

---

## 1. 首次部署前的一次性准备

这些是**人工操作**，做完一次就不用再碰。逐项打勾：

**Cloudflare**
- [ ] Cloudflare 账号下已有 Workers 权限的 API Token
- [ ] `wrangler secret put PROXYCHECK_API_KEY`
- [ ] `wrangler secret put TURNSTILE_SECRET_KEY`
- [ ] 构建环境有 `VITE_TURNSTILE_SITE_KEY`（公开值，进前端 bundle，走构建期环境变量而不是 secret）

**GitHub**（只需要主仓库这一个，没有 tap 仓库——见 §5）
- [ ] `changyetech/preflight` 是 **public**——installer 脚本与二进制都是本仓库的 Release 资产，转私有会让安装通道一起 401
- [ ] 仓库 Secret `CLOUDFLARE_API_TOKEN` 已设（`web.yml` 用）
- [ ] 仓库 Secret `CLOUDFLARE_ACCOUNT_ID` 已设——不给的话 token 一旦关联多个账号，CI 里的 `wrangler deploy` 直接报错退出

`release.yml` **不需要配任何 secret**，用 GitHub 自动注入的 `GITHUB_TOKEN` 就够。

值从哪来见 [configuration.md §1.2 / §3.2](./configuration.md)。

---

## 2. 部署 Web

### 2.1 日常：合进 main 就自动上

`web.yml` 在 push `main` 且改动命中 paths 时跑：`pnpm install` → `lint` → `build` → `vitest run` → `wrangler deploy`。

测试**必须排在 build 之后**——`tests/i18n-routing.test.ts` 靠 `env.ASSETS` 读 `dist/client` 里的真实产物。

`concurrency: web` 且 `cancel-in-progress: false`：连续两次 push 会排队串行执行，不会互相取消到一半留下半个部署。

### 2.2 手工部署

```bash
pnpm deploy    # = pnpm build && wrangler deploy -c dist/preflight/wrangler.json
```

**不要**直接对根 `wrangler.jsonc` 执行 `wrangler deploy`——那会绕过 Vite 产物、改用 wrangler 自己的打包。根配置是**输入**，真正部署用的是 `@cloudflare/vite-plugin` 生成的 `dist/preflight/wrangler.json`。

上线前想先看一眼真实产物在 workerd 里的样子：

```bash
make preview    # 默认 :4173
```

### 2.3 部署后验证

```bash
curl -s https://<域名>/api/geo | head -c 200          # 200 + JSON 信封
curl -sI https://<域名>/zh-hans/ | head -3            # 200
curl -sI https://<域名>/no-such-page | head -3        # 必须是真 404，不是 200
```

最后一条是硬要求：语言由路径决定，未知路径必须真 404，**软 404 是明确的红线**（不做 SPA 回退）。

### 2.4 回滚

```bash
wrangler deployments list                  # 找到上一个好版本的 ID
wrangler rollback <deployment-id>
```

回滚只换 Worker 代码与静态资源，**不动** Secret，也不动 Durable Object 里已有的数据（`QuotaCounter` 只存「日期 + 当日计数」两个标量，回滚后当天配额计数原样保留）。

---

## 3. 发布 CLI

### 3.1 发版前

- [ ] `make check-cli` 通过（fmt + clippy + build + test）
- [ ] `cli/Cargo.toml` 的 `version` 已按 SemVer 提升，`Cargo.lock` 已同步（`cargo check` 会更新）
- [ ] `dist plan --tag=cli/v<版本>` 解析正确、5 个目标齐全

```bash
make check-cli
dist plan --tag=cli/v0.2.0
```

`dist plan` 应当打印 `announcing v0.2.0` 并列出 5 个平台的产物、`preflight-installer.sh`、`preflight-installer.ps1` 与 `preflight.rb`。**打印不出来就别打 tag**——tag 推上去再发现配置错，只能删 tag 重来。

### 3.2 打 tag

```bash
git tag cli/v0.2.0
git push origin cli/v0.2.0
```

**tag 必须是斜杠形式 `cli/v<版本>`。** dist 的解析规则会忽略 `/` 之前非 package 名的前缀，所以这个前缀纯粹是给人看的「这是 CLI 的 tag」标记，与应用叫什么解耦。

推上去之后 `release.yml` 自动：交叉编译 5 个目标 → 生成 `preflight-installer.sh` / `.ps1` → 建 GitHub Release 并上传全部产物。

### 3.3 发版后验证

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/changyetech/preflight/releases/latest/download/preflight-installer.sh | sh

preflight --version          # 版本号与 tag 一致
preflight --json | head      # 跑得起来
```

`releases/latest` 这个 URL 依赖「本仓库只有 CLI 发 Release」那条约束——顺带验证一次它确实指向刚发的版本。

### 3.4 出错了怎么办

| 症状 | 原因 | 怎么办 |
|---|---|---|
| 整个 workflow 没触发 | tag 形式不对 | tag 必须含 `major.minor.patch`。删 tag（本地 + 远端）重打 |
| 交叉编译某个目标失败 | 依赖对该平台不兼容 | 修完提升补丁版本重发。**不要**删 Release 重用同一个 tag——已经装了的用户会拿到内容变了的同名版本 |
| 装出来的版本号不对 | `cli/Cargo.toml` 的 `version` 与 tag 不一致 | dist 会直接报错拒绝发布，改一致后重打 tag |

**CLI 没有回滚**。发出去的 Release 用户可能已经装了，正确做法是**发一个新的补丁版本**。真要撤，才考虑把那个 Release 标为 pre-release 或删掉——但 `releases/latest` 会立刻指向上一个版本，等于把已装用户留在原地。

### 3.5 改 dist 配置

`Cargo.toml` 里 `[workspace.metadata.dist]` 与 `[profile.dist]` 两段**由 `dist init` 整段重写**——手写的中文注释会被吞掉（所以说明都放在文件顶部）。改完必须：

```bash
dist init --yes                  # 同步到 .github/workflows/release.yml
dist plan --tag=cli/v0.2.0       # 复测
```

`.github/workflows/release.yml` 是**生成产物，不要手改**——下次 `dist init` 会覆盖。要改行为就改 `Cargo.toml` 里的配置。

`pr-run-mode = "skip"` 是非默认项，动之前先看理由：dist 默认给 `release.yml` 挂 `on: pull_request` 做 dry-run，那会与 `cli.yml` 的 PR 触发重叠，破坏三条 workflow 零交集的约定。

---

## 4. 还没做完的两项

**自定义域名**：`wrangler.jsonc` 里的 `routes` 目前是**注释状态**。启用步骤写在该文件的注释里（确认 zone 已托管 → 取消注释并填实际 `zone_name` → `pnpm deploy` → `curl -sI` 验证证书）。域名上线后，`Cargo.toml` 的 `homepage` 也应从仓库 URL 换成站点地址。

**Observability**：当前**显式关闭**。官方文档未给出 Workers Logs 是否默认记录客户端 IP 的明确结论，也没找到按字段排除 IP 的配置项——结论确认前，关掉才符合 [ADR-0008](./adr/0008-privacy-informed-consent-upfront.md) 的零留存承诺。**核实清楚之前不要打开。**

---

## 5. 加回 Homebrew 通道

当前**没有** Homebrew 安装方式。`brew install <owner>/<tap>/<formula>` 会被 Homebrew 解析成仓库 `github.com/<owner>/homebrew-<tap>`——`homebrew-` 前缀是命名硬规则，formula 不可能住在主仓库里，所以走 Homebrew 就必然多维护一个仓库，当前不值这个面。

要加回来的话，改动如下（都不难，难的是那个仓库要一直在）：

1. 建 public 仓库 `changyetech/homebrew-tap`，空的就行——formula 由 CI 自动推进去，你不用往里写任何东西
2. 配仓库 Secret `HOMEBREW_TAP_TOKEN`，对该 tap 仓库有 write
3. `Cargo.toml` 的 `[workspace.metadata.dist]`：`installers` 加回 `"homebrew"`，补 `tap = "changyetech/homebrew-tap"` 与 **`publish-jobs = ["homebrew"]`**
   - ⚠️ 只填 `tap` 不填 `publish-jobs`，dist **不会真的去推** formula，发版会静悄悄地少一半，只有一句 WARN
   - `[workspace.package]` 的 `homepage` 是 formula 的必填项，已经在了
4. `dist init --yes` 重生成 → `dist plan --tag=cli/v<版本>` 确认产物里出现 `preflight.rb`
5. 文案三处同步改回：`README.md` 的「安装 CLI」、`src/locales/en.ts` 与 `zh-hans.ts` 的 `actions.installCommand`，以及钉住它的 `tests/copy.test.ts`（其中有一条专门断言「不得出现 brew」，要一并删掉）

**主仓库仍然必须 public**，这一条与 Homebrew 无关：tap 里只有那个 `.rb` 文件，二进制始终躺在主仓库的 Release 里，formula 的 `url` 指向的就是它。
