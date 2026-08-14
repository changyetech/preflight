# Preflight CLI 使用手册

- 状态：**使用指南（descriptive）**——面向 CLI 用户，回答「怎么用」。判级规则本身见 [verdict.md](./verdict.md)（normative，本文不复述）
- 相关：[configuration.md](./configuration.md)（CLI 配置键的权威清单）、[deployment.md](./deployment.md)（发版流程）、[ADR-0012](./adr/0012-cli-direct-third-party-not-worker-api.md)（CLI 直连第三方，不走本站 API）

Preflight CLI 在终端里对当前网络环境做一次体检：出口 IP 归属、时区一致性、IPv6 / DNS / UDP 泄露、IP 风险与滥用收录、本机代理与 DNS 配置等共 10 项（O1–O6 + C1–C4，注册表见 [verdict.md §1](./verdict.md)），最后给出一个带覆盖度的综合结论。

不存储任何检测结果，探测直连第三方（ipify / STUN / proxycheck / StopForumSpam），不经过 preflight 网站的服务器。

---

## 1. 安装

**Linux / macOS：**

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/changyetech/preflight/releases/latest/download/preflight-installer.sh | sh
```

**Windows（PowerShell）：**

```powershell
powershell -c "irm https://github.com/changyetech/preflight/releases/latest/download/preflight-installer.ps1 | iex"
```

**从源码构建**（需要 Rust 工具链）：

```bash
make cli-release        # 产物在 target/release/preflight
```

验证安装：

```bash
preflight --version
```

### 1.1 卸载

```bash
preflight uninstall           # 删除二进制与 install receipt，配置保留
preflight uninstall --purge   # 连配置目录一起删除
```

手动清理（二进制已损坏、或想逐项确认时）：

| 对象 | Linux / macOS | Windows |
|---|---|---|
| 二进制 | 安装器按 `$XDG_BIN_HOME` → `$XDG_DATA_HOME/../bin` → `~/.local/bin` 顺序落盘，通常在 `~/.local/bin/preflight` | PowerShell 安装器的对应目录 |
| install receipt | `${XDG_CONFIG_HOME:-~/.config}/preflight/preflight-receipt.json` | `%LOCALAPPDATA%\preflight\preflight-receipt.json` |
| 配置文件 | `${XDG_CONFIG_HOME:-~/.config}/preflight/config.toml` | `%APPDATA%\preflight\config.toml` |

安装器若曾向 shell rc（`~/.profile` 等）追加过 PATH 行，`uninstall` 不会去改它——留着无害，介意就手动删除该行。

---

## 2. 快速开始

裸敲即体检——没有必须记住的子命令：

```bash
preflight
```

全部探测并发执行，结束后一次性输出报告：先是综合结论（三档：低 / 中 / 高风险，或「数据不足」）与覆盖度（`n/10`），随后是逐项检测结果。进度提示只在交互终端的 stderr 上出现，stdout 永远是一份干净的报告——重定向到文件不会混进进度行。

---

## 3. 命令总览

```
preflight [OPTIONS]                  # 体检（默认命令）
preflight dns [--check]              # 公共 DNS 服务器清单（可实测连通性）
preflight config <ACTION>            # 查看与修改配置
preflight uninstall [--purge]        # 卸载（--purge 连配置一起删）
```

### 3.1 全局与顶层参数

| 参数 | 位置 | 说明 |
|---|---|---|
| `--lang <LANG>` | 任意位置（全局） | 界面语言：`en` / `zh-hans` |
| `--json` | 子命令**之前** | 机器可读输出（体检与 `dns` 均支持） |
| `-v, --verbose` | 子命令**之前** | 人读报告里附上每个检测项的解释 |
| `-h, --help` | — | 帮助 |
| `-V` / `--version` | — | 短版本号 / 完整版本信息（含版权行） |

语言未显式指定时依次取：`--lang` > 配置文件 `language` > 系统 locale > 英文。

### 3.2 `preflight`（体检）

跑全部 10 项检测并输出报告。

```bash
preflight                  # 人读报告
preflight -v               # 附每项检测的解释
preflight --lang zh-hans   # 中文界面
preflight --json           # 机器可读 JSON（见 §6）
preflight > report.txt     # 重定向：自动关色、固定宽度
```

**退出码**（脚本化的关键约定）：

| 退出码 | 含义 |
|---|---|
| `0` | 体检完成——**不论风险档位**。风险档位从报告 / JSON 里读，不从退出码读：用退出码表达风险会让脚本把「高风险」和「工具挂了」混为一谈 |
| `1` | 工具自身失败（配置不合法、语言无效等） |
| `2` | 体检跑了，但没有产出任何贡献信号（结论为「数据不足」）。报告照常输出，只是不构成可用结论 |

**输出行为**：

- 颜色：stdout 非 TTY（管道、重定向）自动关闭；也可用 `NO_COLOR` 环境变量或 `config set no-color true` 关闭。
- 宽度：跟随终端窗口，渲染前量一次；重定向时落回固定宽度，产物不随窗口大小变化。

### 3.3 `preflight dns`

列出内置的公共 DNS 服务器清单（IP / 品牌 / 地区 / 过滤级别，数据与 Web 端同源）：

```bash
preflight dns              # 静态清单
preflight dns --check      # 对每台服务器发真实 DNS 查询，实测连通性与延迟
preflight --json dns --check   # 机器可读
```

`--check` 的三种状态：

| 状态 | 判据 |
|---|---|
| 通（`ok`） | 收到应答，TXID 匹配、RCODE = NOERROR、有 A 记录且非私网地址 |
| 应答可疑（`suspicious`） | 有应答但不满足上述全部条件——可能被劫持或污染 |
| 不通（`unreachable`） | 超时无有效应答 |

### 3.4 `preflight config`

| 命令 | 说明 |
|---|---|
| `config path` | 打印实际使用的配置文件路径 |
| `config list` | 列出每个键的**生效值**（合并全部来源后，见 §4；secret 只报「已设置 / 未设置」，绝不回显） |
| `config get <key>` | 查看单个键的生效值（口径同 `list`） |
| `config set <key> [value]` | 写入配置文件 |
| `config unset <key>` | 从配置文件删掉该键，恢复内置默认（幂等） |

可配置的键是**白名单**（[verdict.md §8](./verdict.md)），判级阈值与检测项开关永远不可配：

| 键 | 取值 | 默认 | 说明 |
|---|---|---|---|
| `language` | `en` / `zh-hans` | 跟随系统 locale | 界面语言 |
| `proxycheck-key` | 交互式输入 | 未设置 | proxycheck.io API key，见 §5 |
| `timeout` | 1–120（秒） | 10 | 网络探测超时 |
| `no-color` | `true` / `false` | `false` | 关闭彩色输出 |

示例：

```bash
preflight config set language zh-hans
preflight config set timeout 20
preflight config set proxycheck-key    # 交互式输入，不回显——没有明文 flag，
                                       # 明文会进 shell history 与 ps
