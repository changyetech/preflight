# 产品更名为 Preflight，命令名 `preflight`

`ipcheck` 描述的是「查一个 IP」，而本产品做的是**用敏感工具之前的网络环境体检**——8 个检测项、覆盖度、三档判级。名字把产品归到了「查 IP 的站点」那一类，与实际形态不符。我们决定在第一个 `cli/v*` tag 之前更名为 **Preflight**：产品名写作 `Preflight`，命令名、包名、Worker 名、配置目录一律小写 `preflight`，站点域名改为 `preflight.omnikit.run`。

「起飞前检查清单」这个隐喻与产品的使用时机和结构同时对上：多项、逐条、有覆盖度、通过才起飞。它替代的是一个在 GitHub / crates.io / npm / PyPI / 域名上全是红海、且无法品牌化的通用词。

## Considered Options

- **保留 `ipcheck`**。否决：名不副实之外还有一个硬约束——它在所有注册表与主流域名后缀上都已被占，拿不到任何可主张的命名空间。更名的成本随第一个 release tag 落地而陡增（Homebrew formula 名、installer 脚本 URL、`releases/latest` 路径一旦发布就被外部引用钉死），此刻是成本最低的窗口。
- **`Egress` / `Vantage` 等更独特的词**。否决：可搜索性确实更好，但需要一句话解释才能让用户知道这是什么。Preflight 牺牲搜索换直觉——见下方「代价」。
- **加限定符（`netpreflight`、`preflight-net` 等）**。否决：搜索引擎会把复合词拆开，SEO 一分不赚，却牺牲了命令名的简短。限定符由域名 `preflight.omnikit.run` 承担即可。
- **改写历史文档中的旧名**。否决：`docs/adr/`、`docs/plans/`、`docs/specs/` 是 point-in-time 记录，正文保持写作当时的名字。改写会抹掉更名这件事在历史里的痕迹，并让 ADR-0003 中「`ipcheck.omnikit.run` 已验证 Universal SSL 覆盖一级子域」这类**实测事实**变成一个从未存在过的域名。

## Consequences

- **命令名 `preflight`**：`$PATH`、Homebrew core formula 与 cask 均无冲突（更名时实测）。crates.io / npm / PyPI 上的同名包均为废弃小包，且本仓库不向这三处发布，不构成阻塞。
- **标识符全量更名**：Cargo package 与 bin 名、Worker 名（`wrangler.jsonc`）、`package.json` 的 `preflight-web`、构建产物目录 `dist/preflight/`、配置目录 `~/.config/preflight/`、环境变量 `PREFLIGHT_CONFIG`、localStorage 键 `preflight-theme`、`Makefile` 的 `APP_NAME`。
- **`preflight-theme` 使旧主题偏好失效**：键名变更后，老访客的 localStorage 里 `ipcheck-theme` 不再被读取，主题回落到系统默认。这是纯外观偏好、无隐私载荷（[ADR-0016](./0016-two-locales-en-zh-hans.md)），不做迁移。
- **历史文档正文保留旧名**，但 `2026-08-10-ipcheck-web*.md` 六份文档的**文件名**重命名为 `2026-08-10-preflight-web*.md`，全仓库交叉引用同步更新。
- **`ai-ipcheck` 不受影响**：它是已归档的 Python 前身，命令名本就是 `ipcheck`。契约 golden 向量中的 `aiIpcheckDiverges` 键名一并保留——它指代的是那份历史实现，不是本产品。
- **`refs/ipcheck/` 与 `refs/ipcheck-web-redesign.html` 保持原名**：只读参考素材，不属于本产品的命名空间。
- **代价（诚实记录）**：`preflight` 在 Web 开发圈的第一含义是 CORS preflight request，自然搜索基本让不出位置。我们接受它——导流依靠域名与仓库直链，不指望搜索引擎。同时 `SpectralOps/preflight`（一个校验脚本与可执行文件的安全工具）是最近的语义邻居，认知上存在混淆风险，但不占用任何我们需要的命名空间。
