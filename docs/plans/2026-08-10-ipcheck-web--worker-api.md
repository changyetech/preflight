# ipcheck Web — Worker API

- 父计划：[2026-08-10-ipcheck-web.md](./2026-08-10-ipcheck-web.md)
- 实现规格：[docs/specs/2026-08-10-ipcheck-web.md](../specs/2026-08-10-ipcheck-web.md) 第 2.1 / 2.4 / 5 节
- Depends on: `--scaffold`

## 目标

交付 `/api/geo` 与 `/api/risk` 两个接口，含 Turnstile 校验、限流、Durable Object 配额守卫与优雅降级；并把接口契约固化为 `docs/api.md`。

## 范围

**包含**：API 契约文档、两个接口、DO 配额守卫、Turnstile 校验、Rate Limiting 绑定、proxycheck v3 与 StopForumSpam 适配、可替换的地理数据来源。

**不包含**：IPv6 检测（O3 是浏览器侧行为，归 `--ui-panel`）、任何 UI。

## 步骤

1. 写 `docs/api.md` 契约（端点、请求/响应 schema、错误码、降级响应），并在 `CLAUDE.md` 的规格索引中登记
   → verify：`CLAUDE.md` 中存在指向 `docs/api.md` 的链接（项目强制索引规则）
2. 抽象地理数据来源：生产从 `request.cf` 读，测试注入固定值（规格 5.2）
   → verify：单测能在不依赖真实 `request.cf` 的前提下断言 O1 输出
3. 先写 `/api/geo` 的失败测试（字段完整性、缺失字段降级为 null），再实现
   → verify：`pnpm vitest run` 由红转绿
4. 实现 Durable Object 配额守卫（SQLite 后端，单实例，按 UTC 日重置）
   → verify：单测覆盖「未达上限放行」「达上限拒绝」「跨日重置」三种情形
5. 接入 Turnstile 服务端校验
   → verify：单测覆盖「无 token 拒绝」「无效 token 拒绝」「有效 token 放行」
6. 配置 Rate Limiting 绑定作用于 `/api/risk`
   → verify：连续超限请求返回限流响应
7. 实现 proxycheck v3 适配：取 `network.type`、Proxy/VPN/TOR 布尔量、风险分
   → verify：以固定 fixture 断言解析结果；风险分分级 <30/<70/≥70 边界各有一例
8. 实现 StopForumSpam 适配，取滥用收录布尔量
   → verify：fixture 断言「有收录 / 无收录 / 服务不可用」三态
9. 组装 `/api/risk`：Turnstile → 限流 → 配额 → 数据源 → 组装响应；**忽略请求体中的任何 IP 参数，只用来源 IP**
   → verify：单测断言「请求体传入伪造 IP 时，实际查询的仍是来源 IP」——此项不可省略，否则本站会退化为任意 IP 查询代理，proxycheck key 会被白嫖
10. 实现降级：配额耗尽返回明确的「额度已用尽」状态而非错误
    → verify：单测断言该状态码与响应体形状

## 验收标准

- `docs/api.md` 存在且已在 `CLAUDE.md` 索引
- 上述全部单测通过
- 伪造 IP 参数无法影响查询目标（第 9 步）
- proxycheck API key 仅存在于 Worker Secret，不在仓库、不在响应中
- 无任何持久化用户数据的代码路径（ADR-0002 / ADR-0008）
