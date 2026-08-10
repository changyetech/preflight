# 不使用 Workers KV；限流用 Rate Limiting 绑定，第三方 API 配额用 Durable Object

限流要求每次请求写一次计数，而 Workers KV 免费版只有 **1,000 次写入/天**（读 100,000/天），比 Worker 自身的 100,000 请求/天 早 100 倍触顶；KV 又是最终一致的，本就不适合做计数器。我们决定这个项目不引入 KV：机器人防护用 Turnstile，单 IP 高频用 Workers Rate Limiting 绑定，proxycheck.io 的每日配额守卫用单实例 Durable Object。

## Consequences

- Rate Limiting 绑定按 Cloudflare 数据中心分别计数，官方明说其"permissive、最终一致、不应作为精确记账系统"。它能挡单点猛刷，**挡不住全球总量硬上限**。
- 因此"proxycheck 每日 N 次"这类硬配额只能靠全局强一致计数器，即单实例 Durable Object。免费版已支持 SQLite 后端 DO（100,000 请求/天、100,000 行写入/天、5 GB），够用。
- 需要 Durable Object 这一点，反过来影响托管形态选择（见后续 ADR）。
- 若将来要缓存 IP 查询结果，用 Cache API，仍然不要用 KV。
