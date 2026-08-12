//! 检测项与覆盖度。契约见 docs/verdict.md 第 1 / 4 节。

use std::fmt;

/// 8 个检测项。ID 是跨端稳定标识，**不得复用、不得改号**——
/// 这条对已删除的 ID 同样成立：`C5`（原厂商端点检测）是废弃编号，不得复用（ADR-0013）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckId {
    /// 出口 IP 与归属。
    O1,
    /// 系统时区一致性（系统时区 vs 出口 IP 时区）。
    O2,
    /// IPv6 泄露。
    O3,
    /// IP 类型与风险。
    O4,
    /// 本机真实 IP。
    C1,
    /// 本地 DNS 服务器与 DNS 泄露。
    C2,
    /// 代理检测（环境变量 / 系统代理 / TUN）。
    C3,
    /// `$TZ` 时区一致性（命令行进程认的那个）。
    C4,
}

/// 覆盖度的分母恒为 8。
pub const TOTAL_CHECKS: usize = 8;

pub const ALL_CHECKS: [CheckId; TOTAL_CHECKS] = [
    CheckId::O1,
    CheckId::O2,
    CheckId::O3,
    CheckId::O4,
    CheckId::C1,
    CheckId::C2,
    CheckId::C3,
    CheckId::C4,
];

impl CheckId {
    pub const fn as_str(self) -> &'static str {
        match self {
            CheckId::O1 => "O1",
            CheckId::O2 => "O2",
            CheckId::O3 => "O3",
            CheckId::O4 => "O4",
            CheckId::C1 => "C1",
            CheckId::C2 => "C2",
            CheckId::C3 => "C3",
            CheckId::C4 => "C4",
        }
    }
}

impl fmt::Display for CheckId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 检测项失败的原因。分档是为了让呈现层能给出**用户能据以行动**的提示，
/// 而不是一律「检测失败」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// 第三方不可用：网络失败、非 200、响应不合法、`status != "ok"`。
    Upstream,
    /// proxycheck 当日配额用尽。单独成一档，因为提示语不同——用户可以配 key 解决。
    QuotaExhausted,
    /// 读本机环境失败。
    Local,
}

/// 一个检测项的终态。
///
/// CLI 侧只有这两种：没有「需 CLI」（它就是 CLI），也没有「按需未测」（O4 自动执行）。
/// 「检测中」不是终态，不会进入报告，因此不在取值域内——一次性渲染意味着
/// 我们只在全部探测结束后才构造这个类型。
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome<T> {
    Done(T),
    Failed(Failure),
}

impl<T> Outcome<T> {
    pub const fn is_done(&self) -> bool {
        matches!(self, Outcome::Done(_))
    }

    pub const fn value(&self) -> Option<&T> {
        match self {
            Outcome::Done(v) => Some(v),
            Outcome::Failed(_) => None,
        }
    }

    pub const fn failure(&self) -> Option<Failure> {
        match self {
            Outcome::Done(_) => None,
            Outcome::Failed(f) => Some(*f),
        }
    }
}

/// 覆盖度。CLI 侧的不变量是 `done + failed == 8`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Coverage {
    pub done: usize,
    pub failed: usize,
}

impl Coverage {
    pub fn tally(outcomes: impl IntoIterator<Item = bool>) -> Self {
        let mut coverage = Coverage::default();
        for done in outcomes {
            if done {
                coverage.done += 1;
            } else {
                coverage.failed += 1;
            }
        }
        coverage
    }

    /// 不变量：两档之和恒为 8。呈现层在打印前断言它。
    pub const fn is_complete(&self) -> bool {
        self.done + self.failed == TOTAL_CHECKS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_exactly_eight_checks_with_distinct_ids() {
        assert_eq!(ALL_CHECKS.len(), TOTAL_CHECKS);
        let mut seen: Vec<&str> = ALL_CHECKS.iter().map(|id| id.as_str()).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), TOTAL_CHECKS, "检测项 ID 必须互不相同");
    }

    #[test]
    fn coverage_invariant_holds_for_any_mix() {
        // 全成功 / 全失败 / 混合，两档之和恒为 8。
        for failed_count in 0..=TOTAL_CHECKS {
            let outcomes = (0..TOTAL_CHECKS).map(|i| i >= failed_count);
            let coverage = Coverage::tally(outcomes);
            assert!(coverage.is_complete(), "{coverage:?}");
            assert_eq!(coverage.failed, failed_count);
            assert_eq!(coverage.done, TOTAL_CHECKS - failed_count);
        }
    }

    #[test]
    fn quota_exhausted_is_its_own_failure_kind() {
        // 契约要求它计入「检测失败」，但提示语不同——用户可以配 key 解决它。
        let outcome: Outcome<()> = Outcome::Failed(Failure::QuotaExhausted);
        assert!(!outcome.is_done());
        assert_eq!(outcome.failure(), Some(Failure::QuotaExhausted));
        assert_ne!(Failure::QuotaExhausted, Failure::Upstream);
    }
}
