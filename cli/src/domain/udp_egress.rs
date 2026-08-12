//! O6 UDP 出口一致性判定（判级契约 docs/verdict.md 2.6 判定表）。
//!
//! 需要至少两个互相独立的 STUN 作对照，否则无法区分「UDP 真的绕过了代理」与
//! 「这个出口本来就是多地址集群」——ADR-0003 那条「用对照区分未知与未命中」的第二次应用。

use std::net::IpAddr;

use super::checks::{Failure, Outcome};

/// 「无从比对」的三种成因。呈现层据此选文案——「两个 STUN 各报各的」与
/// 「UDP 与 TCP 同一个出口」在屏幕上必须长得不一样。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotComparable {
    /// 同族地址不足 2 个（判定表第 2 行）。
    FamilyMismatch,
    /// 出口 IP 未知（第 3 行）。
    UnknownExitIp,
    /// 同族地址彼此不一致（第 4 行）：多出口集群或对称 NAT。
    StunDisagree,
}

/// O6 的检测项数据（判定表第 2–6 行；第 1 行是检测失败，不在本类型内）。
///
/// 同样是判别式枚举而非 `Option<bool>`：两个 STUN 各报各的是**无从比对**，
/// 与「UDP 与 TCP 同一个出口」的未命中必须在类型层面就分得开。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpEgress {
    Comparable {
        mismatch: bool,
        /// 两个及以上 STUN 一致报出的反射地址。
        reflexive_ip: IpAddr,
        exit_ip: IpAddr,
    },
    NotComparable(NotComparable),
}

impl UdpEgress {
    /// 折成判级信号。**「无从比对」产出 `None`，不是 `Some(false)`**（契约 2.3）。
    pub fn mismatch(&self) -> Option<bool> {
        match self {
            UdpEgress::Comparable { mismatch, .. } => Some(*mismatch),
            UdpEgress::NotComparable(_) => None,
        }
    }
}

/// 判定表（契约 2.6），**从上往下取第一条匹配的行**。各行按 `N_ok` → `N_fam` → 取值分层，
/// 任何一组输入只会落进一行：
///
/// | # | 条件 | 判定 |
/// |---|---|---|
/// | 1 | `N_ok < 2` | 检测失败 |
/// | 2 | `N_ok ≥ 2` 且 `N_fam < 2` | 无从比对 |
/// | 3 | `N_fam ≥ 2` 且出口 IP 未知 | 无从比对 |
/// | 4 | `N_fam ≥ 2`，同族地址彼此不一致 | 无从比对 |
/// | 5 | `N_fam ≥ 2`，一致且 = 出口 IP | 未命中 |
/// | 6 | `N_fam ≥ 2`，一致且 ≠ 出口 IP | 命中 |
///
/// **「检测失败」只由第 1 行产生**：协议族筛选永远不会造成检测失败，它只会把结果推进
/// 第 2 行的「无从比对」。两者的可恢复性相反（STUN 没答上来刷新可能就好，
/// 「答了但都是另一个协议族」刷一万次也一样），所以这条分工是刻意的。
///
/// `reflexive_ips` 里只放**答上来**的（即 N_ok），没答上来的不占位。
pub fn judge(reflexive_ips: &[IpAddr], exit_ip: Option<IpAddr>) -> Outcome<UdpEgress> {
    // 第 1 行。UDP socket 全数超时落在这里，而不是绿色的「未泄露」——在「UDP 走没走代理」
    // 这个语义下，一个 STUN 都没答上来只是把测量仪器关掉了，不是测出了没问题。
    if reflexive_ips.len() < 2 {
        return Outcome::Failed(Failure::Upstream);
    }

    // 第 2 行。反射地址是 IPv6 而出口是 IPv4（或反之）不得判命中——那正是 O3 在管的事，
    // 在这里再算一次等于同一个事实进了两次判级（契约 2.6 协议族硬约束）。
    // 出口 IP 未知时无从筛选，N_fam = N_ok，输入因此落到第 3 行。
    let same_family: Vec<IpAddr> = match exit_ip {
        None => reflexive_ips.to_vec(),
        Some(exit) => reflexive_ips
            .iter()
            .copied()
            .filter(|ip| ip.is_ipv4() == exit.is_ipv4())
            .collect(),
    };
    if same_family.len() < 2 {
        return done(UdpEgress::NotComparable(NotComparable::FamilyMismatch));
    }

    // 第 3 行。
    let Some(exit) = exit_ip else {
        return done(UdpEgress::NotComparable(NotComparable::UnknownExitIp));
    };

    // 第 4 行。多出口集群与对称 NAT 会自己落进这里——让误报源在对照里自曝，
    // 比用 /24 之类的启发式去猜它们要诚实。这是无从比对，不是未命中。
    let reflexive_ip = same_family[0];
    if same_family.iter().any(|ip| *ip != reflexive_ip) {
        return done(UdpEgress::NotComparable(NotComparable::StunDisagree));
    }

    // 第 5 / 6 行。
    done(UdpEgress::Comparable {
        mismatch: reflexive_ip != exit,
        reflexive_ip,
        exit_ip: exit,
    })
}

