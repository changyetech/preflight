//! C2 本地 DNS 服务器配置。
//!
//! **只读本机 DNS 配置，不做任何泄露判定**——「查询真的从哪出去了」是 O5 的职责
//! （契约 2.1 的 C2/O5 分工，ADR-0014）。两者不互相蕴含：配了国内 DNS 但走全局隧道的
//! 用户，这里亮黄而 O5 不命中；配了境外 DNS 却在分流模式下从本地出网的用户，反过来。
//!
//! 国内 DNS **只进检测建议，不贡献综合结论**（契约 2.1）——与 `ai-ipcheck` 一致。

use std::net::IpAddr;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct Server {
    pub address: String,
    /// 注册表条目；`None` 表示不是已知服务商。
    pub entry: Option<&'static crate::domain::dns_servers::Entry>,
    /// 是否是私网地址（局域网路由器）。
    pub private: bool,
    /// 是否是国内 DNS（注册表条目的 `region == "CN"`）。
    pub domestic: bool,
}

/// 解析地址，先去掉 IPv6 的 zone index（`fe80::1%en0` 的 `%en0`）。
///
/// std 的解析器不认 zone index，直接 `parse()` 会把这类地址**整条丢掉**——
/// 而 macOS 上路由器下发的 DNS 恰恰常是这个形态（`scutil --dns` 实测）。
fn parse_addr(value: &str) -> Option<IpAddr> {
    value.split('%').next()?.trim().parse().ok()
}

pub fn describe(address: &str) -> Server {
    let known = crate::domain::dns_servers::lookup(address);
    let private = parse_addr(address)
        .map(|ip| match ip {
            IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
            // link-local（fe80::/10）在 macOS 上就是路由器地址。
            IpAddr::V6(v6) => {
                v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local()
            }
        })
        .unwrap_or(false);

    Server {
        address: address.to_string(),
        entry: known,
        private,
        domestic: known.is_some_and(|e| e.region == "CN"),
    }
}

/// 解析 `/etc/resolv.conf`。
pub fn parse_resolv_conf(content: &str) -> Vec<String> {
    dedup(
        content
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                // `#` 与 `;` 都是 resolv.conf 的注释符。
                if line.starts_with('#') || line.starts_with(';') {
                    return None;
                }
                let rest = line.strip_prefix("nameserver")?;
                let addr = rest.trim();
                parse_addr(addr).map(|_| addr.to_string())
            })
            .collect(),
    )
}

/// 解析 `scutil --dns` 的输出。
pub fn parse_scutil_dns(output: &str) -> Vec<String> {
    dedup(
        output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if !line.starts_with("nameserver[") {
                    return None;
                }
                let addr = line.split_once(':')?.1.trim();
                parse_addr(addr).map(|_| addr.to_string())
            })
            .collect(),
    )
}

/// 解析 `networksetup -getdnsservers <服务>` 的输出。
///
/// 用户没手动设置时它返回一句英文说明而不是地址，因此**只收能解析成 IP 的行**。
#[cfg(any(target_os = "windows", target_os = "macos", test))]
pub fn parse_networksetup_dns(output: &str) -> Vec<String> {
    dedup(
        output
            .lines()
            .filter_map(|line| {
                let addr = line.trim();
                parse_addr(addr).map(|_| addr.to_string())
            })
            .collect(),
    )
}

/// 解析 `networksetup -listallnetworkservices`：首行是说明文字，`*` 前缀表示已禁用。
#[cfg(any(target_os = "macos", test))]
pub fn parse_network_services(output: &str) -> Vec<String> {
    output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let name = line.trim();
            (!name.is_empty() && !name.starts_with('*')).then(|| name.to_string())
        })
        .collect()
}

fn dedup(items: Vec<String>) -> Vec<String> {
    let mut seen = Vec::new();
    for item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    seen
}

