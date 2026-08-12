# 分流泄露：契约、向量、ADR 与术语（子计划）

- 父计划：[2026-08-12-split-tunnel-leak.md](./2026-08-12-split-tunnel-leak.md)
- Depends on: —（本子计划是另外两个的前置）

## 目标

按契约 §7.2 的固定顺序，先把判据写死在 normative 文档与 golden 向量里，再让两端各自实现。本子计划**不碰任何实现代码**。

## 范围

**做**：`docs/verdict.md` 修订、`docs/verdict-cases.json` 扩充、ADR-0014、`CONTEXT.md` 三条词条、`docs/country-codes.json`、`CLAUDE.md` 索引与仓库结构树。

**不做**：不改 `docs/api.md`（O5／O6 全程客户端直连第三方，不经 Worker）；不改 `docs/proxycheck.md`。

## 步骤

### 1. `docs/verdict.md` §1 检测项注册表

- 表格加两行：`O5` = **DNS 出口泄露**（可在线，两端自动）、`O6` = **UDP 出口一致性**（可在线，两端自动）
- `C2` 一行的语义从「本地 DNS 服务器与 DNS 泄露」**收缩为**「本地 DNS 服务器配置」
- 「共 8 项，分母恒为 8」改为 **10**
- C5 废弃编号那段保留原样，另补一句：**C2 的语义收缩过**——旧版 `--json` 的 C2 曾在措辞上承诺含泄露判定（实际从未实现），跨版本比对时需知道这一点
- §1.1 的标题一致性约束自动覆盖 O5／O6（它们是 O 类）

→ 验证：表格行数 = 10，`ALL_CHECKS` 的目标状态与之逐一对应

### 2. `docs/verdict.md` §2 信号域

加两行：

| 信号                | 来源 | 定义                                               | 档位   | Web | CLI |
| ------------------- | ---- | -------------------------------------------------- | ------ | --- | --- |
| `dnsEgressLeak`     | O5   | ECS 客户端子网归属国 ≠ 出口 IP 归属国              | **中** | ✅  | ✅  |
| `udpEgressMismatch` | O6   | ≥2 个 STUN 报出**同一** srflx IP 且该 IP ≠ 出口 IP | **中** | ✅  | ✅  |

新增 §2.5 与 §2.6，把判定写死到「换个人实现也只有一种结果」的程度：

**§2.5 `dnsEgressLeak` 的判定**

1. 解析一个**唯一子域**，从响应取 **ECS（EDNS Client Subnet）客户端子网的归属国**
2. 与出口 IP 归属国比 **ISO2 代码**，不同即命中
3. **不得使用 resolver 自身的归属国作判据**——resolver 在哪个国家取决于用户选了哪家 DNS 服务商，与流量走没走代理无关（DNS 设 `223.5.5.5` + TUN 全局的用户会被误报）。resolver 归属**可以展示，不得判定**
4. **不得比 ECS 的 IP 前缀**——掩码位数未知（不同 resolver 用 /24、/20 不等），比前缀是掩码敏感的，比国家不是
5. ECS 缺失（如 Cloudflare `1.1.1.1` 明确不发 ECS）⇒ **无从比对**，按 §2.3 不贡献信号；此时 O5 仍记「**已完成**」

**§2.6 `udpEgressMismatch` 的判定**

| ≥2 个 STUN 的 srflx IP            | 判定                                                        |
| --------------------------------- | ----------------------------------------------------------- |
| 一致，且 = 出口 IP                | `udpEgressMismatch = false`                                 |
| 一致，且 ≠ 出口 IP                | `udpEgressMismatch = true`                                  |
| **不一致**                        | **无从比对**，不贡献信号（多出口集群／对称 NAT 会落在这里） |
| 拿不到（全部失败、WebRTC 被禁用） | O6 **检测失败**，不产出信号                                 |

外加一条硬约束：**srflx 返回 IPv6 而出口 IP 是 IPv4（或反之）不得判命中**——那是 O3（IPv6 泄露）已经在管的事，重复计一次等于同一个事实算两遍。只比同协议族。

**§2.1 显式不贡献综合结论的观察** — 现有那条「本地 DNS 为国内 DNS（C2）」保留，并补一句说明它与 O5 的分工：**C2 是「你配了什么」，O5 是「查询真的从哪出去了」**。配了国内 DNS 但走全局隧道 ⇒ C2 提醒亮黄、O5 不命中。同时加入 resolver 自身归属国（O5 的展示字段，不判定）。

**§3.2 不变量** — 补一句：本次新增的两个信号**均为「中」档**，因此「高只出现在 `full` 形态」原样成立；该不变量的守卫条件不变。

### 3. `docs/verdict.md` §4 覆盖度

- Web：`X + Y + Z + W = 10`
- CLI：`X + Z = 10`
- 「需 CLI」仍为 4（O5／O6 是可在线项，不进这一档）

### 4. `docs/verdict.md` §5 两端差异登记表

新增两条：

**§5.5 DNS 解析路径：浏览器 DoH vs 系统 resolver**

CLI 走系统 resolver；浏览器可能开着自己的 Secure DNS（DoH），此时根本不经系统 resolver。**两端可以得出完全相反的 `dnsEgressLeak`**，且消灭不掉。

