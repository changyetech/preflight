//! C3 代理检测：环境变量代理 / 系统代理 / TUN。
//!
//! **只报开关状态，绝不显示地址**——把 `127.0.0.1:7890` 打在屏幕上等于替用户泄露配置。
//! 与 `ai-ipcheck` 一致。
//!
//! 系统代理只实现 macOS（`scutil --proxy`），其他平台报**未实现**而不是「未开启」：
//! 「没检测」与「检测了、没开」是两回事，后者才是一个可以据以判断的结论。

use std::process::Command;

const PROXY_ENV_VARS: [&str; 6] = [
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
];

/// 一项检测的三态。`Unsupported` 是"本平台没实现"，不是"没开"。
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Enabled,
    Disabled,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    /// 环境变量代理：只记哪些变量被设了，**不记它们的值**。
    pub env_vars: Vec<String>,
    pub system: State,
    /// 系统代理的种类（`HTTP` / `HTTPS` / `SOCKS` / `PAC`），同样不含地址。
    pub system_kinds: Vec<String>,
    pub tun: State,
}

impl Status {
    pub fn env_state(&self) -> State {
        if self.env_vars.is_empty() {
            State::Disabled
        } else {
            State::Enabled
        }
    }

    /// `tunOff` 信号：TUN 明确未开启才是 `Some(true)`。
    ///
    /// **平台不支持时是未知（`None`），不贡献综合结论**（契约 2.3）。这是相对
    /// `ai-ipcheck` 的一处刻意变更：那边 `tun_active is not True` 把"检测不到"
    /// 也算成中风险，于是每个 Windows 用户都被永久判中风险，且无从解决。
    pub fn tun_off(&self) -> Option<bool> {
        match self.tun {
            State::Enabled => Some(false),
            State::Disabled => Some(true),
            State::Unsupported => None,
        }
    }
}

/// 读环境变量代理。**只返回变量名**。
pub fn env_proxy_vars() -> Vec<String> {
    PROXY_ENV_VARS
        .iter()
        .filter(|name| {
            std::env::var(name)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        })
        .map(|name| name.to_ascii_uppercase())
        .fold(Vec::new(), |mut acc, name| {
            if !acc.contains(&name) {
                acc.push(name);
            }
            acc
        })
}

