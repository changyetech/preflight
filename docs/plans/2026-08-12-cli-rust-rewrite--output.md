# CLI 重写 · 呈现层、`--json` 与平价验收

- 父计划：[2026-08-12-cli-rust-rewrite.md](./2026-08-12-cli-rust-rewrite.md)
- Depends on: `--domain`, `--probes`

## 目标

产出人读的报告与机读的 `--json`，并用 `--json` 完成与 `ai-ipcheck` 的**数据与结论平价**验收。

## 范围

**做**：终端渲染、颜色降级、`--json`、退出码约定、`ai-ipcheck` 的一次性 `--json` 补丁与比对。
**不做**：发布、网站、README（属 `--release`）。

## 步骤

1. **渲染结构**（参考 ipcheck Web 的视觉语言，**不用表格边框**）
   - **结论区置顶**：档位配色 + 一句话结论 + 覆盖度行（`已完成 X · 检测失败 Z`）+ 出口 IP
   - **9 张检测卡**：状态符号 + 标题 + 值 +（命中风险时的）建议
   - **丢弃**：顶部导航（终端无锚点）、落地内容（营销文案）
   - `ai-ipcheck` 那条「禁用 `✓✗⚠` 等歧义宽度符号」的禁令是**为右边框对齐**立的，无边框后失效——状态符号可以用
   - 无固定列宽常量：`ai-ipcheck` 的 `COL_LABEL = 20` 是「多语言不存在」这个假设的化石，不移植
   - → verify：快照测试（固定探测结果 → 固定输出）

2. **默认精简，`--verbose` 出完整描述**
   - 默认只给值 + 命中时的建议；`--verbose` 才输出每项的解释文案
   - 理由：Web 上描述文案是白送的（页面可以滚），终端里每多 9 段就多一屏，伤害「启动 Claude 前扫一眼」这个核心场景
   - → verify：两种模式各一个快照测试

3. **颜色与 TTY 降级**
   - 语义档位 → 绿／黄／红
   - 支持 `NO_COLOR` 环境变量与配置项 `no_color`
   - **非 TTY 自动关色**（重定向到文件不应满屏转义序列）
   - → verify：断言非 TTY 与 `NO_COLOR` 下输出不含 ANSI 转义

4. **一次性渲染 + stderr spinner**
   - 探测全部完成后**一次性**输出完整报告到 stdout
   - 进行中在 **stderr** 打 spinner；**非 TTY 时不打**
   - 不做光标控制的渐进渲染：双渲染路径 + SIGWINCH + 终端兼容性引入的 bug 面，比整个探测层还大
   - → verify：`ipcheck | cat` 的 stdout 是一份干净完整的报告

5. **`--json`**
   - 扁平的检测结果 + verdict，字段名沿用契约信号名（camelCase，与 Web 对齐）
   - **不套** `{code, message, data}` 信封——那是 HTTP 契约（`docs/api.md`），CLI 不是 HTTP
   - **退出码**：`0` 恒表示"体检完成"，**不论风险档位**；非 `0` 表示工具自身失败（配置错误、全部探测失败等）。用退出码表达风险档位会让脚本把"高风险"和"工具挂了"混为一谈
   - **key 绝不出现在输出中**
   - → verify：schema 快照测试；高风险场景断言退出码为 `0`；设置 key 后 `--json` 输出 grep 不到 key

6. **`ai-ipcheck` 的一次性 `--json` 补丁**
   - 在本机给 `refs/ipcheck/` 打补丁，把 `main()` 已有的局部变量 dump 成 JSON
   - **不发布、不进归档仓库的默认分支**，只作为重写期的比对工具
   - → verify：`ai-ipcheck --json` 能产出 9 项的值

7. **平价比对**
   - 在若干网络场景下（直连／代理开 TUN／代理不开 TUN／IPv6 开启）两边各跑一次，逐字段 diff
   - **已知且允许的差异**（不得当作回归）：
     - `docs/verdict-cases.json` 中标记 `aiIpcheckDiverges` 的场景——IPv6 泄露时新版判中、旧版判低
     - **黑名单命中场景**——旧版判高，新版按其余信号定档（黑名单不再是信号）
     - 呈现差异全部允许：平价的含义是**数据与结论平价，不含呈现平价**
   - 差异清单写回本文件
   - → verify：比对报告中除上述两类外无差异

### 平价比对结果（2026-08-12，macOS，同一网络环境，两边各跑一次）

**结论一致**：两边都判「中风险」。

