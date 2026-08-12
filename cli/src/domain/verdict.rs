//! 综合结论。契约见 docs/verdict.md 第 2 / 3 节，用例见 docs/verdict-cases.json。
//!
//! 判级规则**只在契约里修改**：改这里之前先改 docs/verdict.md 和 verdict-cases.json，
//! 否则下面那组 golden 测试会红。

/// 综合结论判「高」的阈值（契约 3.1）——**二维**，由 `anonymous` 选择。
///
/// 两个数直接取自 proxycheck v3 官方的 deny 边界（见 docs/proxycheck.md）。
///
/// `anonymous` **不是「用户在用 VPN」**，而是 proxycheck 判定「这个 IP 当前正被用作
/// 匿名化地址」（TOR 出口、开放代理、住宅代理网络）。实测：普通商业 VPN 出口是 false，
/// TOR 出口是 true。因此这一维把「你在用 VPN」和「你的出口正被别人拿来匿名作恶」
/// 分开了——前者是本产品用户的常态。
const HIGH_RISK_SCORE_NOT_ANONYMOUS: u32 = 76;
const HIGH_RISK_SCORE_ANONYMOUS: u32 = 51;

/// 刻意**不与** `risk_level` 的阈值复用常量：那是**分项**分级（契约 6），
/// 语义上是另一件事，且现在连数值都不同了。
const fn high_risk_score(anonymous: bool) -> u32 {
    if anonymous {
        HIGH_RISK_SCORE_ANONYMOUS
    } else {
        HIGH_RISK_SCORE_NOT_ANONYMOUS
    }
}

/// 分项分级的两个阈值（契约 6），直接对齐 proxycheck v3 自己的分档
/// （0–25 / 26–50 / 51–75 / 76–100）：四档收成三色时中间两档并作黄，
/// **绿 = 它建议放行的区间，红 = 它对任何 IP 都建议拒绝的区间**。
///
/// 与综合结论是两把尺子：结论二维（51 或 76），分项只看分数。
/// `anonymous: true` 时两者不同界——结论 51 起判高，分项 76 才转红。
const RISK_LEVEL_MEDIUM: u32 = 26;
const RISK_LEVEL_HIGH: u32 = 76;

/// 判级的输入。
///
/// 每个信号都是**三态**：`Some(true)` 命中 / `Some(false)` 未命中 / `None` 未知。
/// 未知一律不贡献信号（契约 2.3）——把未知塌缩成 `false` 就等于拿"没测成"冒充"没问题"。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Signals {
    /// `$TZ` 时区 ≠ 出口 IP 时区（C4）。**这条才进综合结论。**
    pub tz_mismatch_cli_env: Option<bool>,
    /// 系统时区 ≠ 出口 IP 时区（O2）。
    ///
    /// **在 CLI 侧只展示、不贡献综合结论**（契约 5.1）：命令行进程认 `$TZ`，
    /// 设对了 `$TZ` 的用户不该因为系统时区没改而被报中风险——而设了 `$TZ` 的
    /// 恰恰是最懂行的那批。Web 侧读不到 `$TZ`，才拿这条当降级代理。
    pub tz_mismatch_system: Option<bool>,
    /// 检出 IPv6 泄露（O3）。
    pub ipv6_leak: Option<bool>,
    /// proxycheck 风险分（O4）。
    ///
    /// 与 `anonymous` **成对到达**：判「高」的阈值由后者决定，只有其中一个就无法判定。
    /// 任一缺失时两者同为 `None`（契约 2.3）。
    pub risk_score: Option<u32>,
    /// proxycheck 判定该 IP 当前正被用作匿名化地址（O4）。**不是「用户在用 VPN」。**
    pub anonymous: Option<bool>,
    /// StopForumSpam 有滥用收录（O4）。
    pub abuse_listed: Option<bool>,
    /// ECS 客户端子网归属国 ≠ 出口 IP 归属国（O5，契约 2.5）。
    ///
    /// **`None` 同时表示「检测失败」与「已完成但无从比对」**——后者是 O5 的常态
    /// （resolver 不发 ECS）。两者在判级上等价：都不贡献信号。差别只在覆盖度归档，
    /// 而覆盖度不在本层。
    pub dns_egress_leak: Option<bool>,
    /// ≥2 个 STUN 报出同一 srflx IP 且该 IP ≠ 出口 IP（O6，契约 2.6）。同上。
    pub udp_egress_mismatch: Option<bool>,
    /// TUN／VPN 未开启（C3）。
    pub tun_off: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Low,
    Medium,
    High,
}

