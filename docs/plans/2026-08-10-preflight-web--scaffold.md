# ipcheck Web — 工程骨架

- 父计划：[2026-08-10-preflight-web.md](./2026-08-10-preflight-web.md)
- 实现规格：[docs/specs/2026-08-10-preflight-web.md](../specs/2026-08-10-preflight-web.md) 第 5 节
- Depends on: —

## 目标

建立一个能跑测试、能本地开发、能部署到 `ipcheck.omnikit.run` 的最小骨架，并把三条未决事实中与平台相关的两条验证掉。先把部署路径打通，避免功能写完才发现域名/证书/绑定有坑。

## 范围

**包含**：Vite + React + TypeScript 前端、单 Worker + Static Assets 配置、vitest 测试基建、Makefile 目标、域名与部署、Workers Observability 隐私核查。

**不包含**：任何检测逻辑、任何 API 路由实现、任何 UI 设计。

## 步骤

1. 初始化 Vite + React + TypeScript 工程（包管理器 pnpm）
   → verify：`pnpm build` 产出 `dist/`
2. 配置 `wrangler.jsonc`：单 Worker + `assets` 绑定指向 `dist/`，Worker 入口处理 `/api/*`、其余交给静态资源
   → verify：`wrangler dev` 本地既能开页面、又能命中一个临时 `/api/ping`
3. 接入 vitest + `@cloudflare/vitest-pool-workers`，写一个针对 `/api/ping` 的通过测试与一个针对 404 的通过测试
   → verify：`pnpm vitest run` 全绿
4. 填充 `Makefile` 的 `install / dev / build / test / lint / fmt / clean` 目标，替换现有 TODO 占位
   → verify：`make check` 可跑通
5. 绑定域名 `ipcheck.omnikit.run` 并首次部署
   → verify：`curl -sI https://ipcheck.omnikit.run` 返回 200 且证书有效（确认 Universal SSL 覆盖该一级子域）
6. **验证未决事实 #2**：在 `wrangler dev` 本地模式下打印 `request.cf` 的 `country / city / timezone / asn`，记录是真值还是占位值
   → verify：结论写入本计划的「验证记录」小节
7. **验证未决事实 #3（阻塞上线）**：查明 Cloudflare Workers Observability 默认是否记录客户端 IP，以及关闭方式；按结论配置 `wrangler.jsonc`
   → verify：结论与所采取的配置写入「验证记录」小节；若确认无法关闭，立即回报并暂停后续计划，因为 ADR-0008 的承诺需要重新措辞
8. `.env.example` 补充 `PROXYCHECK_API_KEY` 与 Turnstile 密钥占位（仅占位，真值走 Worker Secret，不入库）
   → verify：`git grep` 确认仓库内无任何真实密钥

## 验收标准

- `make check` 全绿
- `https://ipcheck.omnikit.run` 可访问、证书有效、静态页与 `/api/ping` 均正常
- 未决事实 #2、#3 均有明确结论并记录在案
- 仓库内无真实密钥

## 验证记录

> 执行时填写。未决事实 #2、#3 的结论必须落在这里，规格第 9 节据此更新。

### 步骤 1-4、8：已完成

