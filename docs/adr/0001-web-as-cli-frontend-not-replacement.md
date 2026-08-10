# ipcheck Web 定位为 CLI 门面 + 轻量在线体检，不复刻仅 CLI 项

ipcheck CLI 的 8 个检测项中，本地 DNS 服务器、系统代理/TUN 开关、`$TZ`、`ANTHROPIC_BASE_URL` 四项依赖本机环境，浏览器结构性地拿不到。我们决定 ipcheck Web 只做可在线检测项，仅 CLI 项明确标注"需要 CLI"并引导安装，而不是用浏览器侧技术硬补。

## Considered Options

- **在线替代品** — 用 WebRTC 探测真实/内网 IP、用随机子域探测 DNS 解析器来复刻本地项。否决：现代浏览器已用 mDNS 混淆 ICE candidate，WebRTC 泄露检测大面积失效；给这类用户假的"安全"信号，比不做更有害。Claude 端点检测（读本机环境变量）则理论上无解。
- **独立通用工具站** — 不绑 CLI 自行演进。否决：CLI 已是主产品且发到 0.3.1，网站的边际价值在零门槛试用与引流，不在再造一个只能做一半的替身。

## Consequences

DNS 泄露检测在纯 Cloudflare 栈上另有硬约束：正统做法需自建权威 DNS 记录解析器来源，而 Workers 拿不到 DNS 查询日志、CF DNS 也不会把查询转给 Worker。该项一并划归仅 CLI 项，网站不接第三方黑盒来假装支持。