/// 初步结论的取值域——**不含 `High`**（契约 3.2）。
///
/// 用一个独立类型而不是复用 `Level`，是为了让「初步 · 高风险」在类型层面构造不出来。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreliminaryLevel {
    Low,
    Medium,
}

/// 结论的三种形态。
///
/// `Insufficient` **没有档位字段**：一个信号都没产出时，「低风险 · 未发现异常」
/// 必须在类型层面就渲染不出来，而不是靠约定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Insufficient,
    Preliminary(PreliminaryLevel),
    Full(Level),
}

impl Verdict {
    /// 形态名，与契约和 golden 用例里的 `stage` 一致。
    pub const fn stage(&self) -> &'static str {
        match self {
            Verdict::Insufficient => "insufficient",
            Verdict::Preliminary(_) => "preliminary",
            Verdict::Full(_) => "full",
        }
    }

    /// 档位名。`Insufficient` 没有档位，返回 `None`。
    pub const fn level(&self) -> Option<&'static str> {
        match self {
            Verdict::Insufficient => None,
            Verdict::Preliminary(PreliminaryLevel::Low) => Some("low"),
            Verdict::Preliminary(PreliminaryLevel::Medium) => Some("medium"),
            Verdict::Full(Level::Low) => Some("low"),
            Verdict::Full(Level::Medium) => Some("medium"),
            Verdict::Full(Level::High) => Some("high"),
        }
    }
}

/// 分项分级（契约 6）。**不等于**综合结论——风险分 69 分项是黄，综合结论仍可能是低。
pub const fn risk_level(score: u32) -> Level {
    if score < RISK_LEVEL_MEDIUM {
        Level::Low
    } else if score < RISK_LEVEL_HIGH {
        Level::Medium
    } else {
        Level::High
    }
}

