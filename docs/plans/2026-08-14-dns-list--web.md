# 子计划：Web 页面 `/dns/`

- 父计划：[2026-08-14-dns-list.md](./2026-08-14-dns-list.md)
- 规格依据：spec §5、§6、§7
- Depends on: data

## 范围

新增 `dns/index.html` + `zh-hans/dns/index.html`、页面组件与样式、`vite.config.ts` 多页 input；改造 `src/copy.ts` 的 `LOCALES.path` 与 `src/components/LangSwitch.tsx`；sitemap / hreflang；`tests/i18n-routing.test.ts` 扩充。**不碰** `src/domain/`、`src/usePanel.ts`、`src/probes/`、`worker/`（无新增端点）。

## 步骤

1. **路由基线**：确认 `tests/i18n-routing.test.ts` 现有覆盖（两语种入口命中、未知路径真 404）→ 验证：基线绿。**真 404、不做 SPA 回退**这条红线在本子计划全程不得松动。
2. **`LOCALES.path` 改语种前缀**：`path` 由 `"/"` / `"/zh-hans"` 改为 `""` / `"/zh-hans"`，新增由「语种前缀 + 页面 slug」拼 URL 的派生函数；`LangSwitch` 接受当前页 slug → 验证：首页切换行为不变（回归），`/dns/` 切换落到 `/zh-hans/dns/`（这是 spec §5.3 记录的**既有缺陷**修复）。
3. **两个 HTML 入口**：`dns/index.html`、`zh-hans/dns/index.html`，各自的 `<html lang>` / `<title>` / description 独立撰写（独立搜索意图，不复用首页文案）；防闪白内联脚本比照现有入口 → 验证：`make build` 产出 `dist/client/dns/index.html` 与 `dist/client/zh-hans/dns/index.html`。
4. **`vite.config.ts`** 多页 input 补两条 → 验证：构建产物布局正确。
5. **页面内容**：静态表（五列，数据来自 `docs/dns-servers.json`，打进 bundle）+ 引导段「要测你这台机器上实际可用的，用 `preflight dns --check`」+ 返回首页链接 → 验证：两语种页面文案完整（无字段级回落）。
6. **不实测的显式约束**：页面**不得**发起任何 DoH/HTTPS 探测。理由见 spec §5.2（浏览器发不出 DNS 查询，能测的少数端点测的也不是真实路径，误导性结论比没有更糟）→ 验证：页面无对外请求（网络面板为空）。
7. **窄屏表格**：宽内容须在自身 `overflow-x: auto` 容器内滚动，页面 body 绝不横向滚动（此前修过同类问题，commit `d065a1c`）→ 验证：窄屏无右侧溢出。
8. **尾斜杠 301**：`/dns` → `/dns/`。优先验证 Static Assets 内建 html_handling 是否已提供该重定向；不满足再在 `worker/index.ts` 补一条 301（**仅此一条**，不引入通用重写逻辑）→ 验证：测试断言 301 及 Location。
9. **sitemap + hreflang**：两个新 URL 进 sitemap；两页各自声明 `en` / `zh-Hans` / `x-default` 三条 hreflang，互指同一 slug → 验证：产物中 hreflang 指向正确、无指回首页。
10. **导航入口**：首页提供一个通往 `/dns/` 的链接（位置与措辞落地时定，不新增顶部导航项以免稀释首页的检测意图）→ 验证：两语种均可达。
11. **路由测试扩充**：`/dns/`、`/zh-hans/dns/` 命中；`/dns` 301；`/dns/xxx`、`/zh-hans/dns/xxx` 仍**真 404** → 验证：新增用例全绿，基线用例未回归。

## 验收

- `make check` 绿（注意 `make test` 前需先 `make build`，路由测试依赖 `env.ASSETS` 读真实产物）
- `/dns/` 与 `/zh-hans/dns/` 可达并渲染完整清单；`/dns` 301；未知子路径真 404
- 在 `/dns/` 切换语言落到对应语种的**同一页**，且首页切换行为无回归
- 页面无任何对外网络请求
- 窄屏无横向溢出
- `worker/` 除可能的一条 301 外无改动，`docs/api.md` 未改动
