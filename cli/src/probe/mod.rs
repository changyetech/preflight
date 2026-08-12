//! 探测层：8 个检测项的采集与编排。
//!
//! **并发跑，一次性出结果**：单个探测失败不影响其余探测（契约 4 的覆盖度要求
//! 每一项都有确定的终态）。不引 tokio——这是个跑几秒就退出的 CLI，
//! 异步运行时带来的编译时间与二进制体积换不回任何东西。

pub mod dns;
pub mod echo;
pub mod http;
pub mod ipify;
pub mod proxycheck;
pub mod stopforumspam;
pub mod timezone;

pub mod proxy;

use std::thread;
use std::time::Duration;

use crate::domain::checks::{Coverage, Failure, Outcome};
use crate::domain::verdict::{self, Signals, Verdict};

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
            tun_off: self.c3.value().and_then(proxy::Status::tun_off),
        }
    }

    pub fn verdict(&self) -> Verdict {
        verdict::compute(&self.signals())
    }
}

/// 跑完 8 项。
///
/// 编排上分两拨：先并发跑彼此独立的探测，再跑依赖出口 IP 的那些。
/// O2/C4 依赖 proxycheck 给的出口时区，因此排在最后——这条依赖链正是
/// docs/verdict.md 5.4 记的那处级联的来源。
pub fn run(timeout: Duration, proxycheck_key: Option<&str>) -> Report {
    let agent = http::agent(timeout);

    // 第一拨：互不依赖，全部并发。本机探测（C2/C3）也放进来——
    // 它们要 fork 子进程，等待时间不该串在网络探测后面。
    let (exposure, real_ip, dns_servers, proxy_status) = thread::scope(|scope| {
        let exposure = scope.spawn(|| ipify::probe(&agent));
        let real_ip = scope.spawn(|| echo::probe(&agent));
        let dns_servers = scope.spawn(dns::probe);
        let proxy_status = scope.spawn(proxy::probe);

        (
            exposure.join().ok(),
            real_ip.join().ok().flatten(),
            dns_servers.join().ok(),
            proxy_status.join().ok(),
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

    assemble(
        exposure,
        exit_ip,
        lookup,
        abuse,
        real_ip,
        dns_servers,
        proxy_status,
    )
}

fn assemble(
    exposure: Option<ipify::Exposure>,
    exit_ip: Option<String>,
    lookup: proxycheck::Outcome,
    abuse: Option<stopforumspam::Abuse>,
    real_ip: Option<String>,
    dns_servers: Option<Vec<dns::Server>>,
    proxy_status: Option<proxy::Status>,
) -> Report {
    let geo = match &lookup {
        proxycheck::Outcome::Ok(l) => Some(l.geo.clone()),
        _ => None,
    };
    let exit_timezone = geo.as_ref().and_then(|g| g.timezone.clone());

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
    fn coverage_always_sums_to_eight() {
        assert!(blank().coverage().is_complete());
        assert_eq!(blank().coverage().failed, 8);
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
