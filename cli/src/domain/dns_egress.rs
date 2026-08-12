//! O5 DNS 出口泄露判定（判级契约 docs/verdict.md 2.5 判定表）。
//!
//! 判据是**出口 resolver 眼里客户端在哪个国家**，与出口 IP 的归属国比。
//! 探测层只把 ip-api 的两个 geo 字符串原样递过来，切国家名与查表都在这里——
//! 换 provider 时上层一行不用动。

use std::collections::HashMap;
use std::sync::OnceLock;

use super::checks::{Failure, Outcome};

/// 英文国家名 → ISO2。两端共吃 docs/country-codes.json：Web 打进 bundle，CLI 编译期内联。
///
/// 用 `include_str!` 而非运行时读取：不必解析相对路径、不受 cwd 影响，且 cargo 会把它
/// 登记为构建依赖——改这份表会触发重编译。
const COUNTRY_CODES: &str = include_str!("../../../docs/country-codes.json");

fn iso2_by_country_name() -> &'static HashMap<String, String> {
    static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    TABLE.get_or_init(|| {
        #[derive(serde::Deserialize)]
        struct Table {
            countries: HashMap<String, String>,
        }
        let table: Table =
            serde_json::from_str(COUNTRY_CODES).expect("country-codes.json 必须可解析");
        table.countries
    })
}

/// 单次 DNS 出口探测的原始观测。
///
/// 探测**失败**（网络错误、响应不可解析）在类型外表达——探测层返回 `None`，
/// 对应判定表最后一行的「O5 检测失败」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// ECS 客户端子网归属，形如 `"Japan - IT7 Networks Inc"`；`None` = 响应里没有 ECS 段。
    pub ecs_geo: Option<String>,
    /// 出口 resolver 自身的归属。**只展示，不判定**（契约 2.1 / 2.5 硬约束 1）。
    pub resolver_geo: Option<String>,
}

/// ECS 客户端子网的归属国，已归一化为 ISO2。
///
/// 两种「未知」必须分开：不发 ECS 的服务商（Cloudflare 1.1.1.1 是明确的一家）与
/// 认不出的国家名，前者是用户可以换 DNS 解决的，后者是我们的表不全。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EcsCountry {
    Known(String),
    /// 响应中没有 ECS 段。
    NoEcs,
    /// 有 ECS 段，但国家名映射不出 ISO2。
    Unmapped,
}

/// 「无从比对」的三种成因。呈现层据此选文案——它们对用户的可行动性完全不同。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotComparable {
    NoEcs,
    UnmappedCountry,
    UnknownExitCountry,
}

/// 比对结果（契约 2.5 判定表）。
///
/// 「无从比对」与「未命中」是两回事，因此**不是** `Option<bool>` 加注释，而是判别式枚举：
/// 没有 ECS、国家名查不到表、出口国未知，三者都不得回退成「两国不同」（2.5 硬约束 3），
/// 而把它们表达成一个可空布尔量，迟早有人写出 `!leak` 就当成绿色。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    Comparable {
        leak: bool,
        /// ECS 客户端子网归属国，ISO2。
        ecs_country: String,
        /// 出口 IP 归属国，ISO2。两个国家都必须呈现（契约 5.4 呈现约束）。
        exit_country: String,
    },
    NotComparable(NotComparable),
}

impl Comparison {
    /// 折成判级信号。**「无从比对」产出 `None`，不是 `Some(false)`**——
    /// 按契约 2.3 未知不冒充任何一侧，而这个方法是判级层唯一的入口。
    pub fn leak(&self) -> Option<bool> {
        match self {
            Comparison::Comparable { leak, .. } => Some(*leak),
            Comparison::NotComparable(_) => None,
        }
    }
}

/// O5 的检测项数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsEgress {
    /// ip-api 的 `dns.geo` 原样字符串。**只展示，不判定**：resolver 在哪个国家取决于
    /// 用户选了哪家 DNS 服务商，与流量走没走代理无关，拿它判定是系统性误报。
    pub resolver_geo: Option<String>,
    pub comparison: Comparison,
}

/// ip-api 的 geo 字符串形如 `"<国家名> - <组织名>"`，判定只要前半段（契约 2.5 步骤 2）。
pub fn ecs_country_of(ecs_geo: Option<&str>) -> EcsCountry {
    // `NoEcs` 只留给**真的没有 ECS 段**的情形：呈现层据它写「你的 DNS 服务商不发送 ECS」，
    // 而有 ECS 段却切不出国家名（如 `" - Some ISP"`）时那句话是假的。
    let Some(geo) = ecs_geo.map(str::trim).filter(|g| !g.is_empty()) else {
        return EcsCountry::NoEcs;
    };

    let name = geo.split(" - ").next().unwrap_or_default().trim();
    if name.is_empty() {
        return EcsCountry::Unmapped;
    }

    // 查不到一律视为未知（契约 2.5 硬约束 3）：把「我不认识这个国家名」当成「两国不同」
    // 会凭空造出误报，而误报比漏报更快毁掉用户对本产品的信任。
    match iso2_by_country_name().get(name) {
        Some(iso2) => EcsCountry::Known(iso2.clone()),
        None => EcsCountry::Unmapped,
    }
}

