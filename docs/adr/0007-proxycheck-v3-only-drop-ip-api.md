# IP 类型与风险只用 proxycheck v3，弃用 ip-api

ipcheck CLI 先查 ip-api，命中 proxy/hosting 才升级查 proxycheck，以此节省用户配额。ipcheck Web 中该前置门是冗余的——风险查询本就在按需按钮之后。我们决定弃用 ip-api，单次 proxycheck v3 调用同时取得网络类型、代理检出与风险分。

## Considered Options

- **保留 ip-api 做免费自动项**，让「机房/住宅」能进首屏、给初步结论多一个信号。否决：多换来的那一个信号，代价是一个明确禁止商业使用、且免费版只能明文 HTTP 传输用户 IP 的第三方——对一个主打隐私的站自相矛盾。

## Consequences

- proxycheck v3（2026-06-24 转正）"almost all query flags have been retired and you will now always receive a large and full result"，一次调用即返回 `network.type`（Residential / Business / Wireless / Hosting）、Proxy/VPN/TOR/Scraper 布尔量、风险分与地理数据。
- 注册免费版 1,000 次/天，且官方声明功能不按付费档位区分，故配额守卫的压力远低于最初估算——但 [ADR-0002](./0002-no-kv-for-rate-limiting.md) 的 Durable Object 守卫仍然保留。
- 出口 IP 的地理/ASN 数据继续走 `request.cf`（免费无限），不依赖 proxycheck 的地理段。
- 这是相对 CLI 的一处刻意结构分歧；CLI 侧的懒惰查询策略不应被搬进 Web。
