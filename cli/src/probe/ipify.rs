//! ipify —— 出口 IPv4（O1 的地址部分）与 IPv6 泄露判定（O3）。
//!
//! 出口 IP **不取自 proxycheck**：那样一来配额耗尽会连出口 IP 都显示不出来，
//! 而出口 IP 是结论区必须呈现的东西。ipify 免费无限、无配额。

use super::http;

const V4_ENDPOINT: &str = "https://api.ipify.org?format=json";
const V6_ENDPOINT: &str = "https://api6.ipify.org?format=json";

/// O3 的判定结果。
#[derive(Debug, Clone, PartialEq)]
pub enum Ipv6 {
    /// v6 端点可达 ⇒ IPv6 泄露，附上暴露的地址。
    Leaked(String),
    /// v4 通、v6 不通 ⇒ 未启用 IPv6。
    Disabled,
    /// v4 就不通 ⇒ 分不清是"没有 IPv6"还是"探测服务挂了"，只能判检测失败。
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Exposure {
    /// 出口 IPv4。`None` 表示 v4 端点不可达。
    pub ipv4: Option<String>,
    pub ipv6: Ipv6,
}

fn extract_ip(body: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Payload {
        ip: String,
    }
    let payload: Payload = serde_json::from_str(body).ok()?;
    let ip = payload.ip.trim();
    (!ip.is_empty()).then(|| ip.to_string())
}

/// 由两个端点的结果推出 O3 判定。**必须有 v4 对照**：
/// 没有它就无法区分「用户没有 IPv6」与「ipify 挂了」（ADR-0003）。
pub fn classify(v4: Option<&str>, v6: Option<&str>) -> Ipv6 {
    match (v4, v6) {
        (Some(_), Some(addr)) => Ipv6::Leaked(addr.to_string()),
        (Some(_), None) => Ipv6::Disabled,
        // v4 不通：无论 v6 通不通都判不出来。v4 不通而 v6 通更是异常，同样按检测失败处理。
        (None, _) => Ipv6::Indeterminate,
    }
}

pub fn probe(agent: &ureq::Agent) -> Exposure {
    let v4 = http::get_text(agent, V4_ENDPOINT).and_then(|b| extract_ip(&b));
    let v6 = http::get_text(agent, V6_ENDPOINT).and_then(|b| extract_ip(&b));

    Exposure {
        ipv6: classify(v4.as_deref(), v6.as_deref()),
        ipv4: v4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_ipify_payload() {
        assert_eq!(
            extract_ip(r#"{"ip":"203.0.113.7"}"#).as_deref(),
            Some("203.0.113.7")
        );
        assert_eq!(extract_ip(r#"{"ip":""}"#), None);
        assert_eq!(extract_ip("not json"), None);
    }

    #[test]
    fn both_reachable_means_ipv6_leak() {
        assert_eq!(
            classify(Some("203.0.113.7"), Some("2001:db8::1")),
            Ipv6::Leaked("2001:db8::1".into())
        );
    }

    #[test]
    fn v4_only_means_ipv6_disabled() {
        assert_eq!(classify(Some("203.0.113.7"), None), Ipv6::Disabled);
    }

    #[test]
    fn v4_unreachable_is_indeterminate_never_disabled() {
        // 判成「未启用 IPv6」就是拿第三方故障冒充「没问题」。
        assert_eq!(classify(None, None), Ipv6::Indeterminate);
        assert_eq!(classify(None, Some("2001:db8::1")), Ipv6::Indeterminate);
    }
}