pub fn compute(signals: &Signals) -> Verdict {
    // 「有没有任何贡献信号产出结果」——注意 `tz_mismatch_system` 不在其中：
    // 它在 CLI 侧不贡献综合结论，因此它单独已知不足以让我们给出结论。
    //
    // O5／O6 把「检测项已完成」与「产出了信号」的区别放大成了常态：它们的「无从比对」
    // 正是已完成，而按契约 2.3 不贡献任何信号。因此这里看的恒为**信号是否已知**，
    // 绝不能改成看检测项状态——那会让「O1 失败 ⇒ O5／O6 无从比对」这条常见路径
    // 给出绿色的「初步 · 低」，即把没测成说成没问题（契约 3.2 的红线）。
    // 反向也别过头：**可比对且未命中**（`Some(false)`）必须仍算产出了信号，
    // 否则一次正常测完、干干净净的体检会被误判成「数据不足」。
    let any_contributing_signal = signals.tz_mismatch_cli_env.is_some()
        || signals.ipv6_leak.is_some()
        || signals.risk_score.is_some()
        || signals.abuse_listed.is_some()
        || signals.dns_egress_leak.is_some()
        || signals.udp_egress_mismatch.is_some()
        || signals.tun_off.is_some();

    if !any_contributing_signal {
        return Verdict::Insufficient;
    }

    let medium = signals.tz_mismatch_cli_env == Some(true)
        || signals.ipv6_leak == Some(true)
        || signals.abuse_listed == Some(true)
        // 两个新信号均为**中**档（契约 3.2）。给成「高」会当场打破「高只出现在 full 形态」
        // 这条不变量——它们不来自 O4，一个确定的分流泄露会在 proxycheck 失败时
        // 被压成「初步 · 中」。
        || signals.dns_egress_leak == Some(true)
        || signals.udp_egress_mismatch == Some(true)
        || signals.tun_off == Some(true);

    let Some(score) = signals.risk_score else {
        // 风险分未知 ⇒ 初步结论。取值域不含「高」——而这条成立的前提是
        // **唯一的高档信号来自 O4**（契约 3.2 的不变量）。新增任何不依赖 O4 的
        // 高档信号，都必须先去改契约那一节，而不是在这里加分支绕过去。
        return Verdict::Preliminary(if medium {
            PreliminaryLevel::Medium
        } else {
            PreliminaryLevel::Low
        });
    };

    // `anonymous` 缺失时退回非匿名的阈值（76）。契约要求两者成对到达，
    // 走到这里说明上游给了分数却没给 anonymous——按更难触发的那侧处理，
    // 不凭一个没测到的维度去抬高结论。
    if score >= high_risk_score(signals.anonymous == Some(true)) {
        return Verdict::Full(Level::High);
    }

    Verdict::Full(if medium { Level::Medium } else { Level::Low })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_colour_bands_follow_proxycheck_v3() {
        // v3 的四档 0–25 / 26–50 / 51–75 / 76–100，收成三色。
        assert_eq!(risk_level(25), Level::Low);
        assert_eq!(risk_level(26), Level::Medium);
        assert_eq!(risk_level(75), Level::Medium);
        assert_eq!(risk_level(76), Level::High);
    }

    fn scored(score: u32, anonymous: bool) -> Verdict {
        compute(&Signals {
            risk_score: Some(score),
            anonymous: Some(anonymous),
            ..Default::default()
        })
    }

    #[test]
    fn the_high_threshold_depends_on_the_anonymous_dimension() {
        // 契约 3.1：anonymous 为假 ⇒ 76；为真 ⇒ 51。两个数取自 proxycheck v3 的 deny 边界。
        assert_eq!(scored(75, false), Verdict::Full(Level::Low));
        assert_eq!(scored(76, false), Verdict::Full(Level::High));
        assert_eq!(scored(50, true), Verdict::Full(Level::Low));
        assert_eq!(scored(51, true), Verdict::Full(Level::High));
    }

    #[test]
    fn a_normal_vpn_exit_is_not_punished_for_being_a_vpn() {
        // 普通商业 VPN 出口实测是 anonymous: false、分数 33。
        // 二维判定让这类用户的阈值反而从旧的 70 升到 76。
        assert_eq!(scored(33, false), Verdict::Full(Level::Low));
        assert_eq!(scored(70, false), Verdict::Full(Level::Low));
    }

    #[test]
    fn missing_anonymous_falls_back_to_the_harder_threshold() {
        // 契约要求成对到达；真走到这里，不该凭一个没测到的维度去抬高结论。
        let verdict = compute(&Signals {
            risk_score: Some(70),
            anonymous: None,
            ..Default::default()
        });
        assert_eq!(verdict, Verdict::Full(Level::Low));
    }

    #[test]
    fn item_colour_and_verdict_diverge_on_anonymous_addresses() {
        // 非匿名时两者同界（都是 76）……
        assert_eq!(risk_level(76), Level::High);
        assert_eq!(scored(76, false), Verdict::Full(Level::High));
        // ……匿名时不同界：结论 51 起判高，分项要到 76 才转红。
        // 这一档全靠呈现层的文字解释，见契约 6。
        assert_eq!(risk_level(60), Level::Medium);
        assert_eq!(scored(60, true), Verdict::Full(Level::High));
    }

    #[test]
    fn system_timezone_alone_does_not_produce_a_verdict_on_the_cli() {
        // 它不是 CLI 侧的贡献信号，所以"只知道它"等于什么都不知道。
        assert_eq!(
            compute(&Signals {
                tz_mismatch_system: Some(true),
                ..Default::default()
            }),
            Verdict::Insufficient
        );
    }

    #[test]
    fn unknown_never_masquerades_as_clean() {
        // 全未知 ⇒ 数据不足，绝不能落到「低风险」。
        assert_eq!(compute(&Signals::default()), Verdict::Insufficient);
    }

    #[test]
    fn a_split_tunnel_leak_contributes_medium_never_high() {
        // 契约 3.2 的不变量：唯一的高档信号是 riskScoreHigh，它来自 O4。
        // 两个新信号来自 O5／O6，风险分缺席时结论必须停在「初步 · 中」。
        for signals in [
            Signals {
                dns_egress_leak: Some(true),
                ..Default::default()
            },
            Signals {
                udp_egress_mismatch: Some(true),
                ..Default::default()
            },
        ] {
            assert_eq!(
                compute(&signals),
                Verdict::Preliminary(PreliminaryLevel::Medium)
            );
        }
    }

    #[test]
    fn a_comparable_miss_still_counts_as_a_signal() {
        // 「可比对且未命中」是产出了信号。把它也算成「没产出」，一次正常测完、
        // 干干净净的体检就会被误判成「数据不足」——那是反向过头的那一侧。
        assert_eq!(
            compute(&Signals {
                dns_egress_leak: Some(false),
                ..Default::default()
            }),
            Verdict::Preliminary(PreliminaryLevel::Low)
        );
        assert_eq!(
            compute(&Signals {
                udp_egress_mismatch: Some(false),
                ..Default::default()
            }),
            Verdict::Preliminary(PreliminaryLevel::Low)
        );
    }

    #[test]
    fn insufficient_carries_no_level() {
        assert_eq!(Verdict::Insufficient.level(), None);
        assert_eq!(Verdict::Insufficient.stage(), "insufficient");
    }
}

