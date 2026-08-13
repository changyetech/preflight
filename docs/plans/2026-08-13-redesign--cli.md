# 子计划：CLI 人读报告重排

- 父计划：[2026-08-13-redesign.md](./2026-08-13-redesign.md)
- 规格依据：spec §5、§6；设计权威 `refs/cli-report-redesign.html`
- Depends on: locales

## 范围

`cli/src/render.rs`、`cli/src/copy/`（en + zh_hans 补键）。**不碰** `cli/src/domain/`、`cli/src/probe/`、`json.rs`。

## 步骤

1. **`--json` 冻结基线**：先为当前 `--json` 输出建快照/固定输入对比测试（若已有则确认覆盖）→ 验证：基线测试绿，作为全程回归锚。
2. **文案键补齐**（en + zh_hans 同步）：attention_label / attention_scope / risk_scale_note / reference_only / 各项状态词 / O1 值行标签 / C4 explain + fix，键名适配 `copy/mod.rs` 现有结构 → 验证：`cargo build` 绿（结构体强制两语种同步）。
3. **结论区扩容**：badge + 阶段标注 + summary + facts 对齐块（出口 IP·归属·ASN / 风险分 + 块字符刻度条 + `risk_scale_note` / 覆盖度 / 需关注清单 + scope 句）。「需关注」按 spec §5.2 从信号数据派生，不写死 → 验证：golden 场景下输出与原型结构一致；无 warn/bad 项时整块不出。
4. **检测卡重排**：标题行右端状态词、`=`/`≠` 比对符、说明 76 列悬挂缩进 + dim、`--verbose` 才出 description、C4 命中且时区名已知时输出 `export TZ=<IANA 名>` 修复行 → 验证：`--no-color` 下全部语义仍可读（状态词/缩进承载，无色彩依赖）。
5. **页脚提示行**（--verbose / --json / config set 配额提示），命令写法以实际子命令语法为准 → 验证：与 `cli/src/` 实际 CLI 接口一致。
6. 契约呈现约束核查：O1 标明数据来自 proxycheck、O2/C4 双条展示并标明谁进结论、失败项必渲染、`anonymous: true` 51–75 的「结论高·分项黄」解释在位 → 验证：对应单测。

7. **排版对齐原型**（spec §5.7）：分组标题 + 发丝线、卡内标签列对齐、终端列宽按全角两列计（折行与标点禁则）、状态词右对齐、档位徽章 → 验证：分组/对齐/禁则各有单测，`a_full_report_never_exceeds_76_columns` 覆盖两语种 × 彩色 × verbose。
8. **宽度跟随窗口**（spec §5.7 末条）：`render::Style::sized(color, columns)` 夹 `[40, 110]`，说明文字仍收 76；`main.rs` 用 `terminal_size_of(stdout)` 量一次，非终端落回 `Style::new`（固定 76） → 验证：窄窗（50）无行溢出、宽窗（120）整幅 110 而说明仍 76、超窄夹到 40 且分组标题不消失。

## 验收

- `make check-cli` 绿
- `--json` 与改版前逐字节等价（步骤 1 基线）
- 彩色 / `--no-color` / `--verbose` 三态均与原型改版视图结构一致
