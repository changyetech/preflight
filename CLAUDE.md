# Preflight

## Architecture

This is a **single-repository** project. All code, specifications, contracts, and conventions live in this one repo. Contracts (API, error codes) and project-private conventions are documented under `docs/` and govern the code in the same tree.

## Role
Web application

## Mandatory Specs
<!-- Link only to project-private conventions committed under docs/. Universal conventions come from the `code-conventions` skill at runtime (http-constitution, observability, testing, error-codes) — do not relink them here. -->

## Key Responsibilities
<!-- Define module-specific responsibilities here -->

## Tech Stack

- **前端**：React 19 + TypeScript + Vite 8。多页构建（每个语种一个入口：`index.html` = 英文，`zh-hans/index.html` = 简体中文），不是 SPA——语言由路径决定，未知路径必须真 404，不做 SPA 回退（软 404 是明确的红线）。默认与源语言为英语，两语种（en + zh-hans）见 ADR-0016。
- **后端**：单个 Cloudflare Worker（`worker/index.ts`）。`/api/*` 由 Worker 处理，其余路径交给 Static Assets（`env.ASSETS`）。
- **构建集成**：`@cloudflare/vite-plugin`。开发期 Worker 跑在真实 workerd 里，与前端共用一个 Vite dev server，HMR 与 `/api/*` 同进程可用。
- **有状态资源**：Durable Object `QuotaCounter`（proxycheck 日配额，SQLite 后端，见 ADR-0002）、Rate Limit 绑定 `RISK_RATE_LIMITER`（仅 `/api/risk`）。
- **测试**：Vitest + `@cloudflare/vitest-pool-workers`（miniflare 内跑真实 Worker）。
- **Lint / Format**：oxlint + Prettier。
- **CLI**：Rust（crate 在 `cli/`，与前端 `src/` 隔离）。HTTP 走 `ureq` + rustls + **平台信任库**（目标用户全都开着代理，编译期内置根证书会让 TLS 中间人代理下的探测全挂）；并发用 `std::thread::scope`，**不引 tokio**。发布由 dist 负责，tag 为 `cli/v*`。
- **CLI 直连第三方，不走本站 Worker**（ADR-0012）——避免 CLI 流量吃掉网页版的共享 proxycheck 配额。
- **无数据库、无用户态、不存储任何检测结果**（ADR-0008）。

## Build & Test

统一入口是 `Makefile`（`make help` 列出全部 target）：

| 命令 | 说明 |
|---|---|
| `make dev` | Vite dev server，前端 HMR 与 `/api/*` 同一进程（默认 :5173） |
| `make build` | `tsc -b && vite build` |
| `make preview` | 构建后在 workerd 中预览生产产物（默认 :4173） |
| `make test` | `pnpm vitest run` |
| `make lint` / `make fmt` | oxlint / Prettier |
| `make check` | fmt + lint + build + test（**只管 Web**——改一行 CSS 不该等 Rust 编译） |
| `make cli-build` / `cli-test` / `cli-lint` / `cli-fmt` | CLI 的 cargo 对应命令 |
| `make cli-release` | CLI release 构建（产物在 `target/release/preflight`） |
| `make check-cli` | CLI 的 fmt + clippy + build + test |
| `make check-all` | Web 与 CLI 都跑 |
| `make clean` | 删除 `dist`、`.wrangler` 等产物 |

**构建产物布局**（由 `@cloudflare/vite-plugin` 决定）：

- `dist/client/` — 静态资源（`index.html`、`en/index.html`、`assets/`）
- `dist/preflight/` — Worker bundle 与**生成的** `wrangler.json`（其中的 `assets.directory` 由插件填写）

**部署**：`pnpm deploy`（= build + `wrangler deploy -c dist/preflight/wrangler.json`）。**不要**直接对根 `wrangler.jsonc` 执行 `wrangler deploy`——那会绕过 Vite 产物、改用 wrangler 自己的打包。

**关于 `wrangler.jsonc`**：它是**输入配置**。其中的 `assets.directory`（`./dist/client`）只服务于 vitest-pool-workers——测试直接读这份配置，需要它指向构建产物；真实构建与部署用的是生成的输出配置。因此 `make test` 前需要先 `make build`（`tests/i18n-routing.test.ts` 依赖 `env.ASSETS` 读取真实产物）。

## Specifications & Contracts

This repo documents its own **contracts** and **specs** under `docs/` (contract documents live directly under `docs/`, not in sub-directories):

