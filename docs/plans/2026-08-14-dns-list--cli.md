# 子计划：CLI `preflight dns`

- 父计划：[2026-08-14-dns-list.md](./2026-08-14-dns-list.md)
- 规格依据：spec §4、§6、§7
- Depends on: data

## 范围

`cli/src/main.rs`（子命令）、`cli/src/probe/` 新增 DNS 实测模块、`cli/src/render.rs`（新增表格视图）、`cli/src/json.rs`（新增独立 schema）、`cli/src/copy/`（en + zh_hans 同步补键）、`cli/Cargo.toml`。**不碰** `cli/src/domain/verdict.rs`、`checks.rs`、体检主路径的任何渲染。

## 步骤

1. **体检输出冻结基线**：确认现有 `--json` 与人读报告的快照/固定输入测试覆盖充分 → 验证：基线绿。本子计划结束时体检输出必须**逐字节不变**。
2. **子命令骨架**：`Command` 枚举加 `Dns { check: bool }`，接 `--lang` / `--json` 两个全局 flag；退出码沿用既有约定（成功 `0`，工具失败 `1`；`EXIT_NO_VERDICT` 与本命令无关，不复用）→ 验证：`preflight dns --help` 文案正确，`preflight` 无子命令仍走体检。
3. **静态表渲染**：五列 IP · 提供商 · 地区 · 国内 · 用途，清单原序，复用 `render::Style::sized` 的宽度自适应 → 验证：宽窗（120）、标准（76）、窄窗（50）、`--no-color` 四态无行溢出且语义完整（状态/标记不依赖色彩）。
4. **文案补键**（en + zh_hans 同步）：4 个 variant 词、国内标记词、3 个 `--check` 状态词、列名、页脚引导句 → 验证：`cargo build` 绿（Copy 结构体强制两语种同步）。
5. **引入 `simple-dns`**：`cargo add simple-dns`，随后 `cargo tree` 核对**未引入任何 async 运行时** → 验证：`cargo tree | grep -i tokio` 无输出，记入 PR 描述。
6. **DNS 查询与判据**（TDD，先测后写）：以固定字节向量驱动应答解析，覆盖 NOERROR+公网 A（通）、NXDOMAIN（不通）、A 记录为私网地址（应答可疑）、TXID 不匹配（丢弃）、无应答超时（不通）→ 验证：五个场景各一测，全部不打真实网络。
7. **并发实测**：`std::thread::scope` 逐条并发查询（**不引 tokio**），超时与 socket 写法参照 `probe/stun.rs` → 验证：全表实测的墙钟时间接近最慢一条而非累加。
8. **`--check` 视图**：追加延迟 · 状态两列；按延迟升序重排；窄屏时把地区折进提供商列（渲染为 `Cloudflare (US)`，数据层不变）→ 验证：排序单测（含「不通」条目排末尾）、窄屏折叠单测。
9. **`--json`**：spec §4.6 的独立 schema，`check` 键仅在 `--check` 时出现（不是 `null`）→ 验证：两种形态各一测；体检 `--json` 的基线仍绿。
10. **判级隔离核查**：确认 `dns` 命令未引用 `domain/verdict.rs`、未注册检测项、未改 `docs/verdict.md` → 验证：`docs/verdict.md` 在本 PR 的 diff 中为空。
11. **两端差异登记**：在契约的两端差异登记表补一笔「CLI 有 `dns` 命令与 `--check`，Web 只有静态清单页」——登记**能力差异**，非判据差异 → 验证：登记表已更新。

## 验收

- `make check-cli` 绿
- 体检人读报告与 `--json` 与改动前逐字节等价（步骤 1 基线）
- `cargo tree` 无 async 运行时
- 应答为私网地址的模拟劫持被标为「应答可疑」而非「通」
- `docs/verdict.md` 未改动
