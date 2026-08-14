# CLI 卸载支持：`preflight uninstall`

- 状态：**设计 spec（descriptive）**——驱动实现，落地后以代码为准
- 相关：[cli-guide.md](../cli-guide.md)（用户文档，随本 spec 落地更新）、[configuration.md](../configuration.md)（CLI 配置路径约定）、Cargo.toml `[workspace.metadata.dist]`（install-path 回退列表）
- 依赖：install-path 改为 XDG/`~/.local/bin` 回退列表（commit f7e779e）

## 1. 背景与目标

dist（cargo-dist）只生成 installer，不生成 uninstaller——上游没有这个功能。用户装完 preflight 后没有官方卸载路径，只能自己猜文件位置。

**目标**：一条 `preflight uninstall` 命令完成卸载；文档提供手动清理兜底。

**非目标**：

- 不清理 shell rc 中安装器写入的 PATH 行（改用户 rc 文件的风险大于收益，留着无害）；
- 不做 Windows 「程序和功能」注册表项（安装器本就没写）；
- 不动 dist 配置与发版链路——卸载完全在 CLI 内实现，零耦合。

## 2. 命令契约

```
preflight uninstall           # 删二进制 + install receipt；结束时提示配置文件仍在及删法
preflight uninstall --purge   # 额外删除用户配置目录
```

- **无交互确认**：用户敲的就是 uninstall，意图明确；重装只是一条 curl 命令，可逆。不读 stdin，脚本场景天然可用。
- 执行后向 stdout 打印删除清单（逐条实际路径）与重装命令（cli-guide.md §1 的一行 curl / irm）。
- 语种解析与其他命令一致（`--lang` / 配置 / 环境探测），文案在 `cli/src/copy/`，en 与 zh-hans 两份齐全（无字段级回落）。

## 3. 删除对象与路径

| 对象 | 路径 | 默认 | `--purge` |
|---|---|---|---|
| 二进制本体 | `std::env::current_exe()` | 删 | 删 |
| install receipt | Unix：`${XDG_CONFIG_HOME:-~/.config}/preflight/preflight-receipt.json`；Windows：`%XDG_CONFIG_HOME%\preflight\preflight-receipt.json`，未设 `XDG_CONFIG_HOME` 时 `%LOCALAPPDATA%\preflight\preflight-receipt.json`（与真实 installer.sh / .ps1 逐行核实，2026-08-14） | 删 | 删 |
| 用户配置目录 | `${XDG_CONFIG_HOME:-~/.config}/preflight/`（config.rs 既有推导，全平台同一约定） | 保留，结束时提示路径 | 删整个目录 |

注意：

- **Unix 上 receipt 与配置同目录**（`~/.config/preflight/`）。默认模式只删该目录下的 `preflight-receipt.json` 单个文件，不碰目录里其他内容；`--purge` 删整个目录。
- **Windows 上两者不同目录**（receipt 在 `%LOCALAPPDATA%`，配置走 config.rs 的 XDG 风格路径）。`--purge` 需两处都清：删配置目录，并在 receipt 与配置不同目录时把 receipt 所在的 `preflight` 目录一并删除。
- `PREFLIGHT_CONFIG` 指向的自定义配置文件**不碰**——用户显式管理的东西不动，`--purge` 只清默认目录。

## 4. 执行顺序与自删除

1. 删 receipt（不存在则静默跳过——`make cli-release` 从源码装的用户没有它）；
2. `--purge` 时删配置目录（不存在同样跳过）；
3. 最后删二进制自身，用 `self-replace` crate 的 `self_delete()`——Unix 上等价于 unlink 自身，Windows 上处理「运行中 exe 不能删自己」，两端一个代码路径。

自身放最后：若前面任一步失败，二进制还在，用户可以修复权限后重跑。

## 5. 错误处理与退出码

沿用 cli-guide.md §3.1 的退出码注册表，不新增：

- `0`：卸载完成（含 receipt / 配置本就不存在的情形）；
- `1`：任一删除因权限等原因失败——stderr 报具体路径与原因，已删的不回滚（重跑幂等：不存在即跳过）。

## 6. 依赖

`cli/Cargo.toml` 新增 `self-replace`（mitsuhiko 出品，uv / rye 同款）。唯一新依赖，无传染性 feature，不引 tokio。

## 7. 测试

- **集成测试**（tempdir 全隔离）：把 `env!("CARGO_BIN_EXE_preflight")` 拷入 tempdir，设 `XDG_CONFIG_HOME` 指向 temp 内伪造的 receipt 与 config.toml，分别跑 `uninstall` 与 `uninstall --purge`，断言：二进制消失、receipt 消失、配置按模式去留、退出码 0、输出含删除清单。跑在 cli.yml（ubuntu-latest，现有唯一矩阵）；Windows 行为依赖 `self-replace` 的跨平台保证，编译由 release 构建的 `x86_64-pc-windows-msvc` 目标兜住。
- **单元测试**：receipt 路径推导（XDG_CONFIG_HOME 设 / 未设、Windows LOCALAPPDATA 分支）。

## 8. 文档更新（随实现同 PR）

- cli-guide.md §3 命令总览加 `preflight uninstall [--purge]`；
- cli-guide.md 新增「卸载」小节：子命令用法 + 手动清理兜底（install-path 三个回退位置、receipt 路径、rc 中 PATH 行的说明与删法）。

## 9. 验收标准

- [ ] `preflight uninstall` 后：二进制与 receipt 消失，配置保留，退出码 0，输出含配置路径提示与重装命令；
- [ ] `preflight uninstall --purge` 后：配置目录一并消失；
- [ ] receipt / 配置不存在时不报错（源码安装用户、重复卸载均幂等）；
- [ ] 删除失败时退出码 1 且 stderr 指明路径；
- [ ] en / zh-hans 文案齐全；
- [ ] `x86_64-pc-windows-msvc` 目标编译通过（本地 `cargo check --target` 或 dist 构建验证；cli.yml 无 Windows 矩阵，不做集成测试要求）；
- [ ] cli-guide.md 两处更新完成。
