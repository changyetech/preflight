//! 探测层：10 个检测项的采集与编排。
//!
//! **并发跑，一次性出结果**：单个探测失败不影响其余探测（契约 4 的覆盖度要求
//! 每一项都有确定的终态）。不引 tokio——这是个跑几秒就退出的 CLI，
//! 异步运行时带来的编译时间与二进制体积换不回任何东西。

pub mod dns;
pub mod dns_egress;
pub mod echo;
pub mod http;
pub mod ipify;
pub mod proxycheck;
pub mod stopforumspam;
pub mod stun;
pub mod timezone;

pub mod proxy;

use std::net::IpAddr;
use std::thread;
use std::time::Duration;

use crate::domain::checks::{Coverage, Failure, Outcome};
use crate::domain::dns_egress::DnsEgress;
use crate::domain::udp_egress::UdpEgress;
use crate::domain::verdict::{self, Signals, Verdict};

/// 无依赖的随机源，给 STUN 的 transaction ID 与 DNS 出口探测的唯一子域用。
///
/// `RandomState` 的哈希键在进程内由操作系统随机数播种，每次 `new()` 还会掺入一个
/// 递增计数；再混进当前时刻，足够满足这两处「不可预测且不重复」的要求。
/// 两处都**不是密码学用途**，为它们引一个 `rand` 依赖换不回任何东西。
fn random_u64() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u128(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or_default(),
    );
    hasher.finish()
}

/// O1 出口 IP 与归属。
#[derive(Debug, Clone, PartialEq)]
pub struct ExitInfo {
    pub ip: String,
    /// 归属来自 **proxycheck**，不是 Cloudflare——两个地理库对同一 IP 的判定可能不同，
    /// 见 docs/verdict.md 5.4。呈现层必须标明来源。
    pub geo: Option<proxycheck::Geo>,
}

/// 时区一致性的一条比对。
#[derive(Debug, Clone, PartialEq)]
pub struct TimezoneCheck {
    pub local: Option<String>,
    pub exit: Option<String>,
    pub matches: timezone::Match,
}

/// 一次体检的全部观测值。
#[derive(Debug)]
pub struct Report {
    pub o1: Outcome<ExitInfo>,
    pub o2: Outcome<TimezoneCheck>,
    pub o3: Outcome<ipify::Ipv6>,
    pub o4: Outcome<Risk>,
    pub o5: Outcome<DnsEgress>,
    pub o6: Outcome<UdpEgress>,
    pub c1: Outcome<String>,
    pub c2: Outcome<Vec<dns::Server>>,
    pub c3: Outcome<proxy::Status>,
    pub c4: Outcome<TimezoneCheck>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Risk {
    pub risk: proxycheck::Risk,
    /// `None` = 未知（StopForumSpam 不可用）。**不得塌缩成「无收录」**。
    pub abuse: Option<stopforumspam::Abuse>,
}

impl Report {
    pub fn coverage(&self) -> Coverage {
        Coverage::tally([
            self.o1.is_done(),
            self.o2.is_done(),
            self.o3.is_done(),
            self.o4.is_done(),
            self.o5.is_done(),
            self.o6.is_done(),
            self.c1.is_done(),
            self.c2.is_done(),
            self.c3.is_done(),
            self.c4.is_done(),
        ])
    }

    /// 把观测值折成判级信号。**这是探测层与领域层唯一的接缝**——
    /// 判级只看这里产出的信号，看不到任何原始响应。
    pub fn signals(&self) -> Signals {
        Signals {
            tz_mismatch_cli_env: self.c4.value().and_then(|c| c.matches).map(|m| !m),
            tz_mismatch_system: self.o2.value().and_then(|c| c.matches).map(|m| !m),
            ipv6_leak: self.o3.value().map(|v| matches!(v, ipify::Ipv6::Leaked(_))),
            risk_score: self.o4.value().map(|r| r.risk.risk_score),
            anonymous: self.o4.value().map(|r| r.risk.anonymous),
            // 未知不贡献：SFS 挂了不等于没有收录（契约 2.3）。
            abuse_listed: self
                .o4
                .value()
                .and_then(|r| r.abuse.as_ref())
                .map(|a| a.listed),
            // 取的是**可比对性**，不是检测项状态：O5／O6 的「无从比对」正是「已完成」
            // （探测成功了，只是回答里不含可判定的信息，契约 2.5／2.6），而按 2.3
            // 它不贡献任何信号。用 `is_done()` 判断有无信号，会让「O1 失败 ⇒ 两项无从比对」
            // 这条常见路径给出绿色的「初步 · 低」——把没测成说成没问题。
            dns_egress_leak: self.o5.value().and_then(|d| d.comparison.leak()),
            udp_egress_mismatch: self.o6.value().and_then(UdpEgress::mismatch),
            tun_off: self.c3.value().and_then(proxy::Status::tun_off),
        }
    }

