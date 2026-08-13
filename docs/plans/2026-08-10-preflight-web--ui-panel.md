# ipcheck Web — 检测面板

- 父计划：[2026-08-10-preflight-web.md](./2026-08-10-preflight-web.md)
- 实现规格：[docs/specs/2026-08-10-preflight-web.md](../specs/2026-08-10-preflight-web.md) 第 2.2 / 2.3 / 3 / 4 节
- Depends on: `--scaffold`、`docs/api.md` 契约（非 `--worker-api` 完工）

## 目标

交付首屏结论区与 9 张检测卡片，实现两段式综合结论、覆盖度三分与卡片五态。契约既定即可对 mock 开工，与 `--worker-api` 并行。

## 范围

**包含**：首屏结论区、检测卡片流、O2 时区比对、O3 IPv6 检测（浏览器侧）、风险判定、覆盖度、卡片五态、灰卡与复制安装命令、响应式。

**不包含**：落地内容与 i18n（归 `--content`）。

## 步骤

1. 定义检测项数据模型：9 项、五态、风险贡献（规格第 2 节总表）
   → verify：类型层面无法构造出「仅 CLI 项处于检测中」这类非法状态
2. 先写风险判定的失败测试再实现（规格 3.1 / 3.3）
   → verify：覆盖「风险分≥70→高」「时区不一致→中」「IPv6 泄露→中」「spam 收录→中」「全清→低」「未含 O4 时永不为高」六类断言
3. 实现覆盖度三分计算（已完成 / 需 CLI / 检测失败，恒等于 9）
   → verify：单测断言三数之和恒为 9，且失败项不与「需 CLI」混计
4. 实现 O2 系统时区一致性比对
   → verify：以固定浏览器时区与出口时区断言一致/不一致两态
5. 实现 O3 IPv6 检测的对照实验（规格 2.3 判定表四行）
   → verify：四种组合各有一例断言；**特别断言「v4 不通 + v6 不通 → 检测失败」而非「IPv6 未启用」**
6. **验证未决事实 #1**：在具备 IPv6 的真实网络环境实测 `api6.ipify.org` 的 CORS 响应头
   → verify：结论写入本计划「验证记录」；若 CORS 不通，O3 必须改走规格 5.3 的降级方案，否则会把有 IPv6 的用户误判为安全
7. 实现首屏结论区：出口 IP、城市/国家、一句话定论、覆盖度三分、初步标注
   → verify：未点 O4 时结论区始终带「初步 · 未含 IP 风险评分」
8. 实现卡片流与五态；仅 CLI 项为灰卡，按语义位置穿插，带一键复制 `pip install ai-ipcheck`
   → verify：灰卡无重试入口，失败卡有重试入口
9. 实现 O4 按需卡片，按钮文案含「将把你的出口 IP 发送至 proxycheck.io 查询」（ADR-0008）
   → verify：文案存在性有测试断言，防止后续改版误删
10. O2 卡片文案区分 Claude 桌面版与 Claude Code CLI 的时区来源
    → verify：文案存在性断言
11. 视觉实现：ipinfo.io 式克制浅色、whoer.net 式首屏定论、每卡「这意味着什么」解释
    → verify：移动端视口下首屏结论区与覆盖度完整可见

## 验收标准

- 规格第 8 节验收标准第 1、2、3、4、5、10 条实测通过
- 未决事实 #1 有明确结论并记录在案
- 全部单测通过

## 验证记录

### 未决事实 #1：`api6.ipify.org` 的 CORS 响应头

**结论：判定 CORS 可用，O3 按原方案（ipify 双端点）实现；但该结论来自文档与旁证推断，未在具备 IPv6 的真实网络中实测，保留为待人工验证项。**

证据链（2026-08-10 取证）：

1. **同一份代码、同一层中间件对所有 ipify 主机名生效。** `rdegges/ipify-api` 的 `main.go` 只有一个 router、一个 handler，CORS 由全局中间件加在最外层：

   ```go
   // Setup middlewares.  For this we're basically adding:
   //	- Support for CORS to make JSONP work.
   handler := cors.Default().Handler(router)
   ```

   `rs/cors` 的 `cors.Default()` 即 `Access-Control-Allow-Origin: *`。中间件不按 Host 分流，因此 `api` / `api4` / `api6` 得到的 CORS 行为必然一致。