| 检测项 | `ai-ipcheck` | ipcheck | |
|---|---|---|---|
| C1 本机真实 IP | `223.11.199.13` | 同 | ✅ |
| O1 出口 IP / 国家 / 城市 / 运营商 / 时区 | `212.50.249.204` / Japan / Osaka / IT7 Networks Inc / `Asia/Tokyo` | 同 | ✅ |
| C2 本地 DNS | `fe80::1%en0`、`192.168.1.1` | 同 | ✅（修复后，见下） |
| C3 环境变量 / 系统代理 / TUN | 未设置 / 未开启 / 疑似开启 | 同 | ✅ |
| O4 滥用记录 | 未收录 | 无收录 | ✅ |
| O2 / C4 时区一致性 | 均不一致 | 均不一致 | ✅ |
| C5 Claude 端点 | 官方直连 | 官方直连 | ✅ |
| **综合结论** | **中风险** | **中风险** | ✅ |
| O3 IPv6 | `fc00::1`，判**泄露** | 判**未启用** | ⚠️ 见 ① |
| O4 风险分 / 类型 | 66/100、VPN、已标记为代理、住宅 IP | 33/100、Hosting、proxy=false | ⚠️ 见 ② |

**① O3 的探测方法不同，`ai-ipcheck` 在这里是误报。**
它用 UDP socket 连一个 IPv6 地址、读本地 socket 地址，拿到的是**本机网卡地址**——实测得到 `fc00::1`，那是 ULA（Unique Local Address，私有、不可全球路由），根本泄露不出去，却被报成「IPv6 泄露，暴露真实地址」。ipcheck 用 ipify 的远端回显（契约 2.2 / [ADR-0003](../adr/0003-ipv6-leak-via-third-party-dual-stack-echo.md)），只有真的存在公网 IPv6 出口才判泄露。**这是相对 `ai-ipcheck` 的第三处行为差异，且它修正了一个误报**，不是回归。

**② proxycheck v2 与 v3 不是同一把尺子——这是硬证据。**（完整对比见 [docs/proxycheck.md §4](../proxycheck.md)。）
同一个 IP、同一时刻：v2 给 `risk 66`、`type VPN`、`proxy yes`；v3 给 `risk 33`、`network.type Hosting`、`proxy false`。差了整整一倍。「住宅/机房」那一栏在 `ai-ipcheck` 里来自 ip-api（已被 [ADR-0007](../adr/0007-proxycheck-v3-only-drop-ip-api.md) 弃用），与 proxycheck 的 `network.type` 本就不是同一个字段。

> **已标定（2026-08-12）**：查 proxycheck v3 文档后发现风险分并非连续经验值，而是由 IP 判定推出来的，基准分为 Hosting 33 / VPN 50 / Scraper 75 / TOR 75 / Proxy 100 / Compromised 100。本次实测到的 `risk 33` 正是 **Hosting 的基准分**，不是"标尺被压缩"的证据——上面那句「v3 只拿到一半分数」的推断是错的。
>
> **最终结论（同日再次修订）**：v3 的分档建议是**二维**的——除分数外还看 `detections.anonymous`，deny 边界是 `anonymous: false` ⇒ ≥76、`anonymous: true` ⇒ ≥51。已决定采纳这一维，综合结论改为二维判定，直接取官方边界。
>
> 中间那版「阈值落在 (50, 75]、70 仍然正确」的推导是**单维框架下**的产物，已被二维方案取代；其中「分项分界 34」的论证还一度引错了版本（那句 low/medium 说明出自 v2 文档）。完整的溯源、更正与现行标定见 **[docs/proxycheck.md](../proxycheck.md)**，契约见 [verdict.md §3.1](../verdict.md)。分项分级随后也改为对齐 v3 四档（**26 / 76**），同样见 proxycheck.md §3.2。

**③ 比对中发现并修复的一个真实缺陷（我方）**：`fe80::1%en0` 这类带 zone index 的 IPv6，Rust 的 `IpAddr` 解析器不认，导致整条 DNS 记录被静默丢弃——而 macOS 上路由器下发的 DNS 恰恰常是这个形态。已修（`probe/dns.rs::parse_addr`）并补了回归测试。

## 验收标准

1. 默认／`--verbose` 两种模式各有快照测试
2. `NO_COLOR` 与非 TTY 下输出无 ANSI 转义（有测试）
3. `ipcheck | cat` 的 stdout 是完整报告，spinner 不污染 stdout
4. `--json` 有 schema 快照测试；key 不出现在输出中（有测试）
5. 高风险场景下退出码仍为 `0`（有测试）
6. 平价比对报告完成，差异仅限两类已知项，清单写回本文件