    pub fn verdict(&self) -> Verdict {
        verdict::compute(&self.signals())
    }
}

/// 跑完 10 项。
///
/// 编排上分两拨：先并发跑彼此独立的探测，再跑依赖出口 IP 的那些。
/// O2/C4 依赖 proxycheck 给的出口时区，因此排在最后——这条依赖链正是
/// docs/verdict.md 5.4 记的那处级联的来源。
pub fn run(timeout: Duration, proxycheck_key: Option<&str>) -> Report {
    let agent = http::agent(timeout);

    // 第一拨：互不依赖，全部并发。本机探测（C2/C3）也放进来——
    // 它们要 fork 子进程，等待时间不该串在网络探测后面。
    // O5／O6 的两个探测也在这一拨：它们采集的是原始观测（ECS 归属、反射地址），
    // 与出口 IP 无关——出口 IP 只在**判定**时才用到。
    let (exposure, real_ip, dns_servers, proxy_status, dns_egress, reflexive_ips) =
        thread::scope(|scope| {
            let exposure = scope.spawn(|| ipify::probe(&agent));
            let real_ip = scope.spawn(|| echo::probe(&agent));
            let dns_servers = scope.spawn(dns::probe);
            let proxy_status = scope.spawn(proxy::probe);
            let dns_egress = scope.spawn(|| dns_egress::probe(&agent));
            let reflexive_ips = scope.spawn(|| stun::probe(timeout));

            (
                exposure.join().ok(),
                real_ip.join().ok().flatten(),
                dns_servers.join().ok(),
                proxy_status.join().ok(),
                dns_egress.join().ok().flatten(),
                reflexive_ips.join().unwrap_or_default(),
            )
        });

    let exit_ip = exposure.as_ref().and_then(|e| e.ipv4.clone());

    // 第二拨：依赖出口 IP。proxycheck 与 StopForumSpam 可以并发。
    let (lookup, abuse) = match exit_ip.as_deref() {
        Some(ip) => thread::scope(|scope| {
            let lookup = scope.spawn(|| proxycheck::lookup(&agent, ip, proxycheck_key));
            let abuse = scope.spawn(|| stopforumspam::probe(&agent, ip));
            (
                lookup.join().unwrap_or(proxycheck::Outcome::Unavailable),
                abuse.join().ok().flatten(),
            )
        }),
        None => (proxycheck::Outcome::Unavailable, None),
    };

    assemble(Observations {
        exposure,
        exit_ip,
        lookup,
        abuse,
        real_ip,
        dns_servers,
        proxy_status,
        dns_egress,
        reflexive_ips,
    })
}

/// 一拨探测下来的全部原始观测。用一个结构体而不是九个位置参数：
/// 同类型的 `Option<String>` 挨在一起时，位置参数写反了编译器也不会说话。
struct Observations {
    exposure: Option<ipify::Exposure>,
    exit_ip: Option<String>,
    lookup: proxycheck::Outcome,
    abuse: Option<stopforumspam::Abuse>,
    real_ip: Option<String>,
    dns_servers: Option<Vec<dns::Server>>,
    proxy_status: Option<proxy::Status>,
    dns_egress: Option<crate::domain::dns_egress::Observation>,
    reflexive_ips: Vec<IpAddr>,
}

fn assemble(observations: Observations) -> Report {
    let Observations {
        exposure,
        exit_ip,
        lookup,
        abuse,
        real_ip,
        dns_servers,
        proxy_status,
        dns_egress,
        reflexive_ips,
    } = observations;

    let geo = match &lookup {
        proxycheck::Outcome::Ok(l) => Some(l.geo.clone()),
        _ => None,
    };
    let exit_timezone = geo.as_ref().and_then(|g| g.timezone.clone());

    // O5／O6 的判定拿 O1 的出口归属作另一侧。两者都可能是未知（O1 失败、proxycheck
    // 没给出国家），那时判定表落「无从比对」——**不是**「未命中」。
    let exit_country = geo.as_ref().and_then(|g| g.country_code.clone());
    // 认不出协议族的出口地址等同于未知：反射地址无从筛选，落判定表第 3 行。
    let exit_address: Option<IpAddr> = exit_ip.as_ref().and_then(|ip| ip.parse().ok());

    let o1 = match exit_ip {
        Some(ip) => Outcome::Done(ExitInfo { ip, geo }),
        None => Outcome::Failed(Failure::Upstream),
    };

    let o4 = match lookup {
        proxycheck::Outcome::Ok(l) => Outcome::Done(Risk {
            risk: l.risk,
            abuse,
        }),
        proxycheck::Outcome::QuotaExhausted => Outcome::Failed(Failure::QuotaExhausted),
        proxycheck::Outcome::Unavailable => Outcome::Failed(Failure::Upstream),
    };

    // O2 与 C4 是两条不同的比对，不是一条的两种说法（契约 5.1）。
    let o2 = build_timezone(timezone::system_timezone(), exit_timezone.clone());
    let c4 = build_timezone(timezone::cli_timezone(), exit_timezone);

    Report {
        o1,
        o2,
        o3: match exposure {
            Some(e) if e.ipv6 != ipify::Ipv6::Indeterminate => Outcome::Done(e.ipv6),
            // v4 对照端点不通 ⇒ 分不清「没有 IPv6」与「ipify 挂了」，只能判失败。
            _ => Outcome::Failed(Failure::Upstream),
        },
        o4,
        o5: crate::domain::dns_egress::judge(dns_egress.as_ref(), exit_country.as_deref()),
        o6: crate::domain::udp_egress::judge(&reflexive_ips, exit_address),
        c1: match real_ip {
            Some(ip) => Outcome::Done(ip),
            None => Outcome::Failed(Failure::Upstream),
        },
        c2: match dns_servers {
            Some(servers) if !servers.is_empty() => Outcome::Done(servers),
            _ => Outcome::Failed(Failure::Local),
        },
        c3: match proxy_status {
            Some(status) => Outcome::Done(status),
            None => Outcome::Failed(Failure::Local),
        },
        c4,
    }
}

/// 比不出来时仍算「已完成」：`matches: None` 表达的是「无从比对」，
/// 那是一个有效的观测结果，不是探测失败——而它不贡献信号（契约 2.3）。
fn build_timezone(local: Option<String>, exit: Option<String>) -> Outcome<TimezoneCheck> {
    let matches = timezone::compare(local.as_deref(), exit.as_deref());
    Outcome::Done(TimezoneCheck {
        local,
        exit,
        matches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::verdict::{Level, PreliminaryLevel};

    fn blank() -> Report {
        Report {
            o1: Outcome::Failed(Failure::Upstream),
            o2: Outcome::Failed(Failure::Upstream),
            o3: Outcome::Failed(Failure::Upstream),
            o4: Outcome::Failed(Failure::Upstream),
            o5: Outcome::Failed(Failure::Upstream),
            o6: Outcome::Failed(Failure::Upstream),
            c1: Outcome::Failed(Failure::Upstream),
            c2: Outcome::Failed(Failure::Local),
            c3: Outcome::Failed(Failure::Local),
            c4: Outcome::Failed(Failure::Upstream),
        }
    }

    fn tz(matches: timezone::Match) -> Outcome<TimezoneCheck> {
        Outcome::Done(TimezoneCheck {
            local: Some("Asia/Shanghai".into()),
            exit: Some("America/Los_Angeles".into()),
            matches,
        })
    }

    #[test]
    fn coverage_always_sums_to_ten() {
        assert!(blank().coverage().is_complete());
        assert_eq!(blank().coverage().failed, 10);
    }

    #[test]
    fn everything_failed_yields_no_verdict_not_low_risk() {
        // 全军覆没时报「低风险」＝拿检测失败冒充安全。
        assert_eq!(blank().verdict(), Verdict::Insufficient);
    }

    #[test]
    fn only_the_cli_timezone_signal_reaches_the_verdict() {
        // 契约 5.1：系统时区那条在 CLI 侧只展示，不进结论。
        let mut report = blank();
        report.o2 = tz(Some(false));
        assert_eq!(report.signals().tz_mismatch_system, Some(true));
        assert_eq!(report.verdict(), Verdict::Insufficient);

        report.c4 = tz(Some(false));
        assert_eq!(
            report.verdict(),
            Verdict::Preliminary(PreliminaryLevel::Medium)
        );
    }

    #[test]
    fn indeterminate_timezone_is_not_a_mismatch() {
        let mut report = blank();
        report.c4 = tz(None);
        assert_eq!(report.signals().tz_mismatch_cli_env, None);
        // C4 已完成但无从比对 ⇒ 不贡献信号 ⇒ 仍是数据不足。
        assert_eq!(report.verdict(), Verdict::Insufficient);
        assert_eq!(report.coverage().done, 1);
    }

    #[test]
    fn quota_exhausted_counts_as_failed_and_keeps_the_verdict_preliminary() {
        let mut report = blank();
        report.o4 = Outcome::Failed(Failure::QuotaExhausted);
        report.c4 = tz(Some(true));
        assert_eq!(report.signals().risk_score, None);
        assert_eq!(
            report.verdict(),
            Verdict::Preliminary(PreliminaryLevel::Low)
        );
        assert!(report.coverage().is_complete());
    }

    #[test]
    fn ipv6_leak_contributes_medium() {
        let mut report = blank();
        report.o3 = Outcome::Done(ipify::Ipv6::Leaked("2001:db8::1".into()));
        assert_eq!(report.signals().ipv6_leak, Some(true));
        assert_eq!(
            report.verdict(),
            Verdict::Preliminary(PreliminaryLevel::Medium)
        );

        report.o3 = Outcome::Done(ipify::Ipv6::Disabled);
        assert_eq!(report.signals().ipv6_leak, Some(false));
        assert_eq!(
            report.verdict(),
            Verdict::Preliminary(PreliminaryLevel::Low)
        );
    }

    #[test]
    fn a_completed_but_incomparable_split_tunnel_check_produces_no_signal() {
        // 这是本产品的红线：**「无从比对」不是「产出了信号」**。用 `is_done()` 判断
        // 有无信号，下面这份报告（O1 失败 ⇒ 两项无从比对）就会给出绿色的「初步 · 低」。
        let mut report = blank();
        report.o5 = crate::domain::dns_egress::judge(
            Some(&crate::domain::dns_egress::Observation {
                ecs_geo: Some("Japan - IT7 Networks Inc".into()),
                resolver_geo: None,
            }),
            None, // O1 没给出国家
        );
        report.o6 = crate::domain::udp_egress::judge(
            &[
                "203.0.113.7".parse().unwrap(),
                "203.0.113.7".parse().unwrap(),
            ],
            None, // O1 没给出出口 IP
        );

        // 两项都「已完成」……
        assert_eq!(report.coverage().done, 2);
        // ……却一个信号都没产出。
        assert_eq!(report.signals().dns_egress_leak, None);
        assert_eq!(report.signals().udp_egress_mismatch, None);
        assert_eq!(report.verdict(), Verdict::Insufficient);
    }

    #[test]
    fn a_comparable_split_tunnel_leak_contributes_medium() {
        let mut report = blank();
        report.o5 = crate::domain::dns_egress::judge(
            Some(&crate::domain::dns_egress::Observation {
                ecs_geo: Some("United States - Some ISP".into()),
                resolver_geo: None,
            }),
            Some("JP"),
        );
        assert_eq!(report.signals().dns_egress_leak, Some(true));
        assert_eq!(
            report.verdict(),
            Verdict::Preliminary(PreliminaryLevel::Medium)
        );

        // 反向：可比对且未命中仍算产出了信号，结论因此不是「数据不足」。
        report.o6 = crate::domain::udp_egress::judge(
            &[
                "198.51.100.20".parse().unwrap(),
                "198.51.100.20".parse().unwrap(),
            ],
            Some("198.51.100.20".parse().unwrap()),
        );
        assert_eq!(report.signals().udp_egress_mismatch, Some(false));
    }

    #[test]
    fn abuse_unknown_does_not_masquerade_as_clean() {
        let mut report = blank();
        report.o4 = Outcome::Done(Risk {
            risk: proxycheck::Risk {
                network_type: None,
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: 10,
                anonymous: false,
            },
            abuse: None,
        });
        assert_eq!(report.signals().abuse_listed, None);
        assert_eq!(report.verdict(), Verdict::Full(Level::Low));
    }
}
