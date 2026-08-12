# 分流泄露：ipcheck CLI 实现（子计划）

- 父计划：[2026-08-12-split-tunnel-leak.md](./2026-08-12-split-tunnel-leak.md)
- 契约：[docs/verdict.md](../verdict.md) §1／§2.5／§2.6／§4／§5.5／§5.6
- 决策：[ADR-0010](../adr/0010-verdict-contract-normative-cli-full-implementation.md)（CLI 是全集实现）· [ADR-0012](../adr/0012-cli-direct-third-party-not-worker-api.md)
- Depends on: [--contract](./2026-08-12-split-tunnel-leak--contract.md)
- 与 [--web](./2026-08-12-split-tunnel-leak--web.md) **互不依赖，可并行**

## 目标

在 CLI 侧实现 O5 与 O6，**保住「CLI 是全集实现」这条架构不变量**——O6 是 WebRTC 语义的检测项，而 Rust 没有 WebRTC 栈，这一点靠语义选择（测 UDP 出口，不测浏览器行为）与裸 STUN 实现同时化解。

## 步骤

### 1. `cli/src/probe/stun.rs` — 手写 RFC 5389

用 `std::net::UdpSocket` 发 binding request，**不引 crate、不引 tokio**（CLAUDE.md 既定技术栈）：

- 20 字节请求头：type `0x0001`、length `0`、magic cookie `0x2112A442`、96-bit 随机 transaction ID
- 响应里找 `XOR-MAPPED-ADDRESS`（attribute `0x0020`）：port 与 IPv4 地址跟 magic cookie 异或，IPv6 跟 cookie + transaction ID 异或
- **必须校验 transaction ID 匹配**——UDP 无连接，收到的可能是别人的包
- 两个 STUN（`stun.cloudflare.com:3478`、`stun.l.google.com:19302`）用 `std::thread::scope` 并发，超时取用户配置的 `timeout`（已在契约 §8 白名单内，**不新增配置键**）

产出 `Vec<IpAddr>`（拿到几个算几个），判定留给 `domain/`。

### 2. `cli/src/probe/dns_egress.rs`

`ureq` 请求 `https://<随机前缀>.edns.ip-api.com/json`，重试 2 次、**每次换新前缀**。产出 `{ ecs_geo: Option<String>, resolver_geo: Option<String> }`，原样返回字符串。

注意 CLI 走**平台信任库**（CLAUDE.md：目标用户全都开着代理，编译期内置根证书会让 TLS 中间人代理下的探测全挂）——沿用现有 HTTP 层即可，不要为这个探测另起一套。

### 3. `cli/src/probe/dns.rs` — C2 语义收缩

**代码逻辑不变**（它本来就只读本机配置）。要改的是**措辞**：文件头注释里「C2 本地 DNS 服务器与 DNS 泄露」改为「C2 本地 DNS 服务器配置」，去掉一切暗示本项做过泄露判定的表述。`KNOWN_DNS` 表的 `CN` 标记继续只服务于 §2.1 的分项提醒。

### 4. `cli/src/domain/`

**`checks.rs`**

- `CheckId` 加 `O5`／`O6`，`as_str()` 与 `ALL_CHECKS` 同步
- `TOTAL_CHECKS: usize = 10`
- 测试 `there_are_exactly_eight_checks_with_distinct_ids` **连函数名一起改**为 `..._ten_checks_...`——留旧名字是给未来的人埋雷（父计划验收项 5）
- `coverage_invariant_holds_for_any_mix` 无需改（它已按 `TOTAL_CHECKS` 参数化）

**`verdict.rs`**：接入 `dns_egress_leak` 与 `udp_egress_mismatch`，均为「中」档。按契约 §2.5／§2.6 实现两个判定函数，含「无从比对」三态与同协议族约束。

**国家表**：`include_str!("../../../docs/country-codes.json")`，查不到的国家名 ⇒ 未知，**不得回退成「不同」**。

### 5. 输出层

**`render.rs`**：两项各一段。硬约束——

- O5 必须**同时展示** ECS 判定结果与 resolver 归属，并标明**只有前者进综合结论**（与 §5.1 里 CLI 同时展示 `$TZ` 与系统时区的做法同构）
- ECS 缺失时明写「你的 DNS 服务商不发送 ECS」，状态为**已完成**
- O6「无从比对」（两 STUN 报出不同 IP）与「未命中」必须文案可分

**`json.rs`**：新增 `o5`／`o6` 两个对象。这是 `--json` 的 **schema 变更**。

> **版本**：仓库目前**尚无 `cli/v*` tag**，CLI 版本为 `0.1.0`，因此这次变更**不破坏任何已发布用户**。仍按 semver 的 0.x 约定把破坏性 schema 变更升为 **minor**：`0.1.0 → 0.2.0`。（盘问阶段说的「升 major」建立在 CLI 已发布的假设上，实测 `git tag` 为空，此处按实际情况修正。）

### 6. 文案 `cli/src/copy/`

四语种（`en` 源语言 + `zh_hans`／`zh_hant`／`ru` 的字段级补丁）。O5／O6 的**标题**必须与 Web 侧**逐字一致**（契约 §1.1）——这一条要与 `--web` 子计划对齐后再落笔，不要两边各写各的。

### 7. 测试

- golden 向量参数化测试自动覆盖新用例
- `stun.rs` 单测：XOR-MAPPED-ADDRESS 解码（IPv4 与 IPv6 各一条固定字节串）、transaction ID 不匹配时丢弃
- `dns_egress` 判定单测：ECS 缺失、国家名查不到表、出口国未知
- `checks.rs`：10 项、ID 互不相同、覆盖度不变量

## 验收标准

1. `make check-cli` 全绿（fmt + clippy + build + test）
2. 无新增依赖——`cargo tree` 与改动前一致
3. `ipcheck --json` 输出含 `o5`／`o6`，且 proxycheck key 不出现在其中任何位置
4. 断网跑：O5／O6 落「检测失败」，覆盖度 `已完成 X · 检测失败 Z` 之和恒为 **10**
5. 把系统 DNS 切到 `1.1.1.1` 后，O5 报「已完成 · 无信号」并说明 ECS 缺失，综合结论不变
6. **平价验收**：与 `--web` 在同一网络环境下对比 O5／O6 的信号值，不一致必须能指回契约 §5.5 或 §5.6
7. `grep -rn "DNS 泄露" cli/src/probe/dns.rs` 无结果——C2 的措辞收缩已落实
