# 子计划：Web 视觉/排版/主题重做

- 父计划：[2026-08-13-redesign.md](./2026-08-13-redesign.md)
- 规格依据：spec §4、§6；设计权威 `refs/ipcheck-web-redesign.html`
- Depends on: locales

## 范围

`src/index.css`、`src/App.css`、`src/App.tsx`、`src/components/*`、两个入口 HTML 的 `<head>`。**不碰** `src/domain/`、`src/usePanel.ts` 的状态机语义、`src/probes/`、`worker/`。

## 步骤

1. **Token 层替换**：`index.css` 换为原型的 oklch 双主题 token（`:root` + `:root[data-theme="dark"]`），删 `prefers-color-scheme` 媒体查询驱动 → 验证：两主题下所有旧变量引用无悬空（build + 全页目测）。
2. **主题机制**：防闪白内联脚本进两个入口 `<head>`;新增主题切换组件（三态下拉，`ipcheck-theme` localStorage,`data-theme-pref`/`data-theme` 双属性，跟随系统时监听 `prefers-color-scheme` 变化）→ 验证：三态切换、刷新不闪白、跨页保持。
3. **布局重排**（对照原型逐块）：导航（brand + 5 锚点 + nav-tools）、masthead、结论控制台（console 双栏：出口 IP + geo-grid ｜ 结论 + 覆盖度 meter + chips）、O1–O6 卡片流（auto-fill 栅格、pill 状态、kv/result/note/meaning 结构）、C1–C4 发丝线列表（删 `CliCard` 卡片形态）、landing 三段（why 四栏 / install 双栏 + 复制键 / compare 表 + 移动端堆叠卡）、页脚双栏披露、回顶按钮 → 验证：与原型逐区目测比对（浅色/深色 × 桌面/640px/390px）。
4. **语言切换器**改单键直切（显示另一语种名，href 指对应路径）→ 验证：两页互跳正确。
5. **文案键补齐**：原型新增结构性文案（console 标签、主题菜单、覆盖度图例等）进 `en.ts` + `zh-hans.ts`；既有文案一律以现网为准 → 验证：`tsc` 绿（类型强制两语种同步）。
6. **可达性**：skip link、结论区 `aria-live` `aria-atomic`、卡片 `aria-busy`、覆盖度 meter `role="img"` + 动态 aria-label、焦点环、reduced-motion → 验证：键盘走查 + 测试补断言。
7. demo-only 元素确认未落地（示例数据标记、假时间线、演示判级）→ 验证：grep 原型专有字符串无命中。

## 验收

- `make check` 绿
- spec §8 条 2、3、5（Web 侧）满足
- O1–O6 标题与 CLI 侧在 en/zh-hans 下逐字一致（契约 §1.1，抽查）
