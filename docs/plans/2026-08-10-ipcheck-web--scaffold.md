# ipcheck Web — 工程骨架

- 父计划：[2026-08-10-ipcheck-web.md](./2026-08-10-ipcheck-web.md)
- 实现规格：[docs/specs/2026-08-10-ipcheck-web.md](../specs/2026-08-10-ipcheck-web.md) 第 5 节
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
