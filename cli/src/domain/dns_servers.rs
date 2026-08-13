//! 公共 DNS 注册表。两端共吃 docs/dns-servers.json：Web 打进 bundle，CLI 编译期内联。
//!
//! 与 `country-codes.json` 同模式（ADR-0005）：`include_str!` 让 cargo 把这份表登记为
//! 构建依赖，改 JSON 会触发重编译。解析在首次访问时做一次，之后复用。
//!
//! C2 探测（`probe/dns.rs`）的「已知服务商识别 + 国内标记」与 `dns` 命令的清单展示
//! 共用这一份数据——同一台机器上两处对同一 IP 的称呼因此一致。

use std::sync::OnceLock;

/// DNS 服务商的过滤级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    /// 普通解析，不做内容过滤。
    Standard,
    /// 拦截恶意 / 钓鱼域名。
    Security,
    /// 在 `Security` 基础上再拦成人内容。
    Family,
    /// 拦截广告与追踪域名。
    Adblock,
}

/// 注册表中的一条 DNS 服务商条目。
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Entry {
    pub ip: String,
    /// 品牌名 / 专名，两语种共用不翻译。
    pub name: String,
    /// ISO2 国家码。
    pub region: String,
    pub domestic: bool,
    pub variant: Variant,
}

/// 注册表文件。用 `include_str!` 内联，不依赖 cwd。
const DNS_SERVERS: &str = include_str!("../../../docs/dns-servers.json");

/// 全量条目，首次访问时解析。解析失败 = 编译期数据错误，panic 合理。
fn entries() -> &'static [Entry] {
    static TABLE: OnceLock<Vec<Entry>> = OnceLock::new();
    TABLE.get_or_init(|| {
        #[derive(serde::Deserialize)]
        struct Registry {
            servers: Vec<Entry>,
        }
        let reg: Registry = serde_json::from_str(DNS_SERVERS).expect("dns-servers.json 必须可解析");
        reg.servers
    })
}

/// 按 IP 查找已知服务商。
pub fn lookup(ip: &str) -> Option<&'static Entry> {
    entries().iter().find(|e| e.ip == ip)
}

/// 全量条目（清单原序）。
#[allow(dead_code)] // `dns` 命令（--cli 子计划）将消费
pub fn all() -> &'static [Entry] {
    entries()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_27_entries() {
        assert_eq!(entries().len(), 27);
    }

    #[test]
    fn lookup_finds_known_provider() {
        let e = lookup("1.1.1.1").expect("1.1.1.1 应在注册表中");
        assert_eq!(e.name, "Cloudflare");
        assert!(!e.domestic);
        assert_eq!(e.variant, Variant::Standard);
    }

    #[test]
    fn lookup_misses_unknown_ip() {
        assert!(lookup("203.0.113.53").is_none());
    }

    #[test]
    fn domestic_entries_flagged_correctly() {
        assert!(lookup("223.5.5.5").unwrap().domestic);
        assert!(lookup("114.114.114.114").unwrap().domestic);
        assert!(!lookup("8.8.8.8").unwrap().domestic);
    }

    #[test]
    fn no_duplicate_ips() {
        let mut ips: Vec<&str> = entries().iter().map(|e| e.ip.as_str()).collect();
        ips.sort();
        let before = ips.len();
        ips.dedup();
        assert_eq!(ips.len(), before, "存在重复 IP");
    }

    #[test]
    fn every_ip_is_valid_ipv4() {
        for e in entries() {
            assert!(
                e.ip.parse::<std::net::Ipv4Addr>().is_ok(),
                "{} 不是合法 IPv4",
                e.ip
            );
        }
    }

    #[test]
    fn every_region_is_two_uppercase_letters() {
        for e in entries() {
            assert_eq!(e.region.len(), 2, "{} 的 region 不是两位", e.ip);
            assert!(
                e.region.chars().all(|c| c.is_ascii_uppercase()),
                "{} 的 region 不是全大写",
                e.ip
            );
        }
    }

    #[test]
    fn every_ip_set_matches_old_constant() {
        // 回归锚：新注册表的 IP 集合必须与被删除的 KNOWN_DNS 常量完全一致。
        let old_ips: std::collections::HashSet<&str> = [
            "1.1.1.1",
            "1.0.0.1",
            "1.1.1.2",
            "1.0.0.2",
            "1.1.1.3",
            "1.0.0.3",
            "8.8.8.8",
            "8.8.4.4",
            "9.9.9.9",
            "149.112.112.112",
            "208.67.222.222",
            "208.67.220.220",
            "223.5.5.5",
            "223.6.6.6",
            "119.29.29.29",
            "182.254.116.116",
            "114.114.114.114",
            "114.114.115.115",
            "180.76.76.76",
            "1.2.4.8",
            "210.2.4.8",
            "94.140.14.14",
            "94.140.15.15",
            "185.228.168.9",
            "185.228.169.9",
            "76.76.2.0",
            "76.76.10.0",
        ]
        .into_iter()
        .collect();
        let new_ips: std::collections::HashSet<&str> =
            entries().iter().map(|e| e.ip.as_str()).collect();
        assert_eq!(old_ips, new_ips, "注册表 IP 集合与原 KNOWN_DNS 不一致");
    }
}
