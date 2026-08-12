# CLI 直连第三方，不走本站 Worker API

ipcheck CLI 与 ipcheck Web 现在同处一个仓库、共享同一份判级契约，复用 `/api/geo` 与 `/api/risk` 看起来是顺理成章的下一步。我们决定**不这么做**：CLI 直连 proxycheck、ipify、StopForumSpam，不经过本站 Worker。

## Considered Options

- **CLI 调用本站 `/api/risk`**。否决：`/api/risk` 消耗的是本站注册的 proxycheck 配额（1,000 次／天，Durable Object 守卫，[ADR-0002](./0002-no-kv-for-rate-limiting.md)）。CLI 的流量规模不可预测且随安装量增长，会把网页版用户的共享配额吃干——网页版是零门槛入口，配额耗尽对它的伤害远大于对 CLI 的伤害（CLI 用户可以自己填一个免费 key）。
- **给 CLI 单开一个带鉴权的 API 端点**。否决：需要发放并管理凭证，等于给一个无用户态、不存储任何数据的项目（[ADR-0008](./0008-privacy-informed-consent-upfront.md)）凭空引入用户态。

## Consequences

- **CLI 的配额是用户自己的**：无 key 时 proxycheck 给 100 次／天，注册免费 key 后 1,000 次／天。CLI 提供可选的 key 配置（`ipcheck config set proxycheck-key` 或 `PROXYCHECK_API_KEY`）。
- **无 key 的配额按发起查询的 IP 计，而 CLI 用户的出口 IP 往往是共享代理节点**——同节点的其他人会吃掉同一份配额。用户可能第一次运行就撞上「今日额度已用尽」。CLI 必须显式说明这一点，否则会被当成工具故障。
- CLI 侧的 O4（IP 类型与风险）**自动执行**，不像 Web 那样按需触发：CLI 用自己的配额，且用户装 CLI 并运行本身即构成同意。这不影响判级规则，只影响 `preliminary` 形态出现的频率。
- CLI 必须升级到 proxycheck **v3**（Python 版用的是 v2），与 Web 同源，否则契约里「风险分 ≥ 70」在两端可能指向不同的标尺。
- Worker 侧的既有硬约束在 CLI 侧同样成立：**不接受"查询任意 IP"的参数**，查询目标恒为本机探测到的出口 IP。
- 本仓库因此有两条互不相交的第三方调用路径。新增任何第三方调用，两条路径都要各自登记（Web 侧在 `docs/api.md`，CLI 侧在其文档与文案中）。