fn done(result: UdpEgress) -> Outcome<UdpEgress> {
    Outcome::Done(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(value: &str) -> IpAddr {
        value.parse().expect("测试里的地址必须可解析")
    }

    #[test]
    fn fewer_than_two_answers_is_a_failed_check_not_a_clean_one() {
        // 第 1 行。判成「未泄露」就是拿测量仪器没开冒充没问题。
        for observed in [vec![], vec![ip("203.0.113.7")]] {
            let outcome = judge(&observed, Some(ip("198.51.100.20")));
            assert!(!outcome.is_done(), "{observed:?}");
            assert_eq!(outcome.failure(), Some(Failure::Upstream));
        }
    }

    #[test]
    fn cross_family_addresses_are_not_comparable_never_a_hit() {
        // 第 2 行：srflx 是 IPv6 而出口是 IPv4，那是 O3 在管的事。
        let outcome = judge(
            &[ip("2001:db8::1"), ip("2001:db8::1")],
            Some(ip("198.51.100.20")),
        );
        assert_eq!(
            outcome.value().copied(),
            Some(UdpEgress::NotComparable(NotComparable::FamilyMismatch))
        );
        assert_eq!(outcome.value().and_then(UdpEgress::mismatch), None);
    }

    #[test]
    fn family_filtering_never_produces_a_failed_check() {
        // 契约 2.6 的分工：协议族筛选只会推进第 2 行，检测失败只由第 1 行产生。
        // 「答了但都是另一个协议族」刷新一万次也一样，不该让用户去刷。
        let outcome = judge(
            &[ip("2001:db8::1"), ip("2001:db8::2")],
            Some(ip("198.51.100.20")),
        );
        assert!(outcome.is_done());
    }

    #[test]
    fn a_mixed_family_list_keeps_only_the_exit_family() {
        // 两个 v4 同族且一致，混进来的 v6 不影响判定。
        let outcome = judge(
            &[ip("203.0.113.7"), ip("2001:db8::1"), ip("203.0.113.7")],
            Some(ip("198.51.100.20")),
        );
        assert_eq!(outcome.value().and_then(UdpEgress::mismatch), Some(true));
    }

    #[test]
    fn an_unknown_exit_ip_is_not_comparable() {
        // 第 3 行：O1 没完成时无从比对——出口 IP 未知，N_fam = N_ok。
        let outcome = judge(&[ip("203.0.113.7"), ip("203.0.113.7")], None);
        assert_eq!(
            outcome.value().copied(),
            Some(UdpEgress::NotComparable(NotComparable::UnknownExitIp))
        );
    }

    #[test]
    fn disagreeing_stuns_are_not_comparable_not_a_miss() {
        // 第 4 行。多出口集群与对称 NAT 自曝在这里，不贡献任何信号。
        let outcome = judge(
            &[ip("203.0.113.7"), ip("203.0.113.8")],
            Some(ip("198.51.100.20")),
        );
        assert_eq!(
            outcome.value().copied(),
            Some(UdpEgress::NotComparable(NotComparable::StunDisagree))
        );
        assert_eq!(outcome.value().and_then(UdpEgress::mismatch), None);
    }

    #[test]
    fn agreeing_stuns_decide_the_signal() {
        // 第 5 / 6 行。
        let exit = ip("198.51.100.20");
        assert_eq!(
            judge(&[exit, exit], Some(exit))
                .value()
                .and_then(UdpEgress::mismatch),
            Some(false)
        );
        assert_eq!(
            judge(&[ip("203.0.113.7"), ip("203.0.113.7")], Some(exit))
                .value()
                .and_then(UdpEgress::mismatch),
            Some(true)
        );
    }

    #[test]
    fn not_comparable_and_a_miss_are_told_apart_at_the_type_level() {
        // 「没测出来」说成「没问题」是本产品的红线：两者在这里连字段都不同一套。
        let miss = judge(
            &[ip("198.51.100.20"), ip("198.51.100.20")],
            Some(ip("198.51.100.20")),
        );
        let unknown = judge(
            &[ip("203.0.113.7"), ip("203.0.113.8")],
            Some(ip("198.51.100.20")),
        );
        assert_ne!(miss.value(), unknown.value());
        assert_eq!(miss.value().and_then(UdpEgress::mismatch), Some(false));
        assert_eq!(unknown.value().and_then(UdpEgress::mismatch), None);
    }
}