/// 契约的 golden 向量。两端（这里与 Web 的 vitest）读**同一份文件**——
/// 改判级规则时两侧会同时变红，漂移因此变成编译期问题而不是考古问题。
///
/// 用 `include_str!` 而非运行时读取：不必解析相对路径、不受 cwd 影响，
/// 且 cargo 会把它登记为构建依赖——改 JSON 会触发重编译，测试不会吃到旧内容。
#[cfg(test)]
mod golden {
    use super::*;
    use crate::domain::{dns_egress, udp_egress};
    use serde::Deserialize;
    use std::net::IpAddr;

    const CASES: &str = include_str!("../../../docs/verdict-cases.json");

    #[derive(Deserialize)]
    struct CaseFile {
        cases: Vec<Case>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        id: String,
        applies: Vec<String>,
        signals: CaseSignals,
        expect: Expect,
        /// 指向一条**未命中**的基准用例，含义是「本条与它的结论必须完全相同」
        /// （见 verdict-cases.json 的 conventions）。
        #[serde(default)]
        pairs_with: Option<String>,
    }

    /// `deny_unknown_fields`：用例里写错一个信号名必须让测试红，
    /// 而不是被当成"没给这个信号"静默通过。
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct CaseSignals {
        #[serde(default)]
        tz_mismatch_cli_env: Option<bool>,
        #[serde(default)]
        tz_mismatch_system: Option<bool>,
        #[serde(default)]
        ipv6_leak: Option<bool>,
        #[serde(default)]
        risk_score: Option<u32>,
        #[serde(default)]
        anonymous: Option<bool>,
        #[serde(default)]
        abuse_listed: Option<bool>,
        #[serde(default)]
        tun_off: Option<bool>,
        // O5／O6 给的是**原始观测值**而非派生布尔量（用例文件的 conventions.signals），
        // 因此这里是四个观测字段，两个信号由判定层从它们推出来。
        #[serde(default)]
        dns_ecs_country: Option<String>,
        #[serde(default)]
        exit_country: Option<String>,
        #[serde(default)]
        stun_reflexive_ips: Option<Vec<String>>,
        #[serde(default)]
        exit_ip: Option<String>,
    }

    #[derive(Deserialize)]
    struct Expect {
        stage: String,
        #[serde(default)]
        level: Option<String>,
    }

    /// 用例的扁平观测值 → 判级信号。
    ///
    /// O5／O6 的观测值**过真正的判定层**（`dns_egress::compare` / `udp_egress::judge`），
    /// 不在这里另写一套推导——否则向量测的就是 harness 自己，实现漂移了也照样绿。
    fn signals_of(case: &CaseSignals) -> Signals {
        Signals {
            tz_mismatch_cli_env: case.tz_mismatch_cli_env,
            tz_mismatch_system: case.tz_mismatch_system,
            ipv6_leak: case.ipv6_leak,
            risk_score: case.risk_score,
            anonymous: case.anonymous,
            abuse_listed: case.abuse_listed,
            dns_egress_leak: dns_egress_of(case),
            udp_egress_mismatch: udp_egress_of(case),
            tun_off: case.tun_off,
        }
    }

    /// `None` = **没有产出可用信号**：检测失败与「已完成但无从比对」在判级上等价
    /// （契约 2.3／2.5），差别只在覆盖度归档，而覆盖度不由本文件覆盖。
    ///
    /// 向量给的 `dnsEcsCountry` 已经是 ISO2，因此这里直接构造 `Known`——
    /// 国家名 → ISO2 的映射发生在探测层之前，由 `dns_egress` 的单测覆盖。
    fn dns_egress_of(case: &CaseSignals) -> Option<bool> {
        let ecs = match &case.dns_ecs_country {
            Some(iso2) => dns_egress::EcsCountry::Known(iso2.clone()),
            None => dns_egress::EcsCountry::NoEcs,
        };
        dns_egress::compare(&ecs, case.exit_country.as_deref()).leak()
    }

