# 判级规则提升为 normative 契约，CLI 是全集实现、Web 是投影

判级规则此前散落在两处：`docs/specs/2026-08-10-ipcheck-web.md` §3（design spec，按 CLAUDE.md 的定义是 descriptive、会漂）与 `src/domain/verdict.ts`（真实行为）。用 Rust 在本仓库重写 CLI 后，同一套领域知识要被两个语言各实现一遍，散文 + 各写各的测试必然漂移。我们决定新建 [docs/verdict.md](../verdict.md) 作为**唯一 normative 的判级契约**，CLI 为全集实现（9 项）、Web 为其在浏览器 + 边缘能力边界内的投影，`docs/specs/` 里的判级章节掏空改为引用。

## Considered Options

- **只共享 O1–O4 那四项的判级，C1–C5 另立 CLI 私有文档**。否决：两侧的「综合结论」会成为两个不可比的东西，而"综合结论"恰恰是这个产品唯一对用户承诺的输出。
- **不建新文档，CLI 照抄 spec §3**。否决：CLAUDE.md 明确规定 design spec 是 point-in-time、代码才是当前行为的真相；把它当契约读，等于给未来的漂移背书。
- **代码层面共享**。TypeScript 与 Rust 之间没有可共享的判级实现，这条路不存在。

## Consequences

- **ipcheck CLI 相对 `ai-ipcheck` 有且仅有四处刻意的行为变更**，除此之外严格保持原行为，以便用 `ai-ipcheck` 作为重写期的正确性 oracle。前两处是判级规则的变更，后两处是「未知不得冒充」这条原则在探测层的落地（实现期实测发现，见 [--output 计划的平价比对结果](../plans/2026-08-12-cli-rust-rewrite--output.md)）：
  1. **IPv6 泄露从「仅进检测建议」升格为贡献「中」**，跟随 [ADR-0006](./0006-ipv6-elevated-to-medium-risk.md)。该 ADR 原本只约束 Web，现在它是契约的一部分，因而对两端同时生效。
  2. **Claude 端点命中中转黑名单（C5）从贡献「高」降为分项提醒**，不再进入综合结论。仍然展示、仍然告警，只是不改变档位。
  3. **O3 改用 ipify 双端点远端回显**（契约 2.2 / [ADR-0003](./0003-ipv6-leak-via-third-party-dual-stack-echo.md)），不再读本机网卡地址。`ai-ipcheck` 用 UDP socket 读本地 socket 地址，实测会拿到 ULA（`fc00::1` 这类私有、不可全球路由的地址）并报成「IPv6 泄露，暴露真实地址」——**那是误报**，ULA 出不了本地网络。
  4. **TUN 状态未知时不贡献「中」**。`ai-ipcheck` 的 `tun_active is not True` 把「检测不到」也算成中风险，于是每个 Windows 用户都被永久判中风险且无从解决。契约 2.3 明确要求未知不贡献任何一侧。
- 契约必须能被机器验证，否则它只是措辞更强硬的散文：`docs/verdict-cases.json` 作为共享 golden 向量，两端各写参数化测试消费同一份文件，并同时进入两条 CI workflow 的 paths 过滤。
- 判级规则的变更顺序被固定为：契约 → golden 向量 → 两端实现。先改实现再补文档是被禁止的。
- 黑名单降级的直接收益是**「高」档的唯一来源恒为 `riskScoreHigh`（O4）**，两端一致。这维持了一条不变量：「高」只可能出现在 `full` 形态。若保留一个来自 C5、与 O4 无关的高档信号，该不变量立刻破裂——黑名单命中但 proxycheck 失败或配额耗尽时，一个确定的高风险会被压成「初步 · 中」，而 [ADR-0005](./0005-two-stage-verdict.md) 的 `preliminary` 取值域恰恰排除「高」。
- 支持性事实：`_BLACKLIST_147` 是从 Claude Code 反蒸馏水印解出的**冻结快照**，而该水印已在 CC 2.1.198 从二进制移除，这份名单无法再刷新。让一份不可更新的陈旧数据驱动「高风险」定论，误报只会随时间单向累积。
- 代价：判级规则的每次改动从"改一处代码"变成"改契约 + 改向量 + 改两端"。这是刻意的——它正是我们想要的摩擦。