fn run(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 采集本机 DNS。返回空列表表示采集失败——调用方据此判 C2 检测失败。
pub fn probe() -> Vec<Server> {
    let addresses = collect_addresses();
    addresses.iter().map(|a| describe(a)).collect()
}

fn collect_addresses() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(out) = run(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "Get-DnsClientServerAddress -AddressFamily IPv4 | Select-Object -ExpandProperty ServerAddresses",
            ],
        ) {
            return parse_networksetup_dns(&out);
        }
        return Vec::new();
    }

    #[cfg(not(target_os = "windows"))]
    {
        // macOS 优先取用户手动设置的 DNS：`/etc/resolv.conf` 会被 Tailscale/VPN 顶掉，
        // 那样就漏掉了用户真正设的那个。
        #[cfg(target_os = "macos")]
        {
            let manual = macos_manual_dns();
            if !manual.is_empty() {
                return manual;
            }
        }

        if let Ok(content) = std::fs::read_to_string("/etc/resolv.conf") {
            let servers = parse_resolv_conf(&content);
            if !servers.is_empty() {
                return servers;
            }
        }

        run("scutil", &["--dns"])
            .map(|out| parse_scutil_dns(&out))
            .unwrap_or_default()
    }
}

#[cfg(target_os = "macos")]
fn macos_manual_dns() -> Vec<String> {
    let Some(listing) = run("networksetup", &["-listallnetworkservices"]) else {
        return Vec::new();
    };

    let mut servers = Vec::new();
    for service in parse_network_services(&listing) {
        if let Some(out) = run("networksetup", &["-getdnsservers", &service]) {
            for addr in parse_networksetup_dns(&out) {
                if !servers.contains(&addr) {
                    servers.push(addr);
                }
            }
        }
    }
    servers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_known_providers_and_flags_domestic_ones() {
        let entry = describe("1.1.1.1").entry.expect("1.1.1.1 应识别");
        assert_eq!(entry.name, "Cloudflare");
        assert_eq!(entry.region, "US");
        assert!(!describe("1.1.1.1").domestic);
        assert!(describe("223.5.5.5").domestic);
        assert!(describe("114.114.114.114").domestic);
    }

    #[test]
    fn recognises_router_addresses_as_private() {
        assert!(describe("192.168.1.1").private);
        assert!(describe("10.0.0.1").private);
        assert!(!describe("8.8.8.8").private);
    }

    #[test]
    fn unknown_public_dns_is_neither_labelled_nor_domestic() {
        let server = describe("203.0.113.53");
        assert_eq!(server.entry, None);
        assert!(!server.domestic);
        assert!(!server.private);
    }

    #[test]
    fn parses_resolv_conf_ignoring_comments_and_duplicates() {
        let content = "# generated\nnameserver 8.8.8.8\n; another comment\nnameserver 8.8.8.8\nnameserver 1.1.1.1\nsearch example.com\n";
        assert_eq!(parse_resolv_conf(content), vec!["8.8.8.8", "1.1.1.1"]);
    }

    #[test]
    fn ipv6_addresses_with_a_zone_index_are_not_dropped() {
        // 实测：macOS 上 `scutil --dns` 给的就是 `fe80::1%en0`。
        // 直接 parse 会把它整条丢掉，用户会看到少一台 DNS 服务器。
        let output = "  nameserver[0] : fe80::1%en0\n  nameserver[1] : 192.168.1.1\n";
        assert_eq!(parse_scutil_dns(output), vec!["fe80::1%en0", "192.168.1.1"]);

        let server = describe("fe80::1%en0");
        assert_eq!(server.address, "fe80::1%en0");
        assert!(server.private, "link-local 是路由器地址");
    }

    #[test]
    fn parses_scutil_output() {
        let output = "DNS configuration\n\nresolver #1\n  nameserver[0] : 192.168.1.1\n  nameserver[1] : 8.8.8.8\n  flags  : Request A records\n";
        assert_eq!(parse_scutil_dns(output), vec!["192.168.1.1", "8.8.8.8"]);
    }

    #[test]
    fn networksetup_placeholder_text_yields_no_servers() {
        // 用户没手动设 DNS 时 networksetup 返回一句英文说明，不能被当成地址。
        assert!(parse_networksetup_dns("There aren't any DNS Servers set on Wi-Fi.").is_empty());
        assert_eq!(
            parse_networksetup_dns("223.5.5.5\n223.6.6.6\n"),
            vec!["223.5.5.5", "223.6.6.6"]
        );
    }

    #[test]
    fn disabled_network_services_are_skipped() {
        let listing = "An asterisk (*) denotes that a network service is disabled.\nWi-Fi\n*Thunderbolt Bridge\nEthernet\n";
        assert_eq!(parse_network_services(listing), vec!["Wi-Fi", "Ethernet"]);
    }
}