    /// 同上。`null` 与空数组同义 ⇒ `N_ok = 0` ⇒ 检测失败 ⇒ 没有产出（用例文件的 conventions）。
    fn udp_egress_of(case: &CaseSignals) -> Option<bool> {
        let reflexive: Vec<IpAddr> = case
            .stun_reflexive_ips
            .as_deref()
            .unwrap_or_default()
            .iter()
            // 解析失败**响亮报错**：向量里写错一个地址不该被静默丢成「没答上来」。
            .map(|ip| {
                ip.parse()
                    .unwrap_or_else(|_| panic!("向量里的 IP 无法解析：{ip}"))
            })
            .collect();
        let exit = case.exit_ip.as_deref().map(|ip| {
            ip.parse()
                .unwrap_or_else(|_| panic!("向量里的 IP 无法解析：{ip}"))
        });

        udp_egress::judge(&reflexive, exit)
            .value()
            .and_then(udp_egress::UdpEgress::mismatch)
    }

    /// 两条用例的 signals 中取值不同的字段名。
    ///
    /// 字段缺省与显式 `null` 在这里天然同义——两者都反序列化成 `None`
    /// （用例文件的 conventions.unknown 明写「字段整体缺省等同于 null」）。
    ///
    /// **完整解构（不带 `..`）是有意的守卫**：给 `CaseSignals` 加第 12 个字段时这里会
    /// 编译不过，而不是静默漏比一个字段——配对断言的全部说服力恰恰建立在
    /// 「只差一个字段」上，漏比一个就等于让一条再也证明不了任何事的用例继续显绿。
    fn differing_signals(a: &CaseSignals, b: &CaseSignals) -> Vec<&'static str> {
        let CaseSignals {
            tz_mismatch_cli_env,
            tz_mismatch_system,
            ipv6_leak,
            risk_score,
            anonymous,
            abuse_listed,
            tun_off,
            dns_ecs_country,
            exit_country,
            stun_reflexive_ips,
            exit_ip,
        } = a;

