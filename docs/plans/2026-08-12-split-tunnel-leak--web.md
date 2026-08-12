# 分流泄露：ipcheck Web 实现（子计划）

- 父计划：[2026-08-12-split-tunnel-leak.md](./2026-08-12-split-tunnel-leak.md)
- 契约：[docs/verdict.md](../verdict.md) §1／§2.5／§2.6／§4／§5.5／§5.6
- Depends on: [--contract](./2026-08-12-split-tunnel-leak--contract.md)
- 与 [--cli](./2026-08-12-split-tunnel-leak--cli.md) **互不依赖，可并行**

## 目标

在浏览器侧实现 O5 与 O6，两项**首屏自动执行**、与 O1／O3 并发，不增加首屏总时长。

## 步骤

### 1. 探测层 `src/probes/`

沿用 `ipify.ts` 立下的那条缝：**探测层只产出原始观测，判定全在 `domain/`**。

**`dnsEgress.ts`**

- `GET https://<随机前缀>.edns.ip-api.com/json`，前缀用 `crypto.getRandomValues` 生成（**每次重试必须换新前缀**，否则打的是 DNS 缓存）
- 超时 5s（与 ipify 对齐），失败最多重试 2 次
- 产出 `{ ecsGeo: string | null; resolverGeo: string | null }`——原样返回 ip-api 的 `edns.geo` 与 `dns.geo` 字符串，**不在这里切国家名、不在这里查表**

**`stun.ts`**

- 两个 STUN：`stun.cloudflare.com`、`stun.l.google.com:19302`，并发，各 5s
- 只接受 `srflx` / `prflx` 候选——host 候选证明不了 STUN 工作过（现代浏览器还会把它 mDNS 混淆成 `.local`），**不得当作反射地址**
- `typeof RTCPeerConnection !== "function"`，或构造出的对象缺 `createOffer`／`createDataChannel`（隐私扩展的桩），一律按**拿不到候选**处理
- 组件卸载时关闭所有在途 `RTCPeerConnection`
- 产出 `{ reflexiveIps: string[] }`（拿到几个算几个）

### 2. 判定层 `src/domain/`

**`dnsEgress.ts`**：`ecsGeo` → 取 `" - "` 之前的国家名 → 查 `country-codes.json` → ISO2 → 与出口 IP 的 `country` 比。查不到国家名、`ecsGeo` 为 `null`、出口国未知 —— 三者**都返回「无从比对」**，不得回退成「不同」。

**`udpEgress.ts`**：按契约 §2.6 的四行判定表实现，含**同协议族**约束。

**`types.ts`**：加 `DnsEgressResult` 与 `UdpEgressResult`。两者都要让「无从比对」在**类型层面与「未命中」不可混淆**——参照 `Ipv6Result` 用联合类型的做法，不要用 `boolean | null` 加注释。

**`checks.ts`**：`ONLINE_CHECK_IDS` 加 `"O5"`／`"O6"`（`TOTAL_CHECKS` 随之自动变 10）、`PanelState` 加 `o5`／`o6`、`INITIAL_PANEL` 中两者均为 `{ status: "running" }`。

**`coverage.ts`** 与 **`verdict.ts`**：接入两个新信号，均为「中」档。

### 3. 编排 `src/usePanel.ts`

O5／O6 与 O1／O3 并发发起。**两者都依赖出口 IP／出口国**（来自 O1），因此判定必须等 O1 落地——探测可以先发，比对在 O1 到达后进行。O1 失败 ⇒ O5／O6 **无从比对**，不产信号，但**不算检测失败**（它们自己的探测成功了）。

### 4. 呈现层 `src/components/`

两张新卡，沿用现有卡片的五态。文案硬约束：

- **O5 卡内必须写明**：本项测的是**浏览器**用的 DNS；浏览器若开着 Secure DNS（DoH），结果会与命令行工具不同，后者属 CLI 的判定范围（契约 §5.5）
- **O5 展示 resolver 归属**（`dns.geo`）但必须标明**它不参与判定**，并说明为什么（resolver 在哪取决于你选了哪家 DNS）
- **ECS 缺失时**卡片明写「你的 DNS 服务商不发送 ECS，无法判定 DNS 查询是否走代理」，状态是**已完成**而非失败
- **WebRTC 被禁用时**落「检测失败」，文案说明「浏览器禁用了 WebRTC，本项无法判定 UDP 是否走代理；CLI 不受此限制」——**不得渲染为绿色**
- O6「无从比对」（两个 STUN 报出不同 IP）要与「未命中」在文案上可区分

### 5. 隐私前置告知（ADR-0008）

ADR-0008 要求「第三方调用标注在**触发它的那个控件**上」，而这两项是自动执行的、**没有控件**。因此首屏那句前置告知必须扩成完整清单，新增：`ip-api.com`（DNS 出口探测）、`stun.cloudflare.com` 与 `stun.l.google.com`（UDP 出口探测）。

**这一条不是文案润色，是 ADR-0008 的合规项**——漏了即视为违反该 ADR。

### 6. 文案 `src/copy.ts` + `src/locales/`

五语种（`en` 为源语言，其余按字段回落）。O5／O6 的**标题**在两端共同支持的四语种（`en`／`zh-hans`／`zh-hant`／`ru`）下与 CLI **逐字一致**（契约 §1.1）；`ar` 是 Web 独有，不受该约束。

### 7. 测试 `tests/`

- golden 向量参数化测试自动覆盖新用例（无需改测试代码，只需判定层接上）
- `dnsEgress` 判定单测：ECS 缺失、国家名查不到表、出口国未知——三种「无从比对」各一条
- `udpEgress` 判定单测：契约 §2.6 四行 + IPv6／IPv4 混合那条
- 覆盖度单测：`X + Y + Z + W = 10`
- `stun.ts` 的 `RTCPeerConnection` 不存在时走失败路径（用 stub 覆盖）

## 验收标准

1. `make check` 全绿（**不需要 Rust 工具链**）
2. 首屏总时长不因新增两项变长——`Performance` 面板确认 O1／O3／O5／O6 并发发起
3. 关掉浏览器 WebRTC 后，O6 落**检测失败**且卡片给出上述文案；覆盖度显示 `检测失败 1`
4. 把系统 DNS 切到 `1.1.1.1`（不发 ECS）后，O5 落**已完成 · 无信号**并给出说明，综合结论**不因此改变**
5. 首屏前置告知里能看到 `ip-api.com` 与两个 STUN 域名
6. O5 卡内能读到「本项测的是浏览器用的 DNS」这句话——缺了它，CLI 用户会误以为自己命令行的 DNS 已被检查
7. `TOTAL_CHECKS === 10` 且覆盖度四档之和恒等于它（测试断言，不是人工核对）
