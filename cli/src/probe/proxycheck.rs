//! proxycheck.io v3 —— 同时服务 O1（归属）与 O4（风险）。
//!
//! **CLI 直连，不走本站 Worker**（ADR-0012）：Worker 的配额是网页版用户共享的，
//! CLI 的流量规模不可预测，会把它吃干。CLI 用用户自己的配额：无 key 100 次/天、
//! 配了免费 key 1000 次/天。
//!
//! 与 `worker/proxycheck.ts` 读同一批字段路径——判级契约里「风险分 ≥ 70」在两端
//! 必须指向同一把尺子。
//!
//! **CLI 侧额外吃 `location` 段**（Worker 不吃，它有 `request.cf`）。这带来一处
//! 两端差异，已登记在 docs/verdict.md 5.4。
//!
//! 第三方的行为细节（基准分表、配额、必带参数、已知坑）见 **docs/proxycheck.md**。

use serde::Deserialize;

use super::http;

/// 网络类型。`None` = 未知。
pub type NetworkType = Option<String>;

#[derive(Debug, Clone, PartialEq)]
pub struct Geo {
    pub country_name: Option<String>,
    pub country_code: Option<String>,
    pub region_name: Option<String>,
    pub city_name: Option<String>,
    pub timezone: Option<String>,
    pub asn: Option<String>,
    pub organisation: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Risk {
    pub network_type: NetworkType,
    pub proxy: bool,
    pub vpn: bool,
    pub tor: bool,
    pub scraper: bool,
    pub risk_score: u32,
    /// proxycheck 判定该 IP 当前正被用作匿名化地址。**不是「用户在用 VPN」**——
    /// 实测普通商业 VPN 出口是 false，TOR 出口是 true。判「高」的阈值由它决定（契约 3.1）。
    pub anonymous: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lookup {
    pub geo: Geo,
    pub risk: Risk,
}

/// 一次调用的结局。配额耗尽单独成一档——它不是故障，提示语也不同（可以配 key 解决）。
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Ok(Box<Lookup>),
    QuotaExhausted,
    Unavailable,
}

#[derive(Deserialize)]
struct Envelope {
    status: Option<String>,
    #[serde(flatten)]
    entries: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct Entry {
    #[serde(default)]
    network: Network,
    #[serde(default)]
    location: Location,
    #[serde(default)]
    detections: Detections,
}

#[derive(Deserialize, Default)]
struct Network {
    #[serde(rename = "type")]
    kind: Option<String>,
    asn: Option<String>,
    organisation: Option<String>,
    provider: Option<String>,
}

#[derive(Deserialize, Default)]
struct Location {
    country_name: Option<String>,
    country_code: Option<String>,
    region_name: Option<String>,
    city_name: Option<String>,
    timezone: Option<String>,
}

#[derive(Deserialize, Default)]
struct Detections {
    proxy: Option<bool>,
    vpn: Option<bool>,
    tor: Option<bool>,
    scraper: Option<bool>,
    risk: Option<u32>,
    anonymous: Option<bool>,
}

pub fn lookup(agent: &ureq::Agent, ip: &str, api_key: Option<&str>) -> Outcome {
    // `p=0`：机器可读的紧凑输出。
    // `tag=0`：**不把本次查询写进 proxycheck 的正向检出日志**——这是 ADR-0008 的隐私要求，
    // 不是可选优化。Worker 侧发的是同一组参数。
    let mut url = format!("https://proxycheck.io/v3/{ip}?p=0&tag=0");
    if let Some(key) = api_key {
        url.push_str("&key=");
        url.push_str(key);
    }

    match http::get_text(agent, &url) {
        Some(body) => parse(&body, ip),
        None => Outcome::Unavailable,
    }
}

/// 解析响应。**不能"200 就当成功"**：实测中 proxycheck 会以 HTTP 200 返回一份
/// 根本不是合法 JSON 的 body（字段类型名而非值的 schema 形状，间歇出现）。
/// 因此解析失败、`status != "ok"`、风险分不是数字，一律视为上游不可用。
///
/// 风险分缺失**绝不能默认成 0**——那会把有风险的 IP 静默报成低风险，
/// 正是契约 2.3 要防的那件事。
pub fn parse(body: &str, ip: &str) -> Outcome {
    let Ok(envelope) = serde_json::from_str::<Envelope>(body) else {
        return Outcome::Unavailable;
    };

    match envelope.status.as_deref() {
        Some("ok") => {}
        // proxycheck 用 denied 表达配额/权限拒绝。
        Some("denied") => return Outcome::QuotaExhausted,
        _ => return Outcome::Unavailable,
    }

    let Some(raw) = envelope.entries.get(ip) else {
        return Outcome::Unavailable;
    };
    let Ok(entry) = serde_json::from_value::<Entry>(raw.clone()) else {
        return Outcome::Unavailable;
    };

    // 风险分与 anonymous **必须成对**：阈值由后者决定（契约 3.1），只拿到一个判不了。
    // 缺 anonymous 时默认成 false 会静默抬高阈值、造成漏报——静默降级比响亮失败更难查。
    let (Some(risk_score), Some(anonymous)) = (entry.detections.risk, entry.detections.anonymous)
    else {
        return Outcome::Unavailable;
    };

    Outcome::Ok(Box::new(Lookup {
        geo: Geo {
            country_name: entry.location.country_name,
            country_code: entry.location.country_code,
            region_name: entry.location.region_name,
            city_name: entry.location.city_name,
            timezone: entry.location.timezone,
            asn: entry.network.asn,
            organisation: entry.network.organisation,
            provider: entry.network.provider,
        },
        risk: Risk {
            network_type: entry.network.kind,
            proxy: entry.detections.proxy == Some(true),
            vpn: entry.detections.vpn == Some(true),
            tor: entry.detections.tor == Some(true),
            scraper: entry.detections.scraper == Some(true),
            risk_score,
            anonymous,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 实测样本（2026-08-12，无 key）。
    const REAL: &str = r#"{
        "status": "ok",
        "1.1.1.1": {
            "network": { "asn": "AS13335", "range": "1.1.1.0/24", "hostname": "one.one.one.one",
                         "provider": "Cloudflare, Inc.", "organisation": "Cloudflare, Inc.", "type": "Hosting" },
            "location": { "continent_name": "Oceania", "continent_code": "OC",
                          "country_name": "Australia", "country_code": "AU",
                          "region_name": "New South Wales", "region_code": "NSW",
                          "city_name": "Sydney", "postal_code": "1001",
                          "latitude": -33.8688, "longitude": 151.209, "timezone": "Australia/Sydney" },
            "detections": { "proxy": false, "vpn": false, "tor": false, "scraper": false,
                            "hosting": true, "anonymous": false, "risk": 33, "confidence": 100 }
        },
        "query_time": 1
    }"#;

    #[test]
    fn parses_the_fields_o1_and_o4_need() {
        let Outcome::Ok(lookup) = parse(REAL, "1.1.1.1") else {
            panic!("应当解析成功");
        };
        assert_eq!(lookup.geo.asn.as_deref(), Some("AS13335"));
        assert_eq!(lookup.geo.organisation.as_deref(), Some("Cloudflare, Inc."));
        assert_eq!(lookup.geo.city_name.as_deref(), Some("Sydney"));
        assert_eq!(lookup.geo.timezone.as_deref(), Some("Australia/Sydney"));
        assert_eq!(lookup.risk.network_type.as_deref(), Some("Hosting"));
        assert_eq!(lookup.risk.risk_score, 33);
        assert!(!lookup.risk.proxy);
        assert!(!lookup.risk.anonymous);
    }

    #[test]
    fn schema_shaped_body_is_not_mistaken_for_data() {
        // 实测抓到的那份 HTTP 200 响应：字段类型名而非值，且根本不是合法 JSON。
        // 「200 就当成功」的客户端会在这里出事。
        let schema = "{\n  1.1.1.1:\n  {\n    detections:\n    {\n      risk: int\n    }\n  }\n  status: string\n}";
        assert_eq!(parse(schema, "1.1.1.1"), Outcome::Unavailable);
    }

    #[test]
    fn missing_risk_score_is_unavailable_not_zero() {
        // 默认成 0 会把有风险的 IP 静默报成低风险。
        let body = r#"{"status":"ok","1.1.1.1":{"network":{"type":"Hosting"},"detections":{"proxy":true}}}"#;
        assert_eq!(parse(body, "1.1.1.1"), Outcome::Unavailable);
    }

    #[test]
    fn missing_anonymous_is_unavailable_too() {
        // 契约 3.1 的阈值由 anonymous 决定。默认成 false 会把阈值静默抬到 76，
        // 于是本该判高的 IP 悄悄变成低——比直接报「上游不可用」难查得多。
        let body = r#"{"status":"ok","1.1.1.1":{"detections":{"risk":90}}}"#;
        assert_eq!(parse(body, "1.1.1.1"), Outcome::Unavailable);
    }

    #[test]
    fn non_ok_status_is_refused() {
        assert_eq!(
            parse(r#"{"status":"error","1.1.1.1":{}}"#, "1.1.1.1"),
            Outcome::Unavailable
        );
        assert_eq!(
            parse(r#"{"status":"denied","1.1.1.1":{}}"#, "1.1.1.1"),
            Outcome::QuotaExhausted
        );
    }

    #[test]
    fn entry_for_a_different_ip_is_not_accepted() {
        // 回显的 IP 与我们查的不是同一个，说明我们在读别人的结果。
        assert_eq!(parse(REAL, "8.8.8.8"), Outcome::Unavailable);
    }

    #[test]
    fn missing_geo_fields_degrade_to_unknown_rather_than_failing() {
        // 归属缺字段不该让整次查询失败——风险分才是 O4 的必需项。
        let body = r#"{"status":"ok","1.1.1.1":{"detections":{"risk":5,"anonymous":false}}}"#;
        let Outcome::Ok(lookup) = parse(body, "1.1.1.1") else {
            panic!("应当解析成功");
        };
        assert_eq!(lookup.geo.timezone, None);
        assert_eq!(lookup.risk.network_type, None);
        assert_eq!(lookup.risk.risk_score, 5);
    }
}
