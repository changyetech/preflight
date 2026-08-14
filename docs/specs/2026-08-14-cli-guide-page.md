# CLI 使用手册页面（/guide/）

- 状态：设计规格（descriptive）
- 底稿：[docs/cli-guide.md](../cli-guide.md)（CLI 使用手册，用户可读版全文）
- 相关：多页构建（vite.config.ts）、[2026-08-14-dns-list--web.md](../plans/2026-08-14-dns-list--web.md)（DNS 页的同构先例）、[2026-08-14-legal-pages.md](./2026-08-14-legal-pages.md)（法务页先例）

## 1. 目标

把 CLI 使用手册做成站内可访问的页面，让「刚复制完安装命令」与「在任意页面想查手册」的用户都有入口。

## 2. 决策

1. **路径**：`/guide/`（en）与 `/zh-hans/guide/`（zh-hans），沿用多页构建——两个新的 Vite 入口 HTML，各自正确的 `<html lang>` / `<title>` / meta description / canonical / hreflang 三条。未知子路径（如 `/guide/xxx`）保持真 404，不做 SPA 回退。
2. **内容**：完整手册（安装、快速开始、命令与参数、退出码、配置来源与优先级、proxycheck key、`--json`、使用场景），以 `docs/cli-guide.md` 为底稿改写成页面文案。en 为源语言，zh-hans 完整翻译，结构由 `Copy` 类型约束。判级规则不复述（契约红线），页面只谈「怎么用」。
3. **入口两处**：
   - 顶栏：与 DNS 清单入口并列的跨页链接（同样新标签打开 + `rel="noopener"`、窄屏不隐藏、在本页时 `aria-current="page"` 降色）。两个跨页链接合并进一个 `.nav-pages` 容器，替代原先单链接的 `.nav-dns` 贴右布局。
   - 首页安装区块（`#install`）：命令面板下方加一行「查看完整使用手册 →」链接，指向当前语种的 `/guide/`。
4. **文案结构**：`COPY.guide = { title, description, heading, lede, sections[], scenarios }`。`sections` 每项为统一形状 `{ heading, paras[], code[], table{headers[], rows[][]}, after[] }`（空数组表示该块缺席）——统一形状让 en/zh 的结构约束经得起 `Widen` 类型的展开。`scenarios` 为 `{ heading, items[{ title, body, code[] }] }`。
5. **样式**：新增 `.guide-main`（与首页 `.page`、DNS 页 `.dns-main` 同宽同内边距）+ 代码块与表格样式（表格视觉对齐 `.dns-table`）。安装区块的手册链接与顶栏跨页链接同一约定：新标签打开 + `rel="noopener"`。
6. **sitemap**：`public/sitemap.xml` 追加两条 URL。

## 3. 验收标准

- [ ] `/guide/` 与 `/zh-hans/guide/` 返回 200，`<html lang>` 与标题为对应语种；`/guide/xxx` 真 404（tests/i18n-routing.test.ts）
- [ ] 顶栏两个语种下都有指向本语种 `/guide/` 的入口，新标签 + noopener，本页时 `aria-current`（tests/nav.test.ts）
- [ ] 首页安装区块含指向本语种 `/guide/` 的链接（tests/landing.test.ts）
- [ ] 手册内的安装命令与 `COPY.actions.installCommand` / `installCommandWindows` 同源引用，不第二份字面量
- [ ] `make check` 全绿
