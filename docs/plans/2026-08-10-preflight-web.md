# ipcheck Web 实施计划（父计划）

- 日期：2026-08-10
- 实现规格：[docs/specs/2026-08-10-preflight-web.md](../specs/2026-08-10-preflight-web.md)
- 相关决策：[ADR-0001](../adr/0001-web-as-cli-frontend-not-replacement.md) ~ [ADR-0008](../adr/0008-privacy-informed-consent-upfront.md)

## 概览

把 ipcheck Web 从空脚手架建到可上线：单个 Cloudflare Worker 托管 React 前端与 API，完成 4 项可在线检测、两段式综合结论与覆盖度呈现，5 项仅 CLI 项以灰卡引导安装。

## 子计划与执行顺序

| 顺序 | 子计划 | 交付 | 依赖 |
|---|---|---|---|
| 1 | [`--scaffold`](./2026-08-10-preflight-web--scaffold.md) | 可部署到 `ipcheck.omnikit.run` 的工程骨架与测试基建 | — |
| 2 | [`--worker-api`](./2026-08-10-preflight-web--worker-api.md) | `docs/api.md` 契约、`/api/geo`、`/api/risk`、DO 配额守卫、Turnstile、限流 | scaffold |
| 3 | [`--ui-panel`](./2026-08-10-preflight-web--ui-panel.md) | 首屏结论区、9 张检测卡、五态、覆盖度三分、两段式结论 | scaffold + `docs/api.md` 契约 |
| 4 | [`--content`](./2026-08-10-preflight-web--content.md) | 落地内容、功能对照表、i18n、页脚隐私声明 | ui-panel |

**2 与 3 可并行**：ui-panel 依赖的是 `docs/api.md` 这份契约，而非 worker-api 的完工。契约一落定，前端即可对 mock 开工。

## 全局约束

- 每个子计划先写失败测试再写实现（项目 TDD 要求）
- 任何呈现综合结论的地方必须同时呈现覆盖度（ADR-0004）
- 任何新增第三方调用必须在触发它的控件上标注（ADR-0008）
- 不得引入 KV / D1 或任何持久化用户数据的代码路径（ADR-0002 / ADR-0008）

## 上线前阻塞项

规格第 9 节第 3 条——Cloudflare Workers Observability 是否记录客户端 IP 及其关闭方式——未验证前不得上线，否则 ADR-0008 的零留存承诺无法兑现。归属 `--scaffold`。

## 验收（父）

全部子计划验收通过，且规格第 8 节 10 条验收标准逐条实测通过。
