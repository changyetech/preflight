# CLI 重写 · 判级契约的两端实现

- 父计划：[2026-08-12-cli-rust-rewrite.md](./2026-08-12-cli-rust-rewrite.md)
- 契约：[docs/verdict.md](../verdict.md) · [docs/verdict-cases.json](../verdict-cases.json)
- Depends on: `--foundation`

## 目标

在 Rust 侧实现判级契约，并让 `docs/verdict-cases.json` 成为**两端共同的判据**——两个实现各写一个参数化测试消费同一份文件。

## 范围

**做**：Rust 的检测项／信号／结论模型与 `compute_verdict`、覆盖度、Rust 侧参数化测试、**Web 侧新增的参数化测试**、Web 现有实现与契约的一致性核对。
**不做**：任何探测（信号从测试夹具输入）、任何渲染。

## 步骤

1. **Rust 领域模型**
   - 9 项检测项 ID 为枚举，**不用字符串**
   - 检测项状态：可在线／仅 CLI 的状态域互不相交（CLI 侧无「需 CLI」态）
   - 信号是**三态**：命中／未命中／未知。用 `Option<bool>` 或专用枚举，**不得**把未知塌缩成 `false`
   - `Verdict` 是 enum：`Insufficient`（**无 level 字段**）／`Preliminary { level: Low|Medium }`／`Full { level: Low|Medium|High }`
     - 「低风险」在 `Insufficient` 下必须**在类型层面构造不出来**，而不是靠约定
     - `Preliminary` 的 level 类型**不含 High**
   - → verify：`cargo build`；尝试构造 `Insufficient { level }` 应编译失败

2. **`compute_verdict`**
   - 按 [verdict.md §3](../verdict.md) 实现
   - 高档来源恒为 `riskScoreHigh`（风险分 ≥ 70），**闭区间**
   - 中档：`tzMismatchCliEnv` ／ `ipv6Leak` ／ `abuseListed` ／ `tunOff`
   - **`tzMismatchSystem` 不进 CLI 的综合结论**（[verdict.md §5.1](../verdict.md)）
   - → verify：步骤 4 的参数化测试

3. **覆盖度**
   - CLI 侧不变量：`已完成 X + 检测失败 Z = 9`；无「需 CLI」档、无「按需未测」档
   - 配额耗尽计入「检测失败」
   - → verify：单测断言不变量在各种失败组合下成立

4. **Rust 参数化测试消费 golden 向量**
   - 编译期 `include_str!("../../docs/verdict-cases.json")` 或运行时读取——**择一并固定**，理由写进代码注释
   - 只跑 `applies` 含 `cli` 的用例；`applies` 里出现未知值应**测试失败**而非跳过（防止拼写错误让用例静默失效）
   - → verify：12 条 CLI 用例全绿

5. **Web 参数化测试消费同一份向量**
   - 在 `tests/` 下新增 vitest 用例，只跑 `applies` 含 `web` 的用例
   - 需要一个适配器：把用例的扁平 `signals` 转成 `verdictInputFrom` 期望的嵌套结构（`signals` / `risk` 两个可空对象）
   - **注意语义映射**：`riskScore` 为 `null` ⇒ `risk` 整体为 `null`；`riskScore` 非 null 而 `abuseListed` 为 `null` ⇒ `risk` 存在、`abuseListed` 为 `null`
   - → verify：13 条 Web 用例全绿

6. **Web 现有实现与契约的一致性核对**
   - 逐条核对 `src/domain/verdict.ts` 与 [verdict.md](../verdict.md)。**不一致时改代码，不改契约**（契约是 normative）
   - 已知需确认项：`HIGH_RISK_SCORE` 与分项 `riskLevel` 阈值必须是**两个独立常量**（[verdict.md §6](../verdict.md)），当前虽同为 70 但语义不同
   - → verify：Web 侧全部既有测试 + 新增向量测试通过

7. **机制有效性验证（一次性）**
   - 临时把契约阈值从 70 改成 80，确认**两端测试同时变红**，然后改回
   - → verify：两端各观察到失败，`git diff` 干净

## 验收标准

1. 25 条用例在两端全绿（CLI 12 / Web 13）
2. `Insufficient` 携带 level 在 Rust 侧**编译不过**；`Preliminary` 的 level 类型不含 High
3. 覆盖度不变量有测试
4. 阈值改动会让两端同时变红（步骤 7 已实测）
5. Web 侧分项 `riskLevel` 与综合结论阈值是两个独立常量
6. `docs/verdict-cases.json` 出现在 `cli.yml` 与 `web.yml` 两条 workflow 的 paths 中（`web.yml` 属 `--release`，此处只登记）
