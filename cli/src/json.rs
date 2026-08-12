//! `--json` 输出。
//!
//! **不套 `{code, message, data}` 信封**——那是 `docs/api.md` 的 HTTP 契约，CLI 不是 HTTP。
//! 字段名沿用判级契约的信号名（camelCase，与 Web 对齐）。
//!
//! 这份输出同时是**与 `ai-ipcheck` 做平价比对时唯一的机械化 oracle**：呈现层重设计后
//! 逐行比对不再可能，只能比数据与结论。
//!
//! proxycheck key 在结构上就进不来——`Report` 里根本没有它。

use serde_json::{Value, json};

use crate::domain::checks::{ALL_CHECKS, CheckId, Failure, Outcome, TOTAL_CHECKS};
use crate::probe::{Report, TimezoneCheck, claude, ipify, proxy};

fn failure_name(failure: Failure) -> &'static str {
    match failure {
        Failure::Upstream => "upstream",
        Failure::QuotaExhausted => "quotaExhausted",
        Failure::Local => "local",
    }
}

/// 失败的检测项统一形状：`{"status":"failed","reason":"..."}`。
fn failed(failure: Failure) -> Value {
    json!({ "status": "failed", "reason": failure_name(failure) })
}

fn check<T>(outcome: &Outcome<T>, done: impl FnOnce(&T) -> Value) -> Value {
    match outcome {
        Outcome::Done(value) => {
            let mut body = done(value);
            if let Some(map) = body.as_object_mut() {
                map.insert("status".into(), json!("done"));
            }
            body
        }
        Outcome::Failed(failure) => failed(*failure),
    }
}

fn timezone(check: &TimezoneCheck) -> Value {
    json!({
        "local": check.local,
        "exit": check.exit,
        // 三态：true / false / null（无从比对）。null 不是 false。
        "match": check.matches,
    })
}

pub fn report(report: &Report) -> Value {
    let verdict = report.verdict();
    let coverage = report.coverage();
    let signals = report.signals();

    // 按 ALL_CHECKS 迭代而不是写 9 个字面量键：新增检测项时这里编译不过，
    // 而不是悄悄少一个键。
    let mut checks = serde_json::Map::new();
    for id in ALL_CHECKS {
        checks.insert(id.as_str().to_string(), check_json(id, report));
    }

    json!({
        "verdict": {
            "stage": verdict.stage(),
            // insufficient 没有档位——这里给 null，与「低」明确区分。
            "level": verdict.level(),
        },
        "coverage": {
            "done": coverage.done,
            "failed": coverage.failed,
            "total": TOTAL_CHECKS,
        },
        "signals": {
            "tzMismatchCliEnv": signals.tz_mismatch_cli_env,
            "tzMismatchSystem": signals.tz_mismatch_system,
            "ipv6Leak": signals.ipv6_leak,
            "riskScore": signals.risk_score,
            "anonymous": signals.anonymous,
            "abuseListed": signals.abuse_listed,
            "tunOff": signals.tun_off,
        },
        "checks": Value::Object(checks),
    })
}

fn check_json(id: CheckId, report: &Report) -> Value {
    match id {
        CheckId::O1 => check(&report.o1, |info| {
            json!({
                "ip": info.ip,
                // 归属来自 proxycheck，不是 Cloudflare（契约 5.4）。
                "geoSource": "proxycheck",
                "geo": info.geo.as_ref().map(|geo| json!({
                    "countryName": geo.country_name,
                    "countryCode": geo.country_code,
                    "regionName": geo.region_name,
                    "cityName": geo.city_name,
                    "timezone": geo.timezone,
                    "asn": geo.asn,
                    "organisation": geo.organisation,
                    "provider": geo.provider,
                })),
            })
        }),
        CheckId::O2 => check(&report.o2, timezone),
        CheckId::O3 => check(&report.o3, |result| match result {
            ipify::Ipv6::Leaked(addr) => json!({ "leak": true, "address": addr }),
            ipify::Ipv6::Disabled => json!({ "leak": false, "address": Value::Null }),
            ipify::Ipv6::Indeterminate => json!({ "leak": Value::Null, "address": Value::Null }),
        }),
        CheckId::O4 => check(&report.o4, |risk| {
            json!({
                "networkType": risk.risk.network_type,
                "proxy": risk.risk.proxy,
                "vpn": risk.risk.vpn,
                "tor": risk.risk.tor,
                "scraper": risk.risk.scraper,
                "riskScore": risk.risk.risk_score,
                "anonymous": risk.risk.anonymous,
                // 未知是 null，不是 false（契约 2.3）。
                "abuseListed": risk.abuse.as_ref().map(|a| a.listed),
                "abuseFrequency": risk.abuse.as_ref().map(|a| a.frequency),
                "abuseLastSeen": risk.abuse.as_ref().and_then(|a| a.last_seen.clone()),
            })
        }),
        CheckId::C1 => check(&report.c1, |ip| json!({ "ip": ip })),
        CheckId::C2 => check(&report.c2, |servers| {
            json!({
                "servers": servers.iter().map(|server| json!({
                    "address": server.address,
                    "provider": server.label,
                    "private": server.private,
                    "domestic": server.domestic,
                })).collect::<Vec<_>>(),
                "domestic": servers.iter().any(|s| s.domestic),
            })
        }),
        CheckId::C3 => check(&report.c3, |status| {
            json!({
                // 只有开关状态，没有地址——与人读输出同一条红线。
                "envVars": status.env_vars,
                "envProxy": state_name(&status.env_state()),
                "systemProxy": state_name(&status.system),
                "systemProxyKinds": status.system_kinds,
                "tun": state_name(&status.tun),
            })
        }),
        CheckId::C4 => check(&report.c4, timezone),
        CheckId::C5 => check(&report.c5, |detection| {
            json!({
                "endpoint": endpoint_json(&detection.endpoint),
                "source": detection.source,
            })
        }),
    }
}