preflight config unset timeout
```

若写入的键正被更高优先级的来源覆盖（如 `--lang`、`PROXYCHECK_API_KEY`、`NO_COLOR`），命令会在 stderr 提示——写进去了但当前不生效。

---

## 4. 配置来源与优先级

优先级恒为：**命令行 flag > 环境变量 > 配置文件 > 内置默认**。

**配置文件路径**（`config path` 打印实际值）：

| 平台 | 路径 |
|---|---|
| Linux / macOS | `$XDG_CONFIG_HOME/preflight/config.toml`，未设 XDG 时为 `~/.config/preflight/config.toml` |
| Windows | `%APPDATA%\preflight\config.toml` |
| 任意 | 环境变量 `PREFLIGHT_CONFIG` 可指定任意路径，优先级最高 |

文件为 TOML，键名用下划线（`proxycheck_key`、`no_color`）。**未知键报错退出**而不是静默忽略——拼错的键不会表现成「配了但没生效」。`config set` 落盘时会重新序列化整个文件（手写的注释会丢，想保注释就直接编辑文件）；Unix 上权限置 600。

**环境变量**：

| 变量 | 说明 |
|---|---|
| `PROXYCHECK_API_KEY` | proxycheck key，优先于配置文件；空值视为未设置 |
| `NO_COLOR` | 存在且非空即关色（[no-color.org](https://no-color.org) 约定，不看具体值） |
| `PREFLIGHT_CONFIG` | 配置文件路径覆盖 |

---

## 5. proxycheck API key

O4（IP 类型与风险）与归属查询走 proxycheck.io。不带 key 时用其匿名额度（每日 100 次查询）；设置免费 key 后涨到每日 1000 次：

```bash
preflight config set proxycheck-key    # 交互输入；或 export PROXYCHECK_API_KEY=...
preflight config get proxycheck-key    # 只报「已设置 / 未设置」
```

key 绝不出现在任何输出里（报告、`--json`、日志、错误信息）。CLI 用的是你自己的配额，与 preflight 网页版的共享配额互不影响（[ADR-0012](./adr/0012-cli-direct-third-party-not-worker-api.md)）。

---

## 6. `--json` 输出

`preflight --json` 输出一个 JSON 对象（无 HTTP 信封），字段名沿用判级契约的信号名（camelCase，与 Web 对齐）。顶层结构：

```jsonc
{
  "verdict": { "stage": "...", "level": "..." },   // insufficient 时 level 为 null
  "coverage": { "done": 9, "failed": 1, "total": 10 },
  "signals": {
    // 三态：true / false / null。null = 未知，不是 false（契约 §2.3）
    "tzMismatchCliEnv": false, "tzMismatchSystem": null, "ipv6Leak": false,
    "riskScore": 0, "anonymous": false, "abuseListed": false,
    "dnsEgressLeak": false, "udpEgressMismatch": false, "tunOff": null
  },
  "checks": {
    // O1–O6、C1–C4 各一个键；完成为 {"status":"done", ...检测项字段}，
    // 失败为 {"status":"failed","reason":"upstream|quotaExhausted|local"}
  }
}
```

`preflight --json dns` 是独立 schema：`{ "servers": [{ "ip", "name", "region", "variant", "check"? }] }`，`check` 仅在 `--check` 时出现（`reachable` / `latency_ms` / `status`）。

各检测项的字段语义与信号定义以 [verdict.md](./verdict.md) 为准。

---

## 7. 使用场景

**换了代理节点，先体检再干活**——确认 DNS / UDP / IPv6 没有绕过代理、出口 IP 风险分不高：

```bash
preflight
```

**脚本 / 自动化**——用 `--json` + 退出码。注意风险档位在 JSON 里，退出码只表达「工具是否正常跑完」：

```bash
if ! out=$(preflight --json); then
  echo "preflight 未能得出结论" >&2; exit 1
fi
level=$(echo "$out" | jq -r '.verdict.level')
[ "$level" = "high" ] && echo "高风险出口，停止后续操作" >&2 && exit 1
```

**排查「代理开了但还是被风控」**——看报告里的 O5（DNS 出口泄露）、O6（UDP 出口一致性）、O3（IPv6 泄露）与 C3（TUN 未开启）；`-v` 附上每项的解释。

**挑一台能用的公共 DNS**：

```bash
preflight dns --check      # 实测各家公共 DNS 的连通性与延迟，顺带识别可疑应答
```

**频繁使用**——设置免费 proxycheck key，配额从每日 100 涨到 1000（§5）。

**慢网络 / 严格代理环境下探测总超时**：

```bash
preflight config set timeout 30
```
