# proxycheck.io 集成参考

- 状态：**参考文档**——记录第三方的行为与我们的使用约束。关于 proxycheck 的事实以本文为准，**判级规则以 [docs/verdict.md](./verdict.md) 为准**
- 核实日期：**2026-08-12**（文档抓取 + 实测查询）。proxycheck 会改，**引用本文的结论前先看这个日期**
- 来源：<https://proxycheck.io/api/>（同一页同时挂着 v3 与 legacy v2 两份文档）
- 使用方：`worker/proxycheck.ts`（Web）与 `cli/src/probe/proxycheck.rs`（CLI）——两端读同一批字段路径

两端都用 **v3**。`ai-ipcheck`（已归档的 Python 前身）用的是 v2，**两者不是同一把尺子**，见 §4。

---

## 1. 风险分怎么来的（v3）

风险分是 0–100 的整数。它**不是连续的经验值**，而是先由 IP 判定给一个基准分，再叠加攻击历史：

> The risk score is generated live by the API based on a multitude of factors with heavy weight given to recent attacks. However the score does have a baseline dependant on what the usage type of the IP has been determined as.

### 1.1 基准分表（v3）

| 基准分 | IP 判定 |
|---|---|
| 33% | Hosting |
| **50%** | VPN |
| 75% | Scraper |
| **75%** | TOR |
| 100% | Proxy |
| 100% | Compromised |

### 1.2 proxycheck 自己的分档建议（v3）

注意它是**二维**的——除了分数还看 `detections.anonymous`：

| 风险分 | `anonymous: false` | `anonymous: true` |
|---|---|---|
| 0–25 | Allow Access | Challenge User |
| 26–50 | Challenge User | Challenge User |
| 51–75 | Challenge User | **Deny Access** |
| 76–100 | **Deny Access** | **Deny Access** |

**我们采纳了这一维**：综合结论判「高」的阈值直接取上表的 deny 边界，见 §3.1。

### 1.3 实测样本（2026-08-12，无 key）

| IP | `network.type` | `risk` | `detections` 命中 |
|---|---|---|---|
| `104.16.132.229`（Cloudflare） | Hosting | 0 | hosting |
| `8.8.8.8` | Business | 31 | — |
| `1.1.1.1` | Hosting | 33 | hosting |
| `45.33.32.156` | Hosting | 33 | hosting |
| `185.220.101.1`（已知 TOR 出口） | Business | **100** | proxy, tor, compromised, anonymous |

样本与基准分表吻合：机房 IP 落在 0–33，TOR 出口顶到 100。**基准分不是硬下限**——Cloudflare 自己的段命中 `hosting` 却给 0，说明信誉良好的段会被压下来。

---

## 2. 我们的使用约束

### 2.1 必须带的查询参数

```
https://proxycheck.io/v3/{ip}?p=0&tag=0[&key=...]
```

| 参数 | 为什么 |
|---|---|
| `p=0` | 机器可读的紧凑输出 |
| **`tag=0`** | **不把本次查询写进 proxycheck 的正向检出日志**——这是 [ADR-0008](./adr/0008-privacy-informed-consent-upfront.md) 的隐私要求，**不是可选优化** |

两端必须一致。少带 `tag=0` 就是隐私回退。

### 2.2 配额

| 场景 | 额度 | 按什么计 |
|---|---|---|
| 无 API key | 100 次／天 | **发起查询的 IP** |
| 注册免费 key | 1,000 次／天 | key |

官方声明功能不按付费档位区分，免费版拿到的字段与付费版相同。

**无 key 的「按查询方 IP 计」对本产品有实际后果**：CLI 用户的出口 IP 往往是**共享代理节点**，同节点其他人会吃掉同一份配额——用户可能第一次运行就撞上「今日额度已用尽」。CLI 必须显式说明这一点，否则会被当成工具故障。

两端的配额是**分开的**：Web 用本站注册的 key（Durable Object 守卫，[ADR-0002](./adr/0002-no-kv-for-rate-limiting.md)），CLI 用用户自己的（[ADR-0012](./adr/0012-cli-direct-third-party-not-worker-api.md)）。CLI 不走本站 API 就是为了不吃掉网页版的共享额度。

### 2.3 响应字段（两端共读）

```jsonc
{
  "status": "ok",
  "<被查询的 IP>": {
    "network":    { "asn", "range", "hostname", "provider", "organisation", "type" },
    "location":   { "country_name", "country_code", "region_name", "region_code",
                    "city_name", "postal_code", "continent_name", "continent_code",
                    "latitude", "longitude", "timezone" },
    "detections": { "proxy", "vpn", "tor", "scraper", "hosting", "anonymous",
                    "compromised", "risk", "confidence", "first_seen", "last_seen" }
  }
}
```

- `network.type` 取值：`Residential` / `Business` / `Wireless` / `Hosting`
- `location.timezone` 是 IANA 名——**CLI 的 O1/O2 依赖它**（Web 用 `request.cf`，见 [verdict.md §5.4](./verdict.md)）
- Web **刻意不吃 `location` 段**（它有免费无限的 `request.cf`），CLI 只能吃

### 2.4 已知的坑：HTTP 200 也可能不是合法 JSON

**实测**：proxycheck 会间歇性地以 `HTTP 200` + `content-type: application/json` 返回一份**字段类型名而非值**的 schema 形状 body，且它**根本不是合法 JSON**（键没有引号）：

```
{
  1.1.1.1:
  {
    detections: { risk: int, proxy: bool }
  }
  status: string
}
```

响应带 `__cflb` 负载均衡 cookie，疑似某个后端或缓存层所致。

