# CLI 重写 · 9 项探测

- 父计划：[2026-08-12-cli-rust-rewrite.md](./2026-08-12-cli-rust-rewrite.md)
- 契约：[docs/verdict.md §1](../verdict.md)
- Depends on: `--foundation`

## 目标

实现全部 9 项探测，行为对齐 `ai-ipcheck`（[ADR-0010](../adr/0010-verdict-contract-normative-cli-full-implementation.md) 记明的两处刻意变更除外）。

## 范围

**做**：O1–O4、C1–C5 的采集与解析、并发编排、失败降级。
**不做**：渲染、`--json` 输出格式（属 `--output`）。

## ⚠️ 决策点：CLI 的 O1 地理数据从哪来

`ai-ipcheck` 的出口 IP 归属取自 **ip-api.com**，而 [ADR-0007](../adr/0007-proxycheck-v3-only-drop-ip-api.md) 已弃用它（禁止商用 + 免费版只能明文 HTTP 传用户 IP）。ipcheck Web 用 `request.cf`（免费无限），**CLI 没有这个东西**。这是原方案里没覆盖的缺口。

**本计划采用**：

- **出口 IP 地址**取自 **ipify**（免费无限、无配额，O3 已在调用它的 v4 端点）
- **归属／ASN／组织／IANA 时区**取自 **proxycheck v3** 的地理段（一次调用同时服务 O1 与 O4）

**后果（必须接受或推翻）**：proxycheck 失败或配额耗尽时会**级联**——O1 降级为「只有 IP、无归属」，O2 因拿不到出口 IP 时区而**检测失败**，O4 检测失败。Web 侧不存在这个级联（时区来自免费的 `request.cf`）。

**为什么仍选它**：替代方案是再引入第四个第三方地理源，而 [ADR-0008](../adr/0008-privacy-informed-consent-upfront.md) 要求每个第三方都要向用户披露，且 ip-api 已被否决、其余免费源各有额度或注册门槛。配置了免费 key 后配额是 1000 次／天，实际极难触发。关键缓解是：**IP 地址本身走 ipify，不受配额影响**，因此结论区的「出口 IP」永远显示得出来。

### 实测结论（2026-08-12，无 key，直连 `https://proxycheck.io/v3/{ip}`）

> 下面是**决策当时**的记录。proxycheck 的持续事实（字段、配额、必带参数、已知坑）已收进 **[docs/proxycheck.md](../proxycheck.md)**——计划会过期，那份参考文档不会。

**决策成立。** v3 无 key 返回 `status: "ok"` 与完整结果，字段齐全：

- `network`：`asn`（`"AS13335"`）、`organisation`、`provider`、`hostname`、`range`、**`type`**（`Residential`／`Business`／`Wireless`／`Hosting`）
- `location`：`country_name`／`country_code`／`region_name`／`region_code`／`city_name`／`postal_code`／`continent_name`／`continent_code`／`latitude`／`longitude`／**`timezone`**（IANA 名）
- `detections`：`risk`（0–100）、`proxy`／`vpn`／`tor`／`scraper`／`hosting`／`anonymous`／`compromised`／`confidence`

字段路径与 `worker/proxycheck.ts` 完全一致，两端吃的是同一把尺子。

**实测中额外发现的两条，必须落进实现：**

1. **proxycheck 会以 HTTP 200 返回一份不是合法 JSON 的 body**（字段类型名而非值的 schema 形状，间歇出现；响应带 `__cflb` 负载均衡 cookie，疑似某个后端或缓存层所致）。因此**「200 就当成功」是错的**。解析必须严格，并按 `worker/proxycheck.ts` 的既有规则处理：JSON 解析失败 ／ `status != "ok"` ／ `detections.risk` 不是数字 ⇒ **一律视为上游不可用**，O4 计入检测失败。**绝不能把缺失的风险分默认成 0**——那会把有风险的 IP 静默报成低风险。
2. **必须带 `p=0` 与 `tag=0` 两个查询参数**，与 Worker 一致。`tag=0` 不是可选优化：它让本次查询不被写进 proxycheck 的正向检出日志，是 [ADR-0008](../adr/0008-privacy-informed-consent-upfront.md) 的隐私要求。

**连带产生一处新的两端差异**，已登记进 [verdict.md §5.4](../verdict.md)：Web 的归属数据来自 `request.cf`，CLI 来自 proxycheck——两个地理库对同一 IP 的 **IANA 时区可能不同**，`tzMismatchSystem` 因此可能在两端取相反的值。CLI 的 O1 必须标明归属数据来自 proxycheck。

## 步骤