2. **v4 端点实测确有该响应头**：

   ```
   $ curl -sS -D - -o /dev/null -H "Origin: https://ipcheck.omnikit.run" "https://api.ipify.org?format=json"
   HTTP/2 200
   server: cloudflare
   access-control-allow-origin: *
   vary: Origin
   ```

   `Vary: Origin` 正是 `rs/cors` 的产物，说明这个头由源站生成，而非某层 CDN 补上的。

3. **两个端点靠 DNS 区分，且 v6 端点并不在 Cloudflare 后面**（DoH 查询）：

   | 主机名 | A | AAAA |
   |---|---|---|
   | `api.ipify.org` | `104.26.13.205` 等（Cloudflare） | 无 |
   | `api6.ipify.org` | 无 | `2607:f2d8:1:3c::4` |

   即 v4 对照端点确实只能走 IPv4（无 AAAA，对照实验成立），而 v6 端点直连源站——少了一层可能改写响应头的中间设施，反而更贴近上面第 1 条的源码行为。

4. ipify 官方站点未在文档中承诺 CORS，只给了 JSONP 示例；ADR-0003 已记载 `api.ipify.org` 的 CORS 实测可用。

**残余风险与已做的准备**：若真实 IPv6 环境下 v6 端点仍因任何原因不通（CORS、源站可用性、证书），浏览器抛出的仍是不可区分的 `TypeError`，判定表会落到「v4 通 + v6 不通 = IPv6 未启用」——即把有 IPv6 的用户判成安全，正是本未决事实要防的那个方向。为此：

- 探测层与判定层已拆开：`src/probes/ipify.ts` 只产出两个 `Probe`，`src/domain/ipv6.ts` 只吃 `Probe` 做判定。规格 5.3 的零依赖启发式降级只需换一个产出 `Probe` 对的实现，`judgeIpv6` 与状态机一行不动。
- 未预先实现两套探测——没有实测结论之前，第二套实现是投机。

**待人工验证（上线前）**：在具备原生 IPv6 的网络中，用浏览器打开任意站点的开发者工具执行

```js
await fetch("https://api6.ipify.org?format=json").then((r) => r.json());
```

或在终端执行 `curl -sS -D - -o /dev/null -H "Origin: https://ipcheck.omnikit.run" "https://api6.ipify.org?format=json"`，确认返回 200 且含 `access-control-allow-origin: *`。若不含，切换到规格 5.3 的降级探测实现。

### 步骤 11：移动端视口下的首屏可见性

`pnpm wrangler dev` + Playwright，视口 390 × 844（iPhone 14 等效 CSS 像素）实测：

| 测点 | 结果 |
|---|---|
| 结论区 `.verdict` 完整位于首屏内 | 是（`bottom` = 514px < 844px） |
| 覆盖度 `.coverage` 完整位于首屏内 | 是（`bottom` = 437px） |
| 横向溢出 | 无（`document.body.scrollWidth` = 390 = 视口宽） |

同一次会话内实测到的其他行为：

- 首屏无交互即呈现 O1–O3 结果、初步结论与覆盖度（验收标准 1）：卡片徽章为 `已完成 / 已完成 / 已完成 / 未开始`，结论区带「初步 · 未含 IP 风险评分」
- 灰卡无重试入口、有复制按钮；O4 失败后出现重试入口（步骤 8 verify）
- O4 按钮文案实测为「检测 IP 风险（将把你的出口 IP 发送至 proxycheck.io 查询）」（ADR-0008）
- 本机为纯 IPv4 网络，O3 实测判为「IPv6 未启用」，与判定表第二行一致

**待人工验证**：真机（非视口模拟）下的可见性；以及配置真实 `VITE_TURNSTILE_SITE_KEY` 后 O4 的完整链路（本次无 site key，点击后按设计降级为「人机验证组件未配置」）。

### 与规格 3.4 的偏差（需评审确认）

规格 3.4 写「已完成 X · 需 CLI Y · 检测失败 Z，X + Y + Z = 9」，但按需项 O4 在被触发前不属于这三档中的任何一档，检测中的项同理。实现改为四档并保持 `done + needCli + failed + pending ≡ 9`，`pending` 归零时三档之和自然恒为 9（已由单测覆盖两条不变量）。把未触发的 O4 计入「检测失败」会谎报故障，计入「需 CLI」则违反 ADR-0004 的「失败与需 CLI 必须分开呈现」。规格 3.4 的「典型态」示例（`已完成 3 · 需 CLI 5 · 检测失败 1（若 O3 失败）`）在任何读法下都凑不出 9，建议随本变更一并修正。