        [
            (
                "tzMismatchCliEnv",
                *tz_mismatch_cli_env != b.tz_mismatch_cli_env,
            ),
            (
                "tzMismatchSystem",
                *tz_mismatch_system != b.tz_mismatch_system,
            ),
            ("ipv6Leak", *ipv6_leak != b.ipv6_leak),
            ("riskScore", *risk_score != b.risk_score),
            ("anonymous", *anonymous != b.anonymous),
            ("abuseListed", *abuse_listed != b.abuse_listed),
            ("tunOff", *tun_off != b.tun_off),
            ("dnsEcsCountry", *dns_ecs_country != b.dns_ecs_country),
            ("exitCountry", *exit_country != b.exit_country),
            (
                "stunReflexiveIps",
                *stun_reflexive_ips != b.stun_reflexive_ips,
            ),
            ("exitIp", *exit_ip != b.exit_ip),
        ]
        .into_iter()
        .filter_map(|(name, differs)| differs.then_some(name))
        .collect()
    }

    /// 解析 `pairsWith` 指向的基准用例。
    ///
    /// **悬空引用必须响亮失败**：那是数据错误，静默跳过会让配对断言整条失效——
    /// 一条改错了 id 的用例从此再也证明不了任何事，而它看起来仍然是绿的。
    fn baseline_of<'a>(case: &Case, cases: &'a [Case]) -> &'a Case {
        let target = case
            .pairs_with
            .as_deref()
            .expect("调用方已筛出带 pairsWith 的用例");
        cases
            .iter()
            .find(|c| c.id == target)
            .unwrap_or_else(|| panic!("用例 {} 的 pairsWith 指向不存在的用例 {target}", case.id))
    }

    #[test]
    fn cli_cases_match_the_contract() {
        let file: CaseFile = serde_json::from_str(CASES).expect("verdict-cases.json 必须可解析");
        let mut ran = 0;

        for case in &file.cases {
            for side in &case.applies {
                assert!(
                    matches!(side.as_str(), "web" | "cli"),
                    "用例 {} 的 applies 里有未知取值 {side:?}——\
                     拼错的一侧会让用例静默失效，所以这里必须报错而不是跳过",
                    case.id
                );
            }

            if !case.applies.iter().any(|s| s == "cli") {
                continue;
            }
            ran += 1;

            let got = compute(&signals_of(&case.signals));
            assert_eq!(got.stage(), case.expect.stage, "用例 {}: 形态不符", case.id);
            assert_eq!(
                got.level(),
                case.expect.level.as_deref(),
                "用例 {}: 档位不符",
                case.id
            );
        }

        assert!(ran > 0, "没有跑到任何 CLI 用例，说明筛选逻辑坏了");
    }

    /// `pairsWith`：证明某个取值是**无从比对**而不是**未命中**。
    ///
    /// 单看一条用例分不出这两者——只有让它与一条已知未命中的用例算出**同一个结论**
    /// 才说得清。断言的是两条各自**算出来**的 verdict 相等，而不是各自等于某个硬编码值：
    /// 后者会退化成把同一个期望写两遍，那样一条写错 `expect` 的新用例照样能全绿。
    ///
    /// 这与 Web 侧 `tests/verdict-cases.test.ts` 里的同名断言是对称的一份。
    #[test]
    fn paired_cases_reach_the_same_verdict_as_their_baseline() {
        let file: CaseFile = serde_json::from_str(CASES).unwrap();
        let mut ran = 0;

        for case in &file.cases {
            if case.pairs_with.is_none() || !case.applies.iter().any(|s| s == "cli") {
                continue;
            }
            ran += 1;

            let baseline = baseline_of(case, &file.cases);
            // 基准必须同样适用于本端，否则拿它作对照不成立。
            assert!(
                baseline.applies.iter().any(|s| s == "cli"),
                "用例 {} 的基准 {} 不适用于 cli",
                case.id,
                baseline.id
            );

            // 除被比对的那一个字段外其余 signals 必须逐一相同，否则「结论相同」证明不了
            // 是那个字段不贡献信号——可能是别处的差异把结论又拉了回来。
            assert_eq!(
                differing_signals(&case.signals, &baseline.signals).len(),
                1,
                "用例 {} 与基准 {} 只应差一个被比对的字段，实际差 {:?}",
                case.id,
                baseline.id,
                differing_signals(&case.signals, &baseline.signals)
            );

            assert_eq!(
                compute(&signals_of(&case.signals)),
                compute(&signals_of(&baseline.signals)),
                "用例 {} 与基准 {} 的结论必须相同——「无从比对」不是「未命中」，\
                 但两者在综合结论上等价",
                case.id,
                baseline.id
            );
        }

        assert!(
            ran > 0,
            "没有跑到任何带 pairsWith 的 CLI 用例，说明筛选逻辑坏了"
        );
    }

    #[test]
    #[should_panic(expected = "no-such-case")]
    fn a_dangling_pairs_with_fails_loudly_instead_of_being_skipped() {
        let dangling = Case {
            id: "dangling".into(),
            applies: vec!["cli".into()],
            signals: CaseSignals {
                tz_mismatch_cli_env: None,
                tz_mismatch_system: None,
                ipv6_leak: None,
                risk_score: None,
                anonymous: None,
                abuse_listed: None,
                tun_off: None,
                dns_ecs_country: None,
                exit_country: None,
                stun_reflexive_ips: None,
                exit_ip: None,
            },
            expect: Expect {
                stage: "insufficient".into(),
                level: None,
            },
            pairs_with: Some("no-such-case".into()),
        };

        baseline_of(&dangling, &[]);
    }

    #[test]
    fn insufficient_cases_declare_no_level() {
        // 结构约束：`insufficient` 的用例不该带 level——这与类型层面的设计一致。
        let file: CaseFile = serde_json::from_str(CASES).unwrap();
        for case in &file.cases {
            if case.expect.stage == "insufficient" {
                assert!(
                    case.expect.level.is_none(),
                    "用例 {} 是 insufficient，不该声明档位",
                    case.id
                );
            } else {
                assert!(
                    case.expect.level.is_some(),
                    "用例 {} 不是 insufficient，必须声明档位",
                    case.id
                );
            }
        }
    }
}
