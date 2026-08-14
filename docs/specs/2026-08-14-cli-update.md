# CLI 自更新：`preflight update`

- 状态：**设计 spec（descriptive）**——驱动实现，落地后以代码为准
- 相关：[cli-guide.md](../cli-guide.md)（用户文档，随本 spec 落地更新）、[deployment.md](../deployment.md)（§0「本仓库只有 CLI 发 Release」红线——版本探测依赖它）、[2026-08-14-cli-uninstall.md](./2026-08-14-cli-uninstall.md)（同类生命周期子命令的先例：分发时机、receipt 路径、文案结构）
- 依赖：Release 资产含 `dist-manifest.json` 与 `preflight-installer.sh` / `.ps1`（cli/v0.2.0 实测核实，2026-08-14）

## 1. 背景与目标

dist 配置 `install-updater = false`——没有独立的 `preflight-update` 二进制，用户更新只能重新跑一遍安装命令，且没有「是否有新版」的探测手段。

**目标**：一条 `preflight update` 完成检查 + 更新——已是最新则报告后退出，有新版则下载并执行官方 installer。

**非目标**：

- 不加 `--check`（只查不装）——要了再加；
- 不服务源码安装（`make cli-release` / `cargo install --git`）——无 receipt 时拒绝并提示（见 §4），避免 installer 把新版装进 `~/.local/bin` 与旧二进制在 PATH 上互相遮蔽；
- 不做自动检查（体检时顺带提示新版）——update 只在用户显式敲它时联网；
- 不改 dist 配置（`install-updater` 保持 `false`），不引入独立 updater 二进制，因此无需重跑 `dist init`；
- 不处理「安装后手动把二进制挪走」的布局——installer 的 install-path 解析是确定性的（`$XDG_BIN_HOME` → `$XDG_DATA_HOME/../bin` → `~/.local/bin`），更新落回原位；挪走过的副本只按 §5 的守卫兜底，不追踪。

## 2. 命令契约

```
preflight update    # 已最新：打印当前版本，退出 0；有新版：下载执行官方 installer
```

- **无交互确认**：用户敲的就是 update，意图明确；不读 stdin，脚本场景天然可用。
- 输出（stdout）：已最新时一行「已是最新版本 vX.Y.Z」；更新时先打「vA.B.C → vX.Y.Z」，installer 的 stdout/stderr **直通继承**（下载进度可见），结束打「已更新到 vX.Y.Z」。
- 语种解析与 uninstall 一致（`--lang` + 系统 locale，不读配置文件，见 §6）；文案在 `cli/src/copy/`，en 与 zh-hans 两份齐全（无字段级回落）。

## 3. 版本探测

- 数据源：`GET {base}/dist-manifest.json`，`base` 默认 `https://github.com/changyetech/preflight/releases/latest/download`。
  - **不走 api.github.com**——匿名限流每小时 60 次；`releases/latest/download` 端点无 API 限流，且与安装通道同域名，代理环境行为一致。
  - `releases/latest` 恒指向 CLI 最新版依赖 deployment.md §0 的红线「本仓库只有 CLI 发 GitHub Release」。
  - 环境变量 `PREFLIGHT_UPDATE_BASE` 可覆盖 base——测试接缝（集成测试指向本地假服务），不写入用户文档。
- 解析：取 `releases[]` 中 `app_name == "preflight"` 项的 `app_version`（schema 对 cli/v0.2.0 的真实资产核实）。
- 比较：`x.y.z` 三段数值比较（不引 semver crate——dist 版本恒为纯三段，无预发布号）。**远端 > 本地**才更新；相等或更旧一律报「已是最新」退出 0（Release 被撤到旧版时不做降级）。远端版本解析不出来按失败处理（退出 1），不猜。
- HTTP：复用 `probe::http::agent`（ureq + rustls 平台信任库 + 全局超时）。超时用内置默认 10 秒，不读配置（§6）；它只管 manifest 与 installer 脚本两个小请求，二进制下载发生在 installer 内部、不设超时。

## 4. receipt 守卫

复用 `uninstall::receipt_path` 的推导（与真实 installer 逐行对齐）。receipt 不存在 → 退出 1，提示「非官方 installer 安装，请按当初的安装方式更新」。与 axoupdater 行为一致。守卫在联网之前。

## 5. 更新执行

