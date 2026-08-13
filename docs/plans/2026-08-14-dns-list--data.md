# 子计划：契约数据 `docs/dns-servers.json`

- 父计划：[2026-08-14-dns-list.md](./2026-08-14-dns-list.md)
- 规格依据：spec §2、§3
- Depends on: 无（先行）

## 范围

新增 `docs/dns-servers.json`；`cli/src/domain/` 新增注册表模块；改写 `cli/src/probe/dns.rs`（删除 `KNOWN_DNS` 常量与中英混排标签）；`CLAUDE.md` 索引补条目。**不碰** `cli/src/render.rs`、`main.rs`、`src/`（Web 侧由 `--web` 承接）。

## 步骤

1. **C2 行为冻结基线**：为 `probe/dns.rs` 现有的「已知服务商识别 + 私网识别 + 国内标记」建固定输入测试（若已覆盖则确认充分）→ 验证：基线绿，作为整个替换过程的回归锚。C2 的**外部行为必须逐字节不变**。
2. **写 `docs/dns-servers.json`**：以现有 27 条为底，按 spec §2.1 结构化为 `ip` / `name` / `region` / `domestic` / `variant`。品牌名去掉中文与地区后缀（`"AliDNS 阿里 (CN)"` → `name: "AliDNS"`, `region: "CN"`, `domestic: true`）→ 验证：条目数与现有常量一致，IP 集合完全相同。
3. **逐条核对 `variant`**：对照各服务商官方文档确认过滤级别（Quad9 默认拦恶意域名、Cloudflare `1.1.1.2` vs `1.1.1.3`、CleanBrowsing 各端点、AdGuard），**不得凭 IP 尾数猜测** → 验证：每条 `variant` 在 PR 描述里附官方文档链接。
4. **schema 校验测试**：字段齐全、`variant` 在四值枚举内、`ip` 可解析为 IPv4、无重复 IP、`region` 为两位大写 → 验证：故意构造的坏数据能让测试红。
5. **CLI 注册表模块**：`cli/src/domain/` 下新增模块，`include_str!` 读取 JSON，编译期/首次访问解析为查询结构（按 IP 查 + 全量遍历两种用法）→ 验证：`cargo build` 绿，按 IP 查询的单测覆盖命中/未命中。
6. **替换 `probe/dns.rs`**：删除 `KNOWN_DNS` 常量，改用注册表；`Server.label` 由 `Option<&'static str>` 改为承载结构化字段的形态，`domestic` 从注册表取 → 验证：步骤 1 的基线测试仍绿（**这是本子计划的核心验收**）。
7. **清理**：确认删除后无遗留的中英混排标签、无因本次改动产生的孤儿 import → 验证：`make check-cli` 绿。
8. **索引维护**：`CLAUDE.md` 的 Repository Structure 树里为 `docs/dns-servers.json` 补一行说明（与 `country-codes.json` / `verdict-cases.json` 并列）→ 验证：新增契约文件可从 CLAUDE.md 发现（项目强制规则）。

## 验收

- `make check-cli` 绿
- C2 检测项的输出与改动前逐字节等价（步骤 1 基线）
- `cli/src/probe/dns.rs` 中不再存在 `KNOWN_DNS` 与中英混排字符串
- `docs/dns-servers.json` 已在 `CLAUDE.md` 索引中