/// 判定表本体（契约 2.5）。两侧都已是 ISO2——golden 向量正是在这一层给输入。
///
/// 比的是国家而不是 ECS 的 IP 前缀：掩码位数由 resolver 决定且响应里不一定给出，
/// 比前缀是掩码敏感的，比国家不是（2.5 硬约束 2）。
pub fn compare(ecs: &EcsCountry, exit_country: Option<&str>) -> Comparison {
    let ecs_country = match ecs {
        EcsCountry::Known(iso2) => iso2.trim().to_ascii_uppercase(),
        EcsCountry::NoEcs => return Comparison::NotComparable(NotComparable::NoEcs),
        EcsCountry::Unmapped => return Comparison::NotComparable(NotComparable::UnmappedCountry),
    };

    let exit = exit_country.unwrap_or_default().trim().to_ascii_uppercase();
    if exit.is_empty() {
        return Comparison::NotComparable(NotComparable::UnknownExitCountry);
    }

    Comparison::Comparable {
        leak: ecs_country != exit,
        ecs_country,
        exit_country: exit,
    }
}

/// 探测结果 + O1 的出口国 → O5 的终态。
///
/// **「无从比对」记「已完成」而不是「检测失败」**：探测确实成功了，只是回答里不含可判定的
/// 信息。记成失败会诱导用户反复刷新一个永远不会变的结果（契约 2.5）。
pub fn judge(observation: Option<&Observation>, exit_country: Option<&str>) -> Outcome<DnsEgress> {
    let Some(observation) = observation else {
        return Outcome::Failed(Failure::Upstream);
    };

    Outcome::Done(DnsEgress {
        resolver_geo: observation.resolver_geo.clone(),
        comparison: compare(
            &ecs_country_of(observation.ecs_geo.as_deref()),
            exit_country,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(ecs_geo: Option<&str>) -> Observation {
        Observation {
            ecs_geo: ecs_geo.map(str::to_string),
            resolver_geo: Some("Japan - Google LLC".into()),
        }
    }

    #[test]
    fn the_country_name_is_taken_from_before_the_dash_and_mapped_to_iso2() {
        assert_eq!(
            ecs_country_of(Some("Japan - IT7 Networks Inc")),
            EcsCountry::Known("JP".into())
        );
        // 组织名里含 " - " 也只取第一段。
        assert_eq!(
            ecs_country_of(Some("United States - Foo - Bar")),
            EcsCountry::Known("US".into())
        );
    }

    #[test]
    fn a_missing_ecs_section_is_told_apart_from_an_unmapped_country_name() {
        // 两种未知的可行动性不同：前者换 DNS 服务商能解决，后者是我们的表不全。
        assert_eq!(ecs_country_of(None), EcsCountry::NoEcs);
        assert_eq!(ecs_country_of(Some("   ")), EcsCountry::NoEcs);
        assert_eq!(ecs_country_of(Some(" - Some ISP")), EcsCountry::Unmapped);
    }

    #[test]
    fn an_unmappable_country_name_is_unknown_never_a_leak() {
        // 契约 2.5 判定表第 4 行 + 硬约束 3：这一行在 golden 向量层不可测
        // （向量的输入已经是 ISO2，映射发生在探测层之前），只能由本单测覆盖。
        // 把「我不认识这个国家名」当成「两国不同」会凭空造出一次泄露告警。
        assert_eq!(
            ecs_country_of(Some("Neverland - Some ISP")),
            EcsCountry::Unmapped
        );

        let comparison = compare(&ecs_country_of(Some("Neverland - Some ISP")), Some("JP"));
        assert_eq!(
            comparison,
            Comparison::NotComparable(NotComparable::UnmappedCountry)
        );
        assert_eq!(comparison.leak(), None, "无从比对不得贡献信号");
    }

    #[test]
    fn same_country_is_a_miss_and_a_different_one_is_a_leak() {
        assert_eq!(
            compare(&EcsCountry::Known("JP".into()), Some("JP")).leak(),
            Some(false)
        );
        assert_eq!(
            compare(&EcsCountry::Known("US".into()), Some("JP")).leak(),
            Some(true)
        );
        // 大小写与空白归一后再比。
        assert_eq!(
            compare(&EcsCountry::Known("jp".into()), Some(" JP ")).leak(),
            Some(false)
        );
    }

    #[test]
    fn an_unknown_exit_country_is_not_comparable() {
        // O1 没完成或没给出国家 ⇒ 判定表第 5 行。
        for exit in [None, Some(""), Some("   ")] {
            assert_eq!(
                compare(&EcsCountry::Known("JP".into()), exit),
                Comparison::NotComparable(NotComparable::UnknownExitCountry)
            );
        }
    }

    #[test]
    fn not_comparable_is_still_a_completed_check() {
        // 探测成功了，只是回答里不含可判定的信息。记成失败会诱导用户反复刷新
        // 一个永远不会变的结果（契约 2.5）。
        let outcome = judge(Some(&seen(None)), Some("JP"));
        assert!(outcome.is_done());
        assert_eq!(
            outcome.value().map(|d| d.comparison.clone()),
            Some(Comparison::NotComparable(NotComparable::NoEcs))
        );
        // resolver 归属仍然带出来给呈现层——展示，不判定。
        assert_eq!(
            outcome.value().and_then(|d| d.resolver_geo.clone()),
            Some("Japan - Google LLC".into())
        );
    }

    #[test]
    fn a_failed_probe_is_a_failed_check_not_a_clean_one() {
        let outcome = judge(None, Some("JP"));
        assert!(!outcome.is_done());
        assert_eq!(outcome.failure(), Some(Failure::Upstream));
    }

    #[test]
    fn the_shared_country_table_is_parsable_and_covers_ip_api_spellings() {
        let table = iso2_by_country_name();
        assert!(table.len() > 200, "表被截断了？{}", table.len());
        // 别名两种写法都必须在（表的 conventions.aliases）——少收一个写法就是一次「查不到」。
        assert_eq!(table.get("Netherlands"), Some(&"NL".to_string()));
        assert_eq!(table.get("The Netherlands"), Some(&"NL".to_string()));
        assert_eq!(table.get("Türkiye"), Some(&"TR".to_string()));
    }
}