结构与 §5.1（`$TZ` vs 系统时区）**完全相同**：Web 测的是浏览器实际用的 DNS，CLI 测的是命令行进程实际用的 DNS，而本产品用户关心的恰恰是后者。**Web 的 O5 因此是降级代理**，卡片内必须写明这一点。

**§5.6 WebRTC 栈 vs 裸 UDP socket**

Web 经浏览器 WebRTC 栈拿 srflx，CLI 直接发 RFC 5389 binding request。浏览器可能被扩展、企业策略或隐私设置限制而拿不到候选（此时 O6 检测失败），CLI 的 UDP 直接走系统路由，不受此限。**可观察后果**：同一台机器上 Web 报 O6 检测失败、CLI 正常出结论。

### 5. `docs/verdict-cases.json`

新增用例，`signals` 按既有约定给**原始观测值**而非派生布尔量：

- `dnsEcsCountry`（ISO2 或 `null`）、`exitCountry`（ISO2）
- `stunReflexiveIps`（数组，各 STUN 的 srflx IP；`null` = O6 失败）、`exitIp`

必须覆盖的场景（`applies: ["both"]`，除非注明）：

| 场景                              | 期望                                                              |
| --------------------------------- | ----------------------------------------------------------------- |
| ECS 国 ≠ 出口国，其余干净         | `full`／`preliminary` · **中**                                    |
| ECS 缺失（`dnsEcsCountry: null`） | 不贡献信号——同输入下与「ECS 国 = 出口国」结论相同                 |
| ECS 国 = 出口国                   | 不命中                                                            |
| 两个 STUN 同 IP 且 ≠ 出口 IP      | **中**                                                            |
| 两个 STUN 同 IP 且 = 出口 IP      | 不命中                                                            |
| 两个 STUN **不同 IP**             | 不贡献信号（≠ 不命中，需与「= 出口 IP」用例配对断言两者结论一致） |
| STUN 返回 IPv6、出口 IP 为 IPv4   | **不命中**（O3 的职责）                                           |
| O4 未知 + 仅 `dnsEgressLeak` 命中 | `preliminary` · **中**（验证新信号不破坏形态规则）                |
| O5／O6 均失败、其余全 null        | `insufficient`（验证新项不会把「没测成」变成「低」）              |

→ 验证：`version` 字段保持 `1`（schema 未变，只加字段与用例）；两端参数化测试在实现落地前**应当全部变红**

### 6. `docs/country-codes.json`

ip-api 英文国家名 → ISO2 的映射表（约 250 条）。约定写在文件头 `about` 字段里：

- 键是 **ip-api 返回的英文国家名**（`edns.geo` 中 `" - "` 之前的部分），不是 ISO 3166 官方全称
- **查不到的国家名 ⇒ 视为未知，按 §2.3 不贡献信号**，不得回退成「不同国家」（那会造出误报）
- 两端共吃这一份：Web 打进 bundle，CLI `include_str!`

### 7. ADR-0014

`docs/adr/0014-split-tunnel-leak-checks.md`。三条标准都满足：难以逆转（ID 不复用、分母变更）、无上下文会困惑（刚从 9 砍到 8，为什么又涨到 10）、真实权衡。

必须记进 Considered Options 的否决项：

- 合并为一项（覆盖度无法诚实表达「一半成功」）
- 用 resolver 归属国作判据（打中典型用户的系统性误报）
- 比 ECS IP 前缀（掩码敏感）
- 跟参考站把「WebRTC 禁用」渲染成绿色（叙事不同，绿色在我们的语义下是错的）
- O6 降为分项提醒（对照法能更诚实地处理误报源）

必须记进 Consequences 的代价：

- `edns.ip-api.com` 的 HTTPS 是未定价的口子，随时可能被收紧
- 不发 ECS 的用户永久拿不到 O5 结论
- O6 需要两个 STUN，「少暴露一方」只守住一半
- C2 的能力承诺被**收缩**（措辞层面），这是还债不是新功能

### 8. `CONTEXT.md` 与 `CLAUDE.md`

`CONTEXT.md` 新增三条词条（纯术语，零实现细节）：

- **分流泄露** — 部分流量绕开代理直接出网的现象。_Avoid_：漏代理、代理泄露
- **出口 resolver** — 实际替用户向权威 DNS 发起查询的那台服务器。_Avoid_：本地 DNS（那是 C2 的「本机配置的 DNS 服务器」，两者常常不是同一台）
- **反射地址** — STUN 服务器回报的、它所看到的客户端公网地址。_Avoid_：srflx、公网 IP

`CLAUDE.md`：仓库结构树加 `docs/country-codes.json` 一行（Spec Document Index 强制维护规则）。

## 验收标准

1. `docs/verdict.md` 中不存在任何「8 项」「分母恒为 8」的残留（全文搜 `8` 逐个确认）
2. `verdict-cases.json` 的新用例在**两端现有实现下全部失败**——若有用例意外通过，说明它没测到新东西
3. §2.5／§2.6 的判定表能让第三方**不看代码**写出一致的实现
4. `docs/country-codes.json` 中 `Japan → JP`、`United States → US`、`Russia → RU`、`Taiwan → TW`、`South Korea → KR` 等 ip-api 惯用写法均可查到
5. ADR-0014 的 Considered Options 覆盖上面列出的全部五条否决项——**否决理由才是这份 ADR 的价值**
6. `CONTEXT.md` 三条词条中不含任何实现细节（无 provider 名、无字段名、无阈值）
