# ipcheck Web API 契约

- 状态：**契约（normative）**——代码与本文不一致时，以本文为准（见 CLAUDE.md「Authoritative Source」）
- 实现规格：[docs/specs/2026-08-10-ipcheck-web.md](./specs/2026-08-10-ipcheck-web.md) 第 5 节
- 通用规约：字段名 camelCase、`Content-Type: application/json; charset=utf-8`、响应信封 `{code, message, data}` / `{code, message, details}`（见 `code-conventions` skill 的 http-constitution 与 error-codes）

本站只有两个接口，均无鉴权、无分页、无版本前缀（单体前端专用，不对外承诺兼容性以外的东西）。

## 1. 响应信封

成功：

```json
{ "code": 0, "message": "ok", "data": {} }
```

失败：

```json
{ "code": 2010, "message": "human verification failed", "details": "missing turnstile token" }
```

`code` 是业务错误码，与 HTTP 状态码是两套东西；见第 4 节注册表。

## 2. `GET /api/geo`

O1 出口 IP 与归属。数据全部由本次请求的 `request.cf` 派生，不调用任何第三方，无配额、无限流、无人机验证。

**请求**：无参数、无请求体。

**响应 200**：

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "ip": "1.2.3.4",
    "country": "CN",
    "region": "Shanghai",
    "city": "Shanghai",
    "postalCode": null,
    "continent": "AS",
    "latitude": "31.22222",
    "longitude": "121.45806",
    "timezone": "Asia/Shanghai",
    "asn": 4134,
    "asOrganization": "Chinanet",
    "colo": "SHA"
  }
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `ip` | `string \| null` | 出口 IP，取自 `CF-Connecting-IP`。**不是「真实 IP」**（见 CONTEXT.md），代理背后的地址本站看不到 |
| `country` | `string \| null` | ISO 3166-1 alpha-2 |
| `region` / `city` / `postalCode` | `string \| null` | |
| `continent` | `string \| null` | 两字母洲代码 |
| `latitude` / `longitude` | `string \| null` | Workers 原样给出字符串，不转数字 |
| `timezone` | `string \| null` | IANA 时区名，供前端与浏览器时区比对（O2） |
| `asn` | `number \| null` | |
| `asOrganization` | `string \| null` | |
| `colo` | `string \| null` | 接入的 Cloudflare 数据中心（诊断用） |

**字段缺失一律降级为 `null`，键恒存在**——前端可以无条件访问字段，不必做存在性判断。本接口不会因为地理数据缺失而返回错误。

## 3. `POST /api/risk`

O4 IP 类型与风险。按需触发，串联 proxycheck v3 与 StopForumSpam 两个第三方（ADR-0007 / ADR-0008）。

**请求体**：

```json
{ "turnstileToken": "0.abc..." }
```

| 字段 | 必填 | 说明 |
|---|---|---|
| `turnstileToken` | 是 | Turnstile 前端组件产出的 token |

> **`/api/risk` 不接受、也不读取任何客户端传入的 IP。** 查询目标恒为本次请求的来源 IP（`CF-Connecting-IP`）。请求体或查询串里出现 `ip` 之类的字段一律被忽略，既不改变查询目标，也不产生错误。这是硬约束：否则本站会退化成一个任意 IP 查询代理，proxycheck 配额会被第三方白嫖（规格 5.1 / 验收标准 6）。

**处理顺序**：Turnstile 校验 → Rate Limiting → DO 日配额 → proxycheck v3 + StopForumSpam → 组装响应。任一前置环节拒绝，都不会消耗后续环节的资源。

### 3.1 响应 200 · `status: "ok"`

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "status": "ok",
    "ip": "1.2.3.4",
    "networkType": "Hosting",
    "proxy": false,
    "vpn": true,
    "tor": false,
    "scraper": false,
    "riskScore": 100,
    "riskLevel": "high",
    "abuseListed": true
  }
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `status` | `"ok" \| "quotaExhausted"` | 判别式，见 3.2 |
| `ip` | `string` | 实际被查询的来源 IP，回显以便前端核对 |
| `networkType` | `"Residential" \| "Business" \| "Wireless" \| "Hosting" \| null` | 取 proxycheck `network.type`；`null` = 未知 |
| `proxy` / `vpn` / `tor` / `scraper` | `boolean` | 取 proxycheck `detections.*` |
| `riskScore` | `number` | 0–100，取 proxycheck `detections.risk` |
| `riskLevel` | `"low" \| "medium" \| "high"` | 由 `riskScore` 分级：`< 30` low，`< 70` medium，`>= 70` high（规格 3.2） |
| `abuseListed` | `boolean \| null` | StopForumSpam 是否有滥用收录；`null` = 该第三方不可用，前端应显示「未知」而非「无收录」 |

`abuseListed` 为 `null` 时本接口仍返回 200：StopForumSpam 只贡献「中」风险信号，它挂掉不应连累 proxycheck 的结果。

**响应中永不包含 proxycheck API key，也不包含 proxycheck 的原始响应体。**

### 3.2 响应 200 · `status: "quotaExhausted"`

```json
{ "code": 0, "message": "ok", "data": { "status": "quotaExhausted" } }
```

当日 proxycheck 配额（1,000 次／天，UTC 日切）已被 Durable Object 守卫判定用尽。

这**不是错误**：HTTP 200、`code: 0`。O4 是一个可选加项，配额用尽属于预期内的容量状态，前端据此把 O4 卡片显示为「今日额度已用尽」并计入「检测失败」，O1–O3 与初步结论完全不受影响（规格 5.3 / 验收标准 8）。用错误码表达会诱导前端把它当故障重试，反而放大问题。

此形态下 `data` 只有 `status` 一个键——没有查询发生，就没有结果可报。

### 3.3 错误响应

| HTTP | `code` | 触发条件 |
|---|---|---|
| 400 | 1001 | 请求体不是合法 JSON |
| 403 | 2010 | 缺失 `turnstileToken`，或 Turnstile siteverify 判定 token 无效 |
| 429 | 2020 | 触发 Rate Limiting 绑定的单 IP 限流 |
| 500 | 5001 | proxycheck 不可用（网络失败、非 200、`status != "ok"`） |
| 500 | 5002 | 无法确定来源 IP（`CF-Connecting-IP` 缺失） |

`details` 只放对用户/前端有意义的短语，绝不回填第三方的原始错误文本或任何密钥。

## 4. 错误码注册表

沿用通用码段（1000–1999 参数 / 2000–2999 认证 / 5000+ 系统），本项目自有码如下。新增错误码必须先登记在此表。

| 码 | HTTP | `message` | 触发 |
|---|---|---|---|
| 0 | 200 | `ok` | 成功 |
| 1001 | 400 | `parameter error` | 请求体解析失败 |
| 2010 | 403 | `human verification failed` | Turnstile token 缺失或校验不通过 |
| 2020 | 429 | `too many requests` | 单 IP 限流 |
| 5001 | 500 | `upstream unavailable` | proxycheck 调用失败 |
| 5002 | 500 | `client ip unavailable` | 拿不到来源 IP |

## 5. 隐私约束（ADR-0002 / ADR-0008）

- 两个接口都**不写任何持久化存储**。Durable Object 里只有「日期 + 当日计数」两个标量，不含 IP、不含任何用户数据。
- 不设 KV、不设 D1、不设结果缓存。
- `PROXYCHECK_API_KEY` 与 `TURNSTILE_SECRET_KEY` 只存在于 Worker Secret，不进仓库、不进响应、不进日志。
- 新增任何第三方调用，必须同步更新第 3 节的第三方清单，并在触发它的前端控件上标注。