- `pnpm build` 产出 `dist/`（真实构建产物见验证输出）。
- `wrangler dev` 本地起服务，`GET /api/ping` 返回 `{"ok":true}`（200），首页 `/` 返回 200，未知 `/api/*` 返回 404。
- `pnpm vitest run`：2/2 通过（`tests/worker.test.ts`：/api/ping 通过 + 未知 /api/* 404）。
- `make check`（fmt + lint + test + build）全绿。
- `.env.example` 补充 `PROXYCHECK_API_KEY` / `TURNSTILE_SITE_KEY` / `TURNSTILE_SECRET_KEY` 占位；`git grep` 未发现真实密钥特征串。

### 步骤 5：域名绑定与首次部署 —— 待人工执行

无 Cloudflare 账号凭据，未实际部署。已在 `wrangler.jsonc` 中把 `routes`（custom domain）字段写好并注释，附人工执行步骤（见该文件注释）。人工需要执行：

1. 在 Cloudflare Dashboard 确认 `ipcheck.omnikit.run` 所在 zone 已托管、Universal SSL 已覆盖该一级子域。
2. 取消 `wrangler.jsonc` 中 `routes` 字段的注释（`{ "pattern": "ipcheck.omnikit.run", "custom_domain": true }`），执行 `pnpm wrangler deploy`。
3. 执行 `curl -sI https://ipcheck.omnikit.run`，确认返回 200 且证书有效。

### 未决事实 #2：`wrangler dev` 本地 `request.cf` 是真值还是占位值 —— 已确认：占位值

`wrangler dev`（`pnpm wrangler dev`）启动时终端明确打印：

```
[wrangler:warn] Unable to fetch the `Request.cf` object! Falling back to a default placeholder...
```

结论：本地 `wrangler dev` 默认拿不到真实 `request.cf` 地理字段，落回固定占位值（miniflare 默认值）。这印证了规格 5.2 的设计前提——地理数据来源必须做成可替换抽象，生产读 `request.cf`、测试/本地注入固定值，本任务范围不包含实现该抽象（属于后续 API 任务）。

人工如需拿到真值，可执行 `pnpm wrangler dev --remote`（连真实边缘网络）后重新访问 `/api/ping` 之类的探针，观察 `request.cf` 字段。本任务未执行 `--remote`（会产生真实网络请求/部署行为，且当前无账号凭据）。

### 未决事实 #3：Workers Observability 默认是否记录客户端 IP —— 未确认，已保守关闭

查阅 Cloudflare 官方文档（`developers.cloudflare.com/workers/observability/`、`.../logs/workers-logs/`、`.../logs/logpush/`，以及 `workers_trace_events` Logpush 数据集字段参考）：

- 官方文档确认 Workers Logs 的 invocation log 包含 "Request、Response 及相关 metadata"，且会被 "enriched with information available to Cloudflare in the context of the invocation"，但**未逐字段列出** Request 是否含客户端 IP / `cf-connecting-ip` 等 header。
- `workers_trace_events`（Logpush 数据集）字段参考里 `Event` 字段仅标注为 "Details about the source event"（object），未展开子字段，同样无法确认是否含 IP。
- 找到的唯一明确的"移除 IP"机制是 Managed Transform 的 "Remove visitor IP headers"，但那作用于**发往源站的 HTTP 请求头**，与 Workers Logs/Observability 是否记录 IP 是两回事，不能等价替代。
- 未搜到任何文档描述"按字段排除 IP"配置项适用于 Workers Logs。

**结论：未确认**——无法从官方文档得到"Workers Logs 默认是否记录客户端 IP"的明确结论，也未找到细粒度排除 IP 字段的配置方式。

**已采取的保守配置**：`wrangler.jsonc` 中 `observability.enabled` 设为 `false`，在结论明确前不开启 Observability，避免违反 ADR-0008 的零留存承诺。

**待人工核实**（此为上线阻塞项，ADR-0008 承诺依赖此结论）：

1. 登录 Cloudflare Dashboard → Workers & Pages → 对应 Worker → Observability，实际开启一次，发起几次真实请求，检查 Logs 面板里单条记录是否包含来源 IP（字段名可能是 `clientIp` / `cf-connecting-ip` header / 或 `Event.Request.headers` 里的某个字段）。
2. 若确认包含 IP：查是否有 Log Field / Redaction 设置可关闭该字段（Dashboard 里搜索 "Redact" 或联系 Cloudflare 支持确认）；若无法关闭，需回到 ADR-0008 重新措辞隐私承诺（例如改为"不做二次留存/不外发"而非"不记录"）。
3. 若确认不含 IP：可安全开启 `observability.enabled: true`，更新本文件与 `wrangler.jsonc` 注释。
4. 把最终结论回填本节，并同步更新 `docs/specs/2026-08-10-preflight-web.md` 第 9 节。
