//! C1 本机真实 IP —— 国内直连回显。
//!
//! 规则代理（Clash 等）默认对国内 IP 走直连，因此请求国内回显服务会**绕过代理**，
//! 露出真实 ISP 出口而非代理出口。这是 ipcheck Web 结构性拿不到的东西
//! （CONTEXT.md：「真实 IP」一词只在 CLI 语境成立）。

use super::http;

/// 端点顺序与 `ai-ipcheck` 一致：先两个纯 IP 回显，再退回带文字的页面。
const PLAIN_ENDPOINTS: [&str; 2] = ["http://ip.3322.net", "https://4.ipw.cn"];
const TEXT_ENDPOINT: &str = "https://myip.ipip.net";

/// 从纯回显响应里取 IPv4。整份 body 必须就是一个 IPv4，多余内容一律不接受——
/// 回显服务被劫持成一个门户页时，宽松匹配会捞出页面里随便一个数字串。
pub fn parse_plain(body: &str) -> Option<String> {
    let candidate = body.trim();
    is_ipv4(candidate).then(|| candidate.to_string())
}

/// 从 `myip.ipip.net` 那种带文字的响应里取 IPv4（形如「当前 IP：1.2.3.4  来自：...」）。
pub fn parse_text(body: &str) -> Option<String> {
    let marker = body.find("IP")?;
    let rest = &body[marker + "IP".len()..];
    let start = rest.find(|c: char| c.is_ascii_digit())?;
    let tail = &rest[start..];
    let end = tail
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(tail.len());
    let candidate = &tail[..end];
    is_ipv4(candidate).then(|| candidate.to_string())
}

fn is_ipv4(value: &str) -> bool {
    value.parse::<std::net::Ipv4Addr>().is_ok()
}

pub fn probe(agent: &ureq::Agent) -> Option<String> {
    for endpoint in PLAIN_ENDPOINTS {
        if let Some(ip) = http::get_text(agent, endpoint).and_then(|b| parse_plain(&b)) {
            return Some(ip);
        }
    }
    http::get_text(agent, TEXT_ENDPOINT).and_then(|b| parse_text(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_bare_ipv4_with_surrounding_whitespace() {
        assert_eq!(parse_plain(" 1.2.3.4\n").as_deref(), Some("1.2.3.4"));
    }

    #[test]
    fn rejects_a_body_that_merely_contains_an_ip() {
        // 回显端点被劫持成门户页时，宽松匹配会捞出页面里随便一个数字串。
        assert_eq!(parse_plain("<html>1.2.3.4</html>"), None);
        assert_eq!(parse_plain("your ip is 1.2.3.4"), None);
    }

    #[test]
    fn rejects_non_ipv4_bodies() {
        assert_eq!(parse_plain("2001:db8::1"), None);
        assert_eq!(parse_plain("999.1.1.1"), None);
        assert_eq!(parse_plain(""), None);
    }

    #[test]
    fn extracts_from_the_text_style_endpoint() {
        assert_eq!(
            parse_text("当前 IP：1.2.3.4  来自于：中国 上海").as_deref(),
            Some("1.2.3.4")
        );
        assert_eq!(
            parse_text("当前 IP: 203.0.113.9 来自").as_deref(),
            Some("203.0.113.9")
        );
    }

    #[test]
    fn text_parser_refuses_garbage() {
        assert_eq!(parse_text("当前 IP：未知"), None);
        assert_eq!(parse_text("no marker here"), None);
    }
}
