# 子计划：语种收缩（5 → 2）

- 父计划：[2026-08-13-redesign.md](./2026-08-13-redesign.md)
- 规格依据：spec §3、ADR-0016
- Depends on: 无（先行）

## 范围

Web 与 CLI 的语种清单、入口、文案架构、以及锁五语种不变量的测试。不碰视觉（`--web` 的事）与 CLI 渲染结构（`--cli` 的事）。

## 步骤

1. **测试先行（TDD）**：重写 `tests/i18n-routing.test.ts`、`tests/locales.test.ts`、`tests/copy.test.ts`、`tests/landing.test.ts` 为两语种断言——`/` 与 `/zh-hans` 200、`/en` `/zh-hant` `/ru` `/ar` 真 404、hreflang 恰 3 条、无 RTL/回落断言 → 验证：新测试对现状**红**。
2. Web：`src/copy.ts` 收缩 `LOCALES`（删 `dir` 字段）、删 `PartialCopy`/`merge()`/`RESOLVED`；`zh-hans.ts` 类型改为完整 `Copy`；删 `src/locales/{zh-hant,ru,ar}.ts` → 验证：`tsc` 绿。
3. Web：删 `en/`、`zh-hant/`、`ru/`、`ar/` 入口目录；`index.html` 与 `zh-hans/index.html` 的 canonical/hreflang 改 3 条；`vite.config.ts` input 收缩为 2 → 验证：`make build` 后步骤 1 的测试绿。
4. CLI：`lang.rs` 枚举收缩为 `En | ZhHans`，`ArabicUnsupported` 并入统一「不支持的语言」错误（错误信息列出受支持值）；删 `is_fully_translated()` 与未译全提示；删 `copy/{zh_hant,ru}.rs` → 验证：`make check-cli` 绿。
5. 文档同步：`CLAUDE.md` 仓库结构/技术栈中的五语种描述、`docs/verdict.md` §5 若提及语种解析细节，改为两语种表述 → 验证：grep 无残留 `zh-hant|/ru|/ar` 的规范性表述（`refs/`、ADR 历史文除外）。

## 验收

- `make check-all` 绿
- `dist/client/` 只含 `index.html` 与 `zh-hans/index.html` 两个入口
- CLI `--lang ar` / `--lang ru` 报「不支持的语言」并列出 en/zh-hans