/// 解析 `scutil --proxy`。返回启用的代理种类，**不含主机与端口**。
pub fn parse_macos_proxy(output: &str) -> Vec<String> {
    let mut config = std::collections::HashMap::new();
    for line in output.lines() {
        if let Some((key, value)) = line.split_once(':') {
            config.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    let mut kinds = Vec::new();
    for prefix in ["HTTP", "HTTPS", "SOCKS"] {
        if config.get(&format!("{prefix}Enable")).map(String::as_str) != Some("1") {
            continue;
        }
        // 与 ai-ipcheck 一致：主机与端口都在才算数，缺一个说明配置不完整。
        let has_host = config.contains_key(&format!("{prefix}Proxy"));
        let has_port = config.contains_key(&format!("{prefix}Port"));
        if has_host && has_port {
            kinds.push(prefix.to_string());
        }
    }

    if config.get("ProxyAutoConfigEnable").map(String::as_str) == Some("1") {
        kinds.push("PAC".to_string());
    }

    kinds
}

/// 解析 `ifconfig` + 路由表，判断 TUN/VPN 是否在承载流量。
///
/// 只有接口存在是不够的——macOS 上 `utun0/1/2` 常年存在（iCloud、Handoff 都会建）。
/// 判据是：接口拿到了 `198.18.0.0/15` 这段（代理软件 TUN 模式的惯用网段），
/// 或者路由表里有指向这些接口的条目。
pub fn parse_tun(ifconfig: &str, routes: &str) -> bool {
    let mut interfaces = Vec::new();
    let mut active = false;

    let mut current: Option<String> = None;
    for line in ifconfig.lines() {
        if !line.starts_with(char::is_whitespace)
            && let Some((name, _)) = line.split_once(':')
        {
            current = is_tunnel_name(name).then(|| name.to_string());
            if let Some(name) = &current
                && !interfaces.contains(name)
            {
                interfaces.push(name.clone());
            }
            continue;
        }

        if current.is_some()
            && let Some(addr) = line.split_whitespace().skip_while(|t| *t != "inet").nth(1)
            && addr.starts_with("198.18.")
        {
            active = true;
        }
    }

    for line in routes.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 4 {
            continue;
        }
        if tokens[1].starts_with("198.18.") {
            active = true;
        }
        if tokens.iter().any(|t| is_tunnel_name(t)) {
            active = true;
        }
    }

    active
}

fn is_tunnel_name(name: &str) -> bool {
    let name = name.trim();
    ["utun", "tun", "tap", "wg", "ppp"].iter().any(|prefix| {
        name.starts_with(prefix) && name[prefix.len()..].chars().all(|c| c.is_ascii_digit())
    })
}

fn run(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn probe() -> Status {
    let env_vars = env_proxy_vars();

    let (system, system_kinds) = if cfg!(target_os = "macos") {
        match run("scutil", &["--proxy"]) {
            Some(out) => {
                let kinds = parse_macos_proxy(&out);
                (
                    if kinds.is_empty() {
                        State::Disabled
                    } else {
                        State::Enabled
                    },
                    kinds,
                )
            }
            None => (State::Unsupported, Vec::new()),
        }
    } else {
        // 「未实现」不是「未开启」。
        (State::Unsupported, Vec::new())
    };

    let tun = if cfg!(target_os = "windows") {
        State::Unsupported
    } else {
        let ifconfig = run("ifconfig", &[]);
        let routes = if cfg!(target_os = "macos") {
            run("netstat", &["-rn", "-f", "inet"])
        } else {
            run("ip", &["route"])
        };
        match (ifconfig, routes) {
            (Some(i), Some(r)) => {
                if parse_tun(&i, &r) {
                    State::Enabled
                } else {
                    State::Disabled
                }
            }
            _ => State::Unsupported,
        }
    };

    Status {
        env_vars,
        system,
        system_kinds,
        tun,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_proxy_reports_kinds_without_addresses() {
        let output = "<dictionary> {\n  HTTPEnable : 1\n  HTTPProxy : 127.0.0.1\n  HTTPPort : 7890\n  HTTPSEnable : 1\n  HTTPSProxy : 127.0.0.1\n  HTTPSPort : 7890\n  SOCKSEnable : 0\n}";
        let kinds = parse_macos_proxy(output);
        assert_eq!(kinds, vec!["HTTP", "HTTPS"]);
        // 地址绝不能出现在结果里。
        assert!(
            !kinds
                .iter()
                .any(|k| k.contains("127.0.0.1") || k.contains("7890"))
        );
    }

    #[test]
    fn pac_counts_as_a_system_proxy() {
        let output = "<dictionary> {\n  ProxyAutoConfigEnable : 1\n  ProxyAutoConfigURLString : http://wpad/proxy.pac\n}";
        assert_eq!(parse_macos_proxy(output), vec!["PAC"]);
    }

    #[test]
    fn enabled_without_host_or_port_does_not_count() {
        let output = "<dictionary> {\n  HTTPEnable : 1\n}";
        assert!(parse_macos_proxy(output).is_empty());
    }

    #[test]
    fn nothing_enabled_yields_no_kinds() {
        let output = "<dictionary> {\n  HTTPEnable : 0\n  HTTPSEnable : 0\n  SOCKSEnable : 0\n}";
        assert!(parse_macos_proxy(output).is_empty());
    }

    #[test]
    fn idle_utun_interfaces_do_not_count_as_tun_active() {
        // macOS 上 utun0/1/2 常年存在（iCloud、Handoff），光有接口说明不了什么。
        let ifconfig = "lo0: flags=8049\n\tinet 127.0.0.1 netmask 0xff000000\nutun0: flags=8051\n\tinet6 fe80::1 prefixlen 64\nutun1: flags=8051\n\tinet6 fe80::2 prefixlen 64\n";
        let routes = "Destination        Gateway            Flags   Netif\ndefault            192.168.1.1        UGScg     en0\n";
        assert!(!parse_tun(ifconfig, routes));
    }

    #[test]
    fn tun_carrying_the_proxy_subnet_counts_as_active() {
        let ifconfig = "utun4: flags=8051\n\tinet 198.18.0.1 --> 198.18.0.1 netmask 0xffff0000\n";
        let routes = "Destination        Gateway            Flags   Netif\ndefault            192.168.1.1        UGScg     en0\n";
        assert!(parse_tun(ifconfig, routes));
    }

    #[test]
    fn a_route_through_a_tunnel_interface_counts_as_active() {
        let ifconfig = "utun4: flags=8051\n\tinet6 fe80::1 prefixlen 64\n";
        let routes = "Destination        Gateway            Flags   Netif\ndefault            198.18.0.1         UGScg   utun4\n";
        assert!(parse_tun(ifconfig, routes));
    }

    #[test]
    fn unsupported_tun_is_unknown_not_off() {
        // 相对 ai-ipcheck 的刻意变更：那边把"检测不到"也算成中风险，
        // 于是每个 Windows 用户都被永久判中风险，且无从解决。
        let status = Status {
            env_vars: Vec::new(),
            system: State::Unsupported,
            system_kinds: Vec::new(),
            tun: State::Unsupported,
        };
        assert_eq!(status.tun_off(), None);

        let off = Status {
            tun: State::Disabled,
            ..status.clone()
        };
        assert_eq!(off.tun_off(), Some(true));

        let on = Status {
            tun: State::Enabled,
            ..status
        };
        assert_eq!(on.tun_off(), Some(false));
    }
}