**因此「200 就当成功」是错的。** 两端必须严格解析，并把下列情形**一律视为上游不可用**：

- JSON 解析失败
- `status != "ok"`
- `detections.risk` 不是数字
- **`detections.anonymous` 不是布尔**（判级阈值由它决定，见 §3.1）
- 回显的 IP 与我们查询的不是同一个

**风险分缺失绝不能默认成 0**——那会把有风险的 IP 静默报成低风险，正是 [verdict.md §2.3](./verdict.md) 要防的那件事。

`status: "denied"` 表示配额／权限被拒，单独归为「配额耗尽」——它不是故障，提示语也不同（用户可以配 key 解决）。

---

## 3. 我们的阈值与它的依据

判级规则本身是契约，见 **[verdict.md §3.1](./verdict.md)**（综合结论）与 **[§6](./verdict.md)**（分项分级）。本节只记依据。

### 3.1 综合结论判「高」——二维，直接采用 v3 官方的 deny 边界

| `anonymous` | 阈值 |
|---|---|
| `false` | 风险分 **≥ 76** |
| `true` | 风险分 **≥ 51** |

两个数就是 §1.2 那张表里的 deny 格。**`anonymous` 不是「用户在用 VPN」**——实测一个普通商业 VPN 出口是 `false`，一个已知 TOR 出口是 `true`。它表达的是「这个 IP 当前正被用作匿名化地址」，且带延迟摘牌机制（IP 不再被这么用之后，标记会在一段时间后撤下）。

对本产品的实际效果：

- **普通用户**（干净的 VPN／机房出口，`anonymous: false`）阈值从原先单维的 70 **升到 76**，更不易误报
- **真正在做匿名中转的 IP**（`anonymous: true`）阈值从 70 **降到 51**——VPN 的基准分是 50，意味着「被判定为匿名地址、且有任何攻击历史」即为高风险

这一维把「你在用 VPN」和「你的出口正被别人拿来匿名作恶」分开了。前者是本产品用户的常态，后者才是会触发 AI 服务风控的东西。

**`risk` 与 `anonymous` 必须成对到达**：阈值由后者决定，只拿到一个就判不了。任一缺失 ⇒ 上游不可用（两端都是这么实现的）。缺 `anonymous` 时默认成 `false` 会把阈值静默抬到 76、造成漏报——静默降级比响亮失败难查得多。

### 3.2 分项分级 = v3 四档收成三色

| 分项颜色 | 风险分 | v3 对应档 |
|---|---|---|
| 绿 | 0–25 | Allow（`anonymous: false` 时） |
| 黄 | 26–75 | Challenge |
| 红 | 76–100 | Deny（对两种 `anonymous` 都是） |

四档收成三色时中间两档并作黄——**绿是它建议放行的区间，红是它对任何 IP 都建议拒绝的区间**。

**与综合结论的关系**：非匿名时两者同界（都是 76）；`anonymous: true` 时不同界——结论 51 起判高，分项 76 才转红。因此存在「**结论高 · 分项黄**」这一档（匿名 IP、分数 51–75），呈现层必须靠文字解释补上，见 [verdict.md §6](./verdict.md)。

> ⚠️ **历史**：本文早前版本用 34 作低—中分界，理由是 33 为 Hosting 的基准分。更早一版还引用过「proxycheck 文档写明 below 33 can be considered low risk」——**那句出自 v2 文档**（`&risk=1` 是 v2 的 query flag），拿它论证 v3 的分界是错的。现已整体改为对齐 v3 自己的分档，两条旧理由都不再适用。

## 4. v2 与 v3 不是同一把尺子

`ai-ipcheck` 用 v2，两端现在用 v3。**同一个 IP、同一时刻**的实测：

| | v2 | v3 |
|---|---|---|
| `risk` | **66** | **33** |
| 类型 | VPN，已标记为代理 | `network.type: Hosting`，`proxy: false` |

v2 的基准分表也完全不同（Proxy-无其他数据 66 / Proxy-已知协议端口 100 / 失陷服务器 66 / 疑似 VPN 源的机房 66 / 已知 VPN 服务的服务器 73），分档建议也不同（0–66 / 67–73 / 74–100，按 proxy 与 VPN 分列）。

**结论：任何来自 v2 时代的阈值经验都不能直接沿用。** 这正是 §3 要重新推导的原因。

---

## 5. 决策记录

### 综合结论采纳 `anonymous` 维度（2026-08-12）

阈值直接取 v3 的 deny 边界（§3.1）。契约、golden 向量与两端实现均已跟进；`/api/risk` 的响应因此新增 `anonymous` 字段（[api.md §3.1](./api.md)）。

### 分项分级对齐 v3 四档（2026-08-12）

从 34 / 70 改为 **26 / 76**（§3.2）。代价是纯机房 IP 的 33 分从绿变黄——但机房本来就是 [verdict.md §2.1](./verdict.md) 的黄色分项提醒，颜色与提醒因此一致，不再互相矛盾。

**这一改产生了一处新的、已知的不一致**：`anonymous: true` 且分数 51–75 时，综合结论已判「高」而分项仍是黄。原先（34 / 70）的不一致方向相反——分项红而结论低。两者不可能同时消掉，除非让分项颜色也变成二维的、跟着适用阈值走；那会让两把尺子实质合一，与 [verdict.md §6](./verdict.md)「独立演化」的设计相悖。

当前取舍：**接受「结论高 · 分项黄」，靠文字补齐**。理由是这个方向更安全——结论是用户真正据以行动的东西，它已经是红的；而反方向（分项红、结论低）会让用户以为有大事却查无实据。O4 卡片在该 IP 被判定为匿名地址时会显式说明「判高的阈值对它降到 51」。
