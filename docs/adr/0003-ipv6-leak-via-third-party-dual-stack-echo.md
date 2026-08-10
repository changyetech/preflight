# IPv6 泄露检测借用第三方双栈回显端点，不自建 v4/v6 子域

Cloudflare 对代理主机名自动生成 AAAA 记录，且官方文档写明只有 Enterprise 账户可以关闭 IPv6 兼容——免费版任何橙云主机名必定是双栈，经典的 `v4.` / `v6.` 分离子域探测在纯 Cloudflare 栈上做不出来。我们决定由浏览器直接 fetch `api.ipify.org`（仅 IPv4）与 `api6.ipify.org`（仅 IPv6）来判定 IPv6 是否启用并取得其出口地址。

## Considered Options

- **自建 `v4.` / `v6.` 子域** — 被上述 IPv6 兼容约束堵死。另有第二重障碍：站点域名 `ipcheck.omnikit.run` 本身已是一级子域，其下再开子域即四级域，而 Universal SSL 只覆盖根域与一级子域，"beyond first-level subdomains, use Total TLS or advanced certificates"。
- **灰云子域 + 自建源站** — 需要 VPS，破坏纯 Cloudflare 架构与零固定成本模型，还要自管证书。
- **零依赖启发式**（只看浏览器连到本站用的是 IPv4 还是 IPv6）— 能判断 IPv6 是否通，但拿不到 IPv6 出口地址，无法比对归属地；而"同时暴露两个不同地区的 IP"正是这项检测的意义所在。保留为 ipify 不可用时的降级方案。

## Consequences

- ipify 无需 API key，官方称可无限量使用；已实测 `api.ipify.org` 的 CORS 响应在浏览器侧可用。
- 浏览器对 CORS 失败与网络失败抛出**相同的不透明 `TypeError`**。因此必须用 v4 端点做对照实验：v4 通而 v6 不通 = 用户确实没有 IPv6；两者皆不通 = 第三方故障，应判「检测失败」而非「无 IPv6」。
- 上线前需在具备 IPv6 的网络环境实测 v6 端点的 CORS 响应头，否则会把有 IPv6 的用户误判为无 IPv6——即把有风险的人判成安全。
