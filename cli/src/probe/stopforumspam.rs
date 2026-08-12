//! StopForumSpam —— O4 的滥用收录信号。
//!
//! 它不可用时 `abuse_listed` 为**未知**，而 O4 仍算已完成：SFS 只贡献「中」，
//! 它挂掉不该连累 proxycheck 的结果（契约 2.3）。

use serde::Deserialize;

use super::http;

#[derive(Debug, Clone, PartialEq)]
pub struct Abuse {
    pub listed: bool,
    /// 举报次数与最近举报日期，仅用于呈现，不参与判级。
    pub frequency: u32,
    pub last_seen: Option<String>,
}

#[derive(Deserialize)]
struct Envelope {
    ip: Option<Record>,
}

#[derive(Deserialize)]
struct Record {
    /// SFS 用 0/1 或 true/false 表达，这里两种都收。
    appears: Option<serde_json::Value>,
    #[serde(default)]
    frequency: Option<serde_json::Value>,
    lastseen: Option<String>,
}

/// 返回 `None` 表示**未知**（第三方不可用或响应不合法）——绝不能塌缩成「无收录」。
pub fn parse(body: &str) -> Option<Abuse> {
    let envelope: Envelope = serde_json::from_str(body).ok()?;
    let record = envelope.ip?;
    let appears = truthy(record.appears.as_ref())?;

    Some(Abuse {
        listed: appears,
        frequency: record
            .frequency
            .as_ref()
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        last_seen: record.lastseen.filter(|s| !s.is_empty()).map(|s| {
            // 只留日期部分，时分秒对用户没有意义。
            s.chars().take(10).collect()
        }),
    })
}

fn truthy(value: Option<&serde_json::Value>) -> Option<bool> {
    match value? {
        serde_json::Value::Bool(b) => Some(*b),
        serde_json::Value::Number(n) => Some(n.as_u64()? != 0),
        _ => None,
    }
}

pub fn probe(agent: &ureq::Agent, ip: &str) -> Option<Abuse> {
    let url = format!("https://api.stopforumspam.org/api?json=1&ip={ip}");
    let body = http::get_text(agent, &url)?;
    parse(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_listed_ip() {
        let body = r#"{"success":1,"ip":{"value":"1.2.3.4","appears":1,"frequency":42,"lastseen":"2026-07-01 12:00:00","confidence":88.2}}"#;
        let abuse = parse(body).expect("应当解析成功");
        assert!(abuse.listed);
        assert_eq!(abuse.frequency, 42);
        assert_eq!(abuse.last_seen.as_deref(), Some("2026-07-01"));
    }

    #[test]
    fn parses_a_clean_ip() {
        let body = r#"{"success":1,"ip":{"value":"1.2.3.4","appears":0,"frequency":0}}"#;
        let abuse = parse(body).expect("应当解析成功");
        assert!(!abuse.listed);
        assert_eq!(abuse.last_seen, None);
    }

    #[test]
    fn accepts_boolean_appears_too() {
        let body = r#"{"ip":{"appears":true,"frequency":3}}"#;
        assert!(parse(body).unwrap().listed);
    }

    #[test]
    fn unusable_response_is_unknown_not_clean() {
        // 「查不到」绝不能塌缩成「无收录」——那是拿第三方故障冒充安全。
        assert_eq!(parse("not json"), None);
        assert_eq!(parse(r#"{"success":0}"#), None);
        assert_eq!(parse(r#"{"ip":{"frequency":0}}"#), None);
    }
}