fn state_name(state: &proxy::State) -> &'static str {
    match state {
        proxy::State::Enabled => "on",
        proxy::State::Disabled => "off",
        // 「没检测」与「检测了、没开」必须分得开。
        proxy::State::Unsupported => "unsupported",
    }
}

fn endpoint_json(endpoint: &claude::Endpoint) -> Value {
    match endpoint {
        claude::Endpoint::NotInstalled => json!({ "kind": "notInstalled" }),
        claude::Endpoint::Official => json!({ "kind": "official" }),
        claude::Endpoint::Domestic { host } => json!({ "kind": "domestic", "host": host }),
        claude::Endpoint::Relay { host, blacklisted } => json!({
            "kind": "relay",
            "host": host,
            // 命中只告警，不进综合结论（ADR-0010）——所以它在 checks 里而不在 signals 里。
            "blacklisted": blacklisted,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::checks::Failure;
    use crate::probe::{ExitInfo, Risk, TimezoneCheck, proxycheck, stopforumspam};

    fn blank() -> Report {
        Report {
            o1: Outcome::Failed(Failure::Upstream),
            o2: Outcome::Failed(Failure::Upstream),
            o3: Outcome::Failed(Failure::Upstream),
            o4: Outcome::Failed(Failure::QuotaExhausted),
            c1: Outcome::Failed(Failure::Upstream),
            c2: Outcome::Failed(Failure::Local),
            c3: Outcome::Failed(Failure::Local),
            c4: Outcome::Failed(Failure::Upstream),
            c5: Outcome::Failed(Failure::Local),
        }
    }

    #[test]
    fn there_is_no_http_envelope() {
        // CLI 不是 HTTP，套 {code,message,data} 是照搬。
        let out = report(&blank());
        assert!(out.get("code").is_none());
        assert!(out.get("message").is_none());
        assert!(out.get("data").is_none());
        assert!(out.get("verdict").is_some());
    }

    #[test]
    fn insufficient_verdict_reports_a_null_level_not_low() {
        let out = report(&blank());
        assert_eq!(out["verdict"]["stage"], "insufficient");
        assert!(out["verdict"]["level"].is_null());
    }

    #[test]
    fn all_nine_checks_are_present_with_a_status() {
        let out = report(&blank());
        // 用 ALL_CHECKS 迭代而不是写死字面量：JSON 的键因此被钉在枚举上，
        // 新增检测项时这条会红。
        for id in ALL_CHECKS {
            assert!(
                out["checks"][id.as_str()]["status"].is_string(),
                "缺少 {id}"
            );
        }
        assert_eq!(out["checks"]["O4"]["reason"], "quotaExhausted");
    }

    #[test]
    fn coverage_invariant_is_visible_in_the_payload() {
        let out = report(&blank());
        let done = out["coverage"]["done"].as_u64().unwrap();
        let failed = out["coverage"]["failed"].as_u64().unwrap();
        assert_eq!(done + failed, out["coverage"]["total"].as_u64().unwrap());
    }

    #[test]
    fn unknown_signals_serialise_as_null_never_false() {
        let out = report(&blank());
        for key in [
            "tzMismatchCliEnv",
            "ipv6Leak",
            "riskScore",
            "anonymous",
            "abuseListed",
        ] {
            assert!(out["signals"][key].is_null(), "{key} 应为 null");
        }
    }

    #[test]
    fn abuse_unknown_is_null_while_risk_score_is_present() {
        let mut r = blank();
        r.o4 = Outcome::Done(Risk {
            risk: proxycheck::Risk {
                network_type: Some("Hosting".into()),
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: 33,
                anonymous: false,
            },
            abuse: None,
        });
        let out = report(&r);
        assert_eq!(out["checks"]["O4"]["riskScore"], 33);
        assert!(out["checks"]["O4"]["abuseListed"].is_null());
        assert_eq!(out["signals"]["riskScore"], 33);
    }

    #[test]
    fn indeterminate_timezone_serialises_as_null_match() {
        let mut r = blank();
        r.c4 = Outcome::Done(TimezoneCheck {
            local: Some("Totally/Bogus".into()),
            exit: Some("Asia/Shanghai".into()),
            matches: None,
        });
        let out = report(&r);
        assert_eq!(out["checks"]["C4"]["status"], "done");
        assert!(out["checks"]["C4"]["match"].is_null());
        assert!(out["signals"]["tzMismatchCliEnv"].is_null());
    }

    #[test]
    fn the_payload_never_contains_a_proxycheck_key() {
        // key 在结构上就进不来——Report 里没有它。这条测试守着这个性质。
        let mut r = blank();
        r.o1 = Outcome::Done(ExitInfo {
            ip: "203.0.113.7".into(),
            geo: None,
        });
        r.o4 = Outcome::Done(Risk {
            risk: proxycheck::Risk {
                network_type: None,
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: 1,
                anonymous: false,
            },
            abuse: Some(stopforumspam::Abuse {
                listed: false,
                frequency: 0,
                last_seen: None,
            }),
        });
        let rendered = report(&r).to_string().to_lowercase();
        assert!(!rendered.contains("key"), "{rendered}");
        assert!(!rendered.contains("secret"));
    }
}