1. **HTTP 客户端与并发编排**
   - `ureq` + `rustls-native-certs`，统一超时（配置项 `timeout`）
   - `std::thread::scope` 并发跑各自独立的探测；**不引 tokio**
   - 依赖关系：O2 依赖 proxycheck 的时区 → 编排上 O2 在 O4 之后
   - 单个探测失败**不得**影响其他探测
   - → verify：单测覆盖「某探测 panic／超时时其余仍产出结果」

2. **O1 出口 IP 与归属** — ipify（IP）+ proxycheck v3（归属）
   - → verify：解析函数对固定 JSON 样本的单测；proxycheck 缺字段时降级为未知而非报错

3. **O2 系统时区一致性** — 本机系统时区 vs 出口 IP 时区（均为 IANA 名）
   - 任一侧时区名缺失 ⇒ **无从比对 ⇒ 不算不一致**（[verdict.md §2.3](../verdict.md)）
   - → verify：比对函数单测，含缺失侧的用例

4. **O3 IPv6 泄露** — ipify 双端点（`api.ipify.org` / `api6.ipify.org`）
   - 判定表见 [verdict.md §2.2](../verdict.md)：v4 不通时一律判**检测失败**，不得判「无 IPv6」
   - → verify：四种组合各一条单测

5. **O4 IP 类型与风险** — proxycheck **v3** + StopForumSpam
   - **自动执行**（[ADR-0012](../adr/0012-cli-direct-third-party-not-worker-api.md)），不设开关
   - 无 key 100 次/天、有 key 1000 次/天；配额耗尽 ⇒ 该项「检测失败」+ 提示可配置 key
   - **必须显式提示**：无 key 的配额按出口 IP 计、与同节点其他用户共享——否则用户会以为工具坏了
   - StopForumSpam 不可用 ⇒ `abuseListed` 未知，**O4 仍算已完成**
   - **不得**接受「查询任意 IP」的参数（[verdict.md §9](../verdict.md)）
   - → verify：解析单测；配额耗尽路径单测

6. **C1 本机真实 IP** — 国内直连回显（沿用 `ai-ipcheck` 的端点集）
   - 规则代理对国内 IP 走直连，故即使开着 VPN 也能露出真实 ISP 出口
   - → verify：解析单测
   - 注意：「真实 IP」一词仅在 CLI 语境成立（[CONTEXT.md](../../CONTEXT.md)）

7. **C2 本地 DNS 服务器与 DNS 泄露** — 平台相关采集 + 已知 DNS 服务商标注
   - 国内 DNS **只进检测建议，不贡献综合结论**（[verdict.md §2.1](../verdict.md)）
   - → verify：拿 `ai-ipcheck` 的 `tests/test_cli.py` 里的样本做解析单测

8. **C3 代理检测** — 环境变量 / 系统代理 / TUN
   - 系统代理**只实现 macOS**（`scutil`），与 `ai-ipcheck` 平价；其他平台报「未实现」而非「未开启」
   - TUN：`ifconfig` + `route` 解析
   - **只显示开关状态、不显示地址**（避免泄露 `127.0.0.1:7890` 之类）
   - `tunOff` 贡献「中」
   - → verify：用 `ai-ipcheck` 测试里的 `scutil`／`ifconfig` 样本做解析单测（含 PAC 场景）

9. **C4 CC CLI 时区一致性** — `$TZ` vs 出口 IP 时区
   - 这是**贡献综合结论**的那条时区信号；O2 那条只展示（[verdict.md §5.1](../verdict.md)）
   - → verify：单测覆盖 `$TZ` 已设／未设

10. **C5 Claude 端点检测** — 官方直连 / 国产大模型 / 中转 + 147 黑名单
    - 三态判定沿用 `ai-ipcheck`：未设或 host == `api.anthropic.com` ⇒ 官方；命中国产关键词 ⇒ 国产；其余 ⇒ 中转告警
    - 未设 base url 时先用「是否装了 Claude Code」分岔，没装不误报
    - 黑名单命中**仍要展示并告警**，但**不进综合结论**（[ADR-0010](../adr/0010-verdict-contract-normative-cli-full-implementation.md)）
    - → verify：三态各一条单测 + 黑名单命中不改变档位的单测

## 验收标准

1. 9 项各有解析单测，样本尽量取自 `ai-ipcheck` 的既有测试
2. 单个探测失败不影响其余探测（有测试）
3. 时区「无从比对」不被当成「不一致」（有测试）
4. O3 在 v4 端点不通时判检测失败，不判「无 IPv6」（有测试）
5. 黑名单命中不改变综合结论档位（有测试）
6. 代理相关输出不含任何地址
7. proxycheck v3 响应字段已实测确认，决策点结论写回本文件