1. 下载 `{base}/preflight-installer.sh`（Windows：`.ps1`）到临时目录；
2. **rename-aside**：把 `current_exe` 改名为同目录 `preflight.old`（原文件名 + `.old` 后缀）。两平台统一做：
   - Windows 是硬需求——运行中的 exe 锁写，installer.ps1 的 Copy-Item 会直接失败；rename 运行中的 exe 是允许的；
   - Unix 上当前 installer.sh 用 `mv` 落盘（实测第 793 行）本无冲突，但那是 dist 的实现细节，哪天改成 `cp` 就是 Linux 上的 ETXTBSY——rename-aside 让两平台一个代码路径，且天然获得失败回滚；
3. 执行 installer（Unix：`sh <脚本>`；Windows：`powershell -NoProfile -ExecutionPolicy Bypass -File <脚本>`），stdio 继承；receipt 与 PATH 行由 installer 自己维护（幂等）；
4. 收尾：
   - installer 退出 0 且原路径上已有新二进制 → 删除 `.old`（Unix 上 unlink 运行中的 inode 无害；Windows 上仍被本进程锁定，删除失败则留待下次清理）；
   - installer 退出 0 但原路径上**没有**新二进制（installer 落到了别处，见 §1 非目标末条）→ 把 `.old` 改名回来，新旧并存，不删任何东西；
   - installer 非零退出 → 把 `.old` 改名回来，退出 1。
5. **Windows 残留清理**：任何一次 `preflight` 启动时静默尝试删除同目录 `preflight.exe.old`（失败忽略）——uv / axoupdater 同款手法。

## 6. 分发时机

与 uninstall 一致，**先于配置文件加载**分发：配置文件是 deny_unknown_fields，若某个版本把配置解析弄出过严的 bug，`preflight update` 正是修复通道，它自己不能被坏配置锁死。语言从 `--lang` + 系统 locale 解析（拿默认 `ConfigFile` 走同一条 `Settings::resolve`）；副作用是超时恒为内置默认 10 秒、不读配置文件的 `timeout`——对两个小请求足够。

## 7. 错误处理与退出码

沿用 cli-guide.md §3.2 的退出码注册表，不新增：

- `0`：已是最新，或更新成功；
- `1`：无 receipt / manifest 拉取或解析失败 / installer 下载失败 / installer 非零退出。失败时 `.old` 已按 §5.4 回滚，二进制不丢。

## 8. 依赖

零新增。ureq（探测层既有）、`serde_json`（既有）、`std::process::Command`。

## 9. 测试

- **单元测试**（`update.rs` 内）：版本解析（`x.y.z`、带 `v` 前缀、非法输入）、新旧比较（大于才更新）、manifest 解析（真实 schema 样本、缺字段、非 JSON）。
- **集成测试**（`cli/tests/update.rs`，Sandbox 模式同 uninstall）：测试内用 `std::net::TcpListener` 起本地假 HTTP 服务，`PREFLIGHT_UPDATE_BASE` 指向它，全离线：
  - 同版本 manifest → 输出「已是最新」、二进制未动、退出 0；
  - 高版本 manifest + 假 installer.sh（写一个新文件后 `mv` 到原路径）→ 二进制被替换、`.old` 不残留、退出 0（仅 Unix，CI 是 ubuntu）；
  - 假 installer 非零退出 → 二进制仍在原路径（`.old` 已回滚）、退出 1；
  - 无 receipt → 不发任何请求即退出 1，stderr 含提示；
  - manifest 404 → 退出 1。
  - Windows 行为（rename 运行中 exe、`.old` 清理）编译由 release 的 `x86_64-pc-windows-msvc` 目标兜住，不做集成测试要求。

## 10. 文档更新（随实现同 PR）

- cli-guide.md：§1 新增「更新」小节（1.1 卸载之前或之后均可，含源码安装不适用的说明）；§3 命令总览加 `preflight update`；
- README.md：安装章节加一行更新说明；
- Web 手册（`src/locales/en.ts` / `zh-hans.ts` 的 CLI 页）：命令清单加 `preflight update` 一行，命令数措辞同步（four → five）；
- CLAUDE.md 仓库树：`cli/src/` 下加 `update.rs` 一行。

## 11. 验收标准

- [ ] 已是最新时：打印当前版本，退出 0，二进制未动；
- [ ] 有新版时：执行官方 installer，成功后二进制为新版、`.old` 不残留（Unix）、退出 0；
- [ ] installer 失败时：`.old` 回滚，二进制可继续使用，退出 1；
- [ ] 无 receipt（源码安装）时：不联网，退出 1，提示按原方式更新；
- [ ] manifest 拉取/解析失败时：退出 1，stderr 指明原因；
- [ ] 坏配置文件不影响 `preflight update`（先于配置加载分发）；
- [ ] en / zh-hans 文案齐全；
- [ ] `x86_64-pc-windows-msvc` 目标编译通过；
- [ ] §10 四处文档更新完成。