- Domain contract: 判级规则 → `docs/` — **[docs/verdict.md](docs/verdict.md)**：检测项注册表、信号域、三档判级与三形态、覆盖度、两端差异登记表、CLI 配置键白名单。**Web 与 CLI 两个实现共同的判据**，判级规则只在此文件修改
- Configuration: 仓库所需的全部配置 → `docs/` — **[docs/configuration.md](docs/configuration.md)**：环境变量、Worker Secret 与绑定、GitHub 仓库与 Secrets、三条 workflow 的触发条件、CLI 的用户配置与键白名单、上线检查清单、绝不入库的清单
- Third-party integration reference: proxycheck.io → `docs/` — **[docs/proxycheck.md](docs/proxycheck.md)**：风险分基准分表与官方分档建议、必带的 `p=0`/`tag=0`（`tag=0` 是隐私要求）、配额规则、响应字段、「HTTP 200 也可能不是合法 JSON」这个坑、v2 与 v3 的差异。**带核实日期**——第三方会改
- API interface specifications (endpoints, request/response schemas) → `docs/` — **[docs/api.md](docs/api.md)**：`/api/geo` 与 `/api/risk` 的请求/响应 schema、响应信封、错误码注册表、隐私约束
- Error format and error code registry → `docs/` — 见 [docs/api.md](docs/api.md) 第 1 / 4 节
- Response envelope format, retry/backoff strategies, auth contracts → `docs/` — 见 [docs/api.md](docs/api.md) 第 1 节
- Project-private convention documents → `docs/` (see [Convention Documents](#convention-documents))
- Feature / domain design specs → `docs/specs/`
- Implementation plans → `docs/plans/`

**Rule**: Every spec that governs behavior (API, error codes, conventions, domain contracts) MUST be discoverable from this file (see [Spec Document Index](#spec-document-index-mandatory-maintenance)). Transient feature specs under `docs/specs/` are the exception.

## Development Paradigm: SDD + TDD

Before writing or changing any code, follow the agent coding behavior rules (think-before-coding, simplicity-first, surgical-changes, goal-driven execution, root-cause reasoning) — see the `engineering-guidelines` skill.

### Specification-Driven Development (SDD)

1. Write or update the relevant spec **first** (contracts and conventions under `docs/`; feature/design specs in `docs/specs/`).
2. Get the spec reviewed and approved.
3. Implement against the spec.

### Test-Driven Development (TDD)

1. From the spec, write failing tests.
2. Write the minimum implementation to pass.
3. Refactor while keeping tests green.

For implementation-phase TDD details (AAA structure, naming, mocks, coverage, integration tests), see the `code-conventions` skill.

**All code changes must trace back to a spec document.**

## Authoritative Source: Contracts vs Design Specs

Not every document carries the same authority — distinguish two kinds:

- **Contracts (normative, live)** — API specs, error codes, response envelope, retry policy, auth contracts, and convention documents under `docs/`. The agreed interface and rules, kept in sync with reality. When code deviates, **the code is the defect** — fix the code (or deliberately amend the contract first).
- **Design specs (descriptive, point-in-time)** — feature/domain docs under `docs/specs/` and plans under `docs/plans/`. Written to drive a feature at design time; as logic iterates they drift and go stale.

**Reading vs writing:**

- **Writing** new/changed logic → start from a spec (SDD): update the design spec, then implement.
- **Reading / verifying / "what does the system do today"** → **current code is the source of truth**. A design spec states intent when written, not necessarily current behavior.
- **Spec and code disagree** → never silently trust the spec. For a *design spec*, treat it as drift: verify against code and flag the spec for update. For a *contract*, the opposite default — the contract wins and the code is suspect.

## Implementation Plans

Feature plans live under `docs/plans/`. Each plan declares its goal, scope, dependencies, steps, and acceptance criteria, and links the spec(s) it implements.

**Plan structure:**

1. **Spec first** — Write and approve the design spec in `docs/specs/` (and update the contract docs under `docs/` when the interface changes) before planning implementation.
2. **One plan per feature** — Use a `YYYY-MM-DD-feature.md` filename for discoverability.
3. **Declare dependencies** — A plan MUST link to the spec it implements and state `Depends on: <other-plan>` when sequencing matters.

**Splitting large plans into sub-plans:** When a single plan is too large to review or execute in one pass (multiple phases or independent work streams), split it so each piece is reviewable and mergeable on its own:

1. **Parent plan** — `docs/plans/YYYY-MM-DD-feature.md` with an overview, scope, and links to all sub-plans.
2. **Sub-plans** — `docs/plans/YYYY-MM-DD-feature--<slug>.md` where `<slug>` names the sub-scope (e.g. `--schema`, `--api`, `--ui-list`). Each states its own goal, scope, dependencies, steps, and acceptance criteria.
3. **Order** — The parent plan records the recommended execution order; sub-plans declare `Depends on: <sub-plan-slug>` when sequencing matters.
4. **Don't over-split** — Keep each sub-plan a meaningful, self-contained unit of work; if a split only produces trivial fragments, keep it as one plan.

**Example:**

```
docs/specs/2026-06-01-user-management.md              ← design spec
docs/plans/2026-06-01-user-management.md              ← parent overview
docs/plans/2026-06-01-user-management--schema.md      ← data layer
docs/plans/2026-06-01-user-management--api.md         ← API + handlers; Depends on schema
docs/plans/2026-06-01-user-management--ui-list.md     ← list UI; Depends on api
```

## Domain-Driven Design (DDD)

This project follows DDD principles:

- **Aggregate Roots** must be clearly identified in both specs and code. Each bounded context has explicit aggregate roots.
- **Bounded Contexts** are delineated within this repo. Cross-context communication happens only through well-defined interfaces (as specified under `docs/`), not by reaching into another context's internals.
- **Ubiquitous Language** is defined here and used consistently across specs and code.

### Core Domain Concepts

<!-- Define project core domain concepts here (aggregate roots, value objects, etc.) -->

## Conventions

### Convention Documents

Universal cross-cutting conventions (HTTP/API design, observability, testing, commit messages, error codes, language-specific rules) are **not** duplicated here — reference the `code-conventions` skill at runtime. Project-private conventions are documented under `docs/`; add an index entry here when one is added.

### Spec Document Index (Mandatory Maintenance)

**Rule**: Every governing spec (API contracts, error codes, conventions, domain contracts) MUST be referenced in this file. CLAUDE.md is the context-loading entry point — an unreferenced spec is invisible to agents and risks being ignored or contradicted.

**Exception**: Feature/requirement specs under `docs/specs/` are transient and numerous — they do **not** need an index entry.

**How**: Every governing contract or convention document under `docs/` must appear either in the [Specifications & Contracts](#specifications--contracts) bullet list or the Repository Structure tree below, with its actual filename and relative link.

## Repository Structure

A static map of the repo. Contract and convention documents live directly under `docs/`; `docs/specs/` and `docs/plans/` accumulate dated documents over time.

```
preflight/
├── CLAUDE.md          # This file - project rules, conventions, and module guide
├── AGENTS.md          # → @CLAUDE.md
├── CONTEXT.md         # Ubiquitous language glossary (domain terms only, no implementation)
├── Makefile           # 统一命令入口（dev / build / preview / test / lint / check）
├── index.html         # 英文入口（源语言）；zh-hans/index.html 为中文入口（Vite 多页构建）
├── vite.config.ts     # Vite + cloudflare() 插件；多页入口挂在 environments.client
├── wrangler.jsonc     # Worker 输入配置（绑定、DO、限流）；输出配置由插件生成
├── src/               # 前端
│   ├── App.tsx        # 页面骨架：顶部 sticky 导航 + 首屏结论区 + 检测卡 + 落地内容
│   ├── copy.ts        # 语种注册表 LOCALES + 文案聚合（en / zh-hans 均为完整 Copy，无字段级回落）
│   ├── locales/       # 分语种文案：en.ts（源语言，推导 Copy 类型）/ zh-hans.ts（完整译文）
│   ├── usePanel.ts    # 检测面板状态机（O1-O4 的编排）
│   ├── components/    # Card / Verdict / Landing / LangSwitch
│   ├── domain/        # 纯逻辑：结论判级、覆盖度、时区比对、IPv6、对照表
│   └── probes/        # 浏览器直连的第三方探测（ipify）
├── Cargo.toml         # 纯 workspace（members = ["cli"]）+ dist 发布配置
├── cli/               # Preflight CLI（Rust）——判级契约的**全集实现**，10 项全测
│   └── src/
│       ├── domain/    # 纯逻辑：检测项、覆盖度、判级（golden 向量在此消费）
│       ├── probe/     # 10 项探测 + 第三方调用；不碰终端
│       ├── copy/      # 文案：en 为源语言，zh_hans 为完整译文（无字段级回落）
│       ├── render.rs  # 人读报告（参考 Web 的视觉结构，无表格边框）
│       └── json.rs    # `--json`：机器可读，也是平价验收唯一的 oracle
├── worker/            # Cloudflare Worker
│   ├── index.ts       # 入口：/api/* 路由，其余交给 env.ASSETS
│   ├── geo.ts risk.ts # 两个接口的处理器（契约见 docs/api.md）
│   ├── quota.ts       # QuotaCounter Durable Object（ADR-0002）
│   └── proxycheck.ts stopforumspam.ts turnstile.ts  # 第三方调用
├── tests/             # Vitest（vitest-pool-workers，miniflare 内跑真实 Worker）
├── refs/ipcheck/      # Reference: the published ipcheck CLI repo (read-only source material)
└── docs/
    ├── verdict.md     # 判级契约（normative）：Web 与 CLI 共同的判据
    ├── verdict-cases.json  # 判级契约的 golden 向量，两端参数化测试共吃同一份
   ├── country-codes.json  # 英文国家名 → ISO2，O5 判定所需；两端共吃（Web 打进 bundle，CLI include_str!）
    ├── dns-servers.json   # 公共 DNS 清单（IP/品牌/地区/过滤级别），两端共吃（Web bundle / CLI include_str!）
    ├── configuration.md  # 配置清单：环境变量、Secret、绑定、CI、检查清单
    ├── proxycheck.md  # proxycheck.io 集成参考：基准分表、配额、必带参数、已知坑
    ├── api.md         # API contract: /api/geo & /api/risk schemas, error code registry
    ├── adr/           # Architecture Decision Records (0001-*.md, sequentially numbered)
    ├── specs/         # Feature / design specifications (the "what")
    ├── plans/         # Implementation plans (the "how")
    └── ...            # API specs, error codes, convention docs (contracts live directly here)
```
