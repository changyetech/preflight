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
use crate::domain::dns_servers;
use crate::domain::{dns_egress, udp_egress};
use crate::probe::{Report, TimezoneCheck, dns_check, ipify, proxy, proxycheck};

/// `preflight dns --json` 的独立 schema（spec §4.6）。不并入体检报告信封。
pub fn dns_servers(
    entries: &[dns_servers::Entry],
    results: Option<&[dns_check::CheckResult]>,
) -> Value {
    let servers: Vec<Value> = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let mut server = json!({
                "ip": entry.ip,
                "name": entry.name,
                "region": entry.region,
                "variant": entry.variant,
            });
            if let Some(results) = results {
                let r = &results[i];
                server["check"] = json!({
                    "reachable": r.status == dns_check::Status::Ok || r.status == dns_check::Status::Suspicious,
                    "latency_ms": r.latency.map(|d| d.as_millis() as u64),
                    "status": status_name(r.status),
                });
            }
            server
        })
        .collect();
    json!({ "servers": servers })
}

fn status_name(status: dns_check::Status) -> &'static str {
    match status {
        dns_check::Status::Ok => "ok",
        dns_check::Status::Suspicious => "suspicious",
        dns_check::Status::Unreachable => "unreachable",
    }
}

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

    // 按 ALL_CHECKS 迭代而不是写 10 个字面量键：新增检测项时这里编译不过，
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
            "dnsEgressLeak": signals.dns_egress_leak,
            "udpEgressMismatch": signals.udp_egress_mismatch,
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
                "geo": info.geo.as_ref().map(geo_json),
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
        CheckId::O5 => check(&report.o5, |result| {
            let (leak, ecs_country, exit_country) = match &result.comparison {
                dns_egress::Comparison::Comparable {
                    leak,
                    ecs_country,
                    exit_country,
                } => (
                    Some(*leak),
                    Some(ecs_country.clone()),
                    Some(exit_country.clone()),
                ),
                dns_egress::Comparison::NotComparable(_) => (None, None, None),
            };
            json!({
                // resolver 归属只展示、不参与判定（契约 2.1／2.5 硬约束 1）——只有 `leak` 进综合结论。
                "resolverGeo": result.resolver_geo,
                "leak": leak,
                "ecsCountry": ecs_country,
                "exitCountry": exit_country,
                // 三种「无从比对」成因分开报，null 表示可比对（契约 2.5 硬约束 3）。
                "notComparableReason": not_comparable_reason(&result.comparison),
            })
        }),
        CheckId::O6 => check(&report.o6, |result| {
            let (mismatch, reflexive_ip, exit_ip) = match result {
                udp_egress::UdpEgress::Comparable {
                    mismatch,
                    reflexive_ip,
                    exit_ip,
                } => (
                    Some(*mismatch),
                    Some(reflexive_ip.to_string()),
                    Some(exit_ip.to_string()),
                ),
                udp_egress::UdpEgress::NotComparable(_) => (None, None, None),
            };
            json!({
                "mismatch": mismatch,
                "reflexiveIp": reflexive_ip,
                "exitIp": exit_ip,
                // 三种「无从比对」成因分开报，null 表示可比对（契约 2.6）。
                "notComparableReason": udp_not_comparable_reason(result),
            })
        }),
        CheckId::C1 => check(&report.c1, |real| {
            json!({
                "ip": real.ip,
                // 与 O1 同源（契约 1）：同一个 proxycheck 地理库。
                "geoSource": "proxycheck",
                "geo": real.geo.as_ref().map(geo_json),
            })
        }),
        CheckId::C2 => check(&report.c2, |servers| {
            json!({
               "servers": servers.iter().map(|server| json!({
                   "address": server.address,
                    "provider": server.entry.map(|e| format!("{} ({})", e.name, e.region)),
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
    }
}

/// O1 与 C1 共用同一份归属形状——两处都来自 proxycheck 的同一段字段（契约 1）。
fn geo_json(geo: &proxycheck::Geo) -> Value {
    json!({
        "countryName": geo.country_name,
        "countryCode": geo.country_code,
        "regionName": geo.region_name,
        "cityName": geo.city_name,
        "timezone": geo.timezone,
        "asn": geo.asn,
        "organisation": geo.organisation,
        "provider": geo.provider,
    })
}

fn state_name(state: &proxy::State) -> &'static str {
    match state {
        proxy::State::Enabled => "on",
        proxy::State::Disabled => "off",
        // 「没检测」与「检测了、没开」必须分得开。
        proxy::State::Unsupported => "unsupported",
    }
}

fn not_comparable_reason(comparison: &dns_egress::Comparison) -> Option<&'static str> {
    match comparison {
        dns_egress::Comparison::Comparable { .. } => None,
        dns_egress::Comparison::NotComparable(reason) => Some(match reason {
            dns_egress::NotComparable::NoEcs => "noEcs",
            dns_egress::NotComparable::UnmappedCountry => "unmappedCountry",
            dns_egress::NotComparable::UnknownExitCountry => "unknownExitCountry",
        }),
    }
}

fn udp_not_comparable_reason(result: &udp_egress::UdpEgress) -> Option<&'static str> {
    match result {
        udp_egress::UdpEgress::Comparable { .. } => None,
        udp_egress::UdpEgress::NotComparable(reason) => Some(match reason {
            udp_egress::NotComparable::FamilyMismatch => "familyMismatch",
            udp_egress::NotComparable::UnknownExitIp => "unknownExitIp",
            udp_egress::NotComparable::StunDisagree => "stunDisagree",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::checks::Failure;
    use crate::probe::{ExitInfo, Risk, TimezoneCheck, dns, proxycheck, stopforumspam};

    fn blank() -> Report {
        Report {
            o1: Outcome::Failed(Failure::Upstream),
            o2: Outcome::Failed(Failure::Upstream),
            o3: Outcome::Failed(Failure::Upstream),
            o4: Outcome::Failed(Failure::QuotaExhausted),
            o5: Outcome::Failed(Failure::Upstream),
            o6: Outcome::Failed(Failure::Upstream),
            c1: Outcome::Failed(Failure::Upstream),
            c2: Outcome::Failed(Failure::Local),
            c3: Outcome::Failed(Failure::Local),
            c4: Outcome::Failed(Failure::Upstream),
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
    fn all_ten_checks_are_present_with_a_status() {
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
            "dnsEgressLeak",
            "udpEgressMismatch",
        ] {
            assert!(out["signals"][key].is_null(), "{key} 应为 null");
        }
    }

    #[test]
    fn dns_egress_leak_reports_both_compared_countries_and_the_resolver_geo() {
        // 契约 5.4：O5 必须把参与比对的两个国家都显示出来；resolver 归属只展示不参与判定。
        let mut r = blank();
        r.o5 = Outcome::Done(crate::domain::dns_egress::DnsEgress {
            resolver_geo: Some("Japan - Google LLC".into()),
            comparison: dns_egress::Comparison::Comparable {
                leak: true,
                ecs_country: "JP".into(),
                exit_country: "US".into(),
            },
        });
        let out = report(&r);
        assert_eq!(out["checks"]["O5"]["status"], "done");
        assert_eq!(out["checks"]["O5"]["leak"], true);
        assert_eq!(out["checks"]["O5"]["ecsCountry"], "JP");
        assert_eq!(out["checks"]["O5"]["exitCountry"], "US");
        assert_eq!(out["checks"]["O5"]["resolverGeo"], "Japan - Google LLC");
        assert!(out["checks"]["O5"]["notComparableReason"].is_null());
        assert_eq!(out["signals"]["dnsEgressLeak"], true);
    }

    #[test]
    fn dns_egress_not_comparable_reports_the_reason_and_a_null_leak() {
        // 「无从比对」是已完成，不是检测失败；三种成因分开报（契约 2.5 硬约束 3）。
        let mut r = blank();
        r.o5 = Outcome::Done(crate::domain::dns_egress::DnsEgress {
            resolver_geo: None,
            comparison: dns_egress::Comparison::NotComparable(
                crate::domain::dns_egress::NotComparable::NoEcs,
            ),
        });
        let out = report(&r);
        assert_eq!(out["checks"]["O5"]["status"], "done");
        assert!(out["checks"]["O5"]["leak"].is_null());
        assert!(out["checks"]["O5"]["ecsCountry"].is_null());
        assert!(out["checks"]["O5"]["exitCountry"].is_null());
        assert_eq!(out["checks"]["O5"]["notComparableReason"], "noEcs");
        assert!(out["signals"]["dnsEgressLeak"].is_null());
    }

    #[test]
    fn udp_egress_mismatch_reports_both_addresses() {
        let mut r = blank();
        r.o6 = Outcome::Done(udp_egress::UdpEgress::Comparable {
            mismatch: true,
            reflexive_ip: "203.0.113.7".parse().unwrap(),
            exit_ip: "198.51.100.20".parse().unwrap(),
        });
        let out = report(&r);
        assert_eq!(out["checks"]["O6"]["status"], "done");
        assert_eq!(out["checks"]["O6"]["mismatch"], true);
        assert_eq!(out["checks"]["O6"]["reflexiveIp"], "203.0.113.7");
        assert_eq!(out["checks"]["O6"]["exitIp"], "198.51.100.20");
        assert!(out["checks"]["O6"]["notComparableReason"].is_null());
        assert_eq!(out["signals"]["udpEgressMismatch"], true);
    }

    #[test]
    fn udp_egress_not_comparable_reports_the_reason_and_a_null_mismatch() {
        let mut r = blank();
        r.o6 = Outcome::Done(udp_egress::UdpEgress::NotComparable(
            udp_egress::NotComparable::StunDisagree,
        ));
        let out = report(&r);
        assert_eq!(out["checks"]["O6"]["status"], "done");
        assert!(out["checks"]["O6"]["mismatch"].is_null());
        assert!(out["checks"]["O6"]["reflexiveIp"].is_null());
        assert!(out["checks"]["O6"]["exitIp"].is_null());
        assert_eq!(out["checks"]["O6"]["notComparableReason"], "stunDisagree");
        assert!(out["signals"]["udpEgressMismatch"].is_null());
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

    // O5／O6 三种「无从比对」成因里，上面只覆盖了各一种（noEcs / stunDisagree）。
    // 补齐剩下的，免得某个成因的映射改错也没有测试能发现。

    #[test]
    fn dns_egress_not_comparable_unmapped_country_is_reported() {
        let mut r = blank();
        r.o5 = Outcome::Done(dns_egress::DnsEgress {
            resolver_geo: None,
            comparison: dns_egress::Comparison::NotComparable(
                dns_egress::NotComparable::UnmappedCountry,
            ),
        });
        let out = report(&r);
        assert_eq!(
            out["checks"]["O5"]["notComparableReason"],
            "unmappedCountry"
        );
    }

    #[test]
    fn dns_egress_not_comparable_unknown_exit_country_is_reported() {
        let mut r = blank();
        r.o5 = Outcome::Done(dns_egress::DnsEgress {
            resolver_geo: None,
            comparison: dns_egress::Comparison::NotComparable(
                dns_egress::NotComparable::UnknownExitCountry,
            ),
        });
        let out = report(&r);
        assert_eq!(
            out["checks"]["O5"]["notComparableReason"],
            "unknownExitCountry"
        );
    }

    #[test]
    fn udp_egress_not_comparable_family_mismatch_is_reported() {
        let mut r = blank();
        r.o6 = Outcome::Done(udp_egress::UdpEgress::NotComparable(
            udp_egress::NotComparable::FamilyMismatch,
        ));
        let out = report(&r);
        assert_eq!(out["checks"]["O6"]["notComparableReason"], "familyMismatch");
    }

    #[test]
    fn udp_egress_not_comparable_unknown_exit_ip_is_reported() {
        let mut r = blank();
        r.o6 = Outcome::Done(udp_egress::UdpEgress::NotComparable(
            udp_egress::NotComparable::UnknownExitIp,
        ));
        let out = report(&r);
        assert_eq!(out["checks"]["O6"]["notComparableReason"], "unknownExitIp");
    }

    // ------------------------------------------------------------------
    // 冻结基线：改版前 `--json` 输出的整份负载快照。
    //
    // 与上面的字段级测试不同，这里每条用例断言的是**整个 `report()` 返回值**，
    // 逐字段、逐检测项。任何一次渲染层改动（render.rs / copy/）如果不小心
    // 让某个字段漏进/漏出/改名/改类型，这里必须转红——这正是本组测试存在
    // 的唯一理由（task-C1）。
    //
    // 场景选择参照 docs/verdict-cases.json 里 `applies` 含 "cli" 的用例（该文件
    // 是判级契约 signals 层面的 golden 向量，没有携带 IP / geo / DNS 服务器等
    // 呈现层字段，因此不能直接当 fixture 用；这里按其覆盖的信号组合重新构造
    // 完整的 `Report`，覆盖度对照见 report.md）：
    //   - insufficient（全部失败）
    //   - preliminary·low（O4 配额耗尽，其余测完且干净）
    //   - full·low（10 项全测、全干净）
    //   - full·medium（IPv6 泄露）
    //   - full·high，anonymous=true 且 riskScore=51（契约 3.1 的「结论高但原始
    //     分数仍在 51–75」边界——JSON 里 riskScore 必须原样是 51，不能被判级
    //     阈值污染）
    //   - full·high，anonymous=false 且 riskScore=76（对照上一条的非匿名阈值）
    //   - full·medium，abuseListed 与 tunOff 同时命中

    fn clean_geo() -> proxycheck::Geo {
        proxycheck::Geo {
            country_name: Some("United States".into()),
            country_code: Some("US".into()),
            region_name: Some("California".into()),
            city_name: Some("Los Angeles".into()),
            timezone: Some("America/Los_Angeles".into()),
            asn: Some("AS15169".into()),
            organisation: Some("Example Org".into()),
            provider: Some("Example Cloud".into()),
        }
    }

    /// 10 项全测、全干净的基线报告：riskScore 10、anonymous false、无任何异常信号。
    /// 各场景在此基础上只改动与该场景相关的字段。
    fn full_clean() -> Report {
        Report {
            o1: Outcome::Done(ExitInfo {
                ip: "203.0.113.10".into(),
                geo: Some(clean_geo()),
            }),
            o2: Outcome::Done(TimezoneCheck {
                local: Some("America/Los_Angeles".into()),
                exit: Some("America/Los_Angeles".into()),
                matches: Some(true),
            }),
            o3: Outcome::Done(ipify::Ipv6::Disabled),
            o4: Outcome::Done(Risk {
                risk: proxycheck::Risk {
                    network_type: Some("Residential".into()),
                    proxy: false,
                    vpn: false,
                    tor: false,
                    scraper: false,
                    risk_score: 10,
                    anonymous: false,
                },
                abuse: Some(stopforumspam::Abuse {
                    listed: false,
                    frequency: 0,
                    last_seen: None,
                }),
            }),
            o5: Outcome::Done(dns_egress::DnsEgress {
                resolver_geo: Some("United States - Google LLC".into()),
                comparison: dns_egress::Comparison::Comparable {
                    leak: false,
                    ecs_country: "US".into(),
                    exit_country: "US".into(),
                },
            }),
            o6: Outcome::Done(udp_egress::UdpEgress::Comparable {
                mismatch: false,
                reflexive_ip: "203.0.113.10".parse().unwrap(),
                exit_ip: "203.0.113.10".parse().unwrap(),
            }),
            c1: Outcome::Done(crate::probe::RealIp {
                ip: "198.51.100.5".into(),
                geo: Some(clean_geo()),
            }),
            c2: Outcome::Done(vec![
                dns::Server {
                    address: "192.168.1.1".into(),
                    entry: None,
                    private: true,
                    domestic: true,
                },
                dns::Server {
                    address: "8.8.8.8".into(),
                    entry: crate::domain::dns_servers::lookup("8.8.8.8"),
                    private: false,
                    domestic: false,
                },
            ]),
            c3: Outcome::Done(proxy::Status {
                env_vars: Vec::new(),
                system: proxy::State::Disabled,
                system_kinds: Vec::new(),
                tun: proxy::State::Enabled,
            }),
            c4: Outcome::Done(TimezoneCheck {
                local: Some("America/Los_Angeles".into()),
                exit: Some("America/Los_Angeles".into()),
                matches: Some(true),
            }),
        }
    }

    fn clean_geo_json() -> Value {
        json!({
            "countryName": "United States",
            "countryCode": "US",
            "regionName": "California",
            "cityName": "Los Angeles",
            "timezone": "America/Los_Angeles",
            "asn": "AS15169",
            "organisation": "Example Org",
            "provider": "Example Cloud",
        })
    }

    /// `full_clean()` 对应的整份期望负载，其余快照用例以此为基础按需覆写字段。
    fn full_clean_json() -> Value {
        json!({
            "verdict": { "stage": "full", "level": "low" },
            "coverage": { "done": 10, "failed": 0, "total": TOTAL_CHECKS },
            "signals": {
                "tzMismatchCliEnv": false,
                "tzMismatchSystem": false,
                "ipv6Leak": false,
                "riskScore": 10,
                "anonymous": false,
                "abuseListed": false,
                "dnsEgressLeak": false,
                "udpEgressMismatch": false,
                "tunOff": false,
            },
            "checks": {
                "O1": {
                    "status": "done",
                    "ip": "203.0.113.10",
                    "geoSource": "proxycheck",
                    "geo": clean_geo_json(),
                },
                "O2": {
                    "status": "done",
                    "local": "America/Los_Angeles",
                    "exit": "America/Los_Angeles",
                    "match": true,
                },
                "O3": { "status": "done", "leak": false, "address": null },
                "O4": {
                    "status": "done",
                    "networkType": "Residential",
                    "proxy": false,
                    "vpn": false,
                    "tor": false,
                    "scraper": false,
                    "riskScore": 10,
                    "anonymous": false,
                    "abuseListed": false,
                    "abuseFrequency": 0,
                    "abuseLastSeen": null,
                },
                "O5": {
                    "status": "done",
                    "resolverGeo": "United States - Google LLC",
                    "leak": false,
                    "ecsCountry": "US",
                    "exitCountry": "US",
                    "notComparableReason": null,
                },
                "O6": {
                    "status": "done",
                    "mismatch": false,
                    "reflexiveIp": "203.0.113.10",
                    "exitIp": "203.0.113.10",
                    "notComparableReason": null,
                },
                "C1": {
                    "status": "done",
                    "ip": "198.51.100.5",
                    "geoSource": "proxycheck",
                    "geo": clean_geo_json(),
                },
                "C2": {
                    "status": "done",
                    "servers": [
                        {
                           "address": "192.168.1.1",
                            "provider": null,
                           "private": true,
                           "domestic": true,
                       },
                       {
                           "address": "8.8.8.8",
                            "provider": "Google Public DNS (US)",
                           "private": false,
                           "domestic": false,
                       },
                    ],
                    "domestic": true,
                },
                "C3": {
                    "status": "done",
                    "envVars": [],
                    "envProxy": "off",
                    "systemProxy": "off",
                    "systemProxyKinds": [],
                    "tun": "on",
                },
                "C4": {
                    "status": "done",
                    "local": "America/Los_Angeles",
                    "exit": "America/Los_Angeles",
                    "match": true,
                },
            },
        })
    }

    #[test]
    fn snapshot_insufficient_when_every_check_fails() {
        let out = report(&blank());
        let expected = json!({
            "verdict": { "stage": "insufficient", "level": null },
            "coverage": { "done": 0, "failed": TOTAL_CHECKS, "total": TOTAL_CHECKS },
            "signals": {
                "tzMismatchCliEnv": null,
                "tzMismatchSystem": null,
                "ipv6Leak": null,
                "riskScore": null,
                "anonymous": null,
                "abuseListed": null,
                "dnsEgressLeak": null,
                "udpEgressMismatch": null,
                "tunOff": null,
            },
            "checks": {
                "O1": { "status": "failed", "reason": "upstream" },
                "O2": { "status": "failed", "reason": "upstream" },
                "O3": { "status": "failed", "reason": "upstream" },
                "O4": { "status": "failed", "reason": "quotaExhausted" },
                "O5": { "status": "failed", "reason": "upstream" },
                "O6": { "status": "failed", "reason": "upstream" },
                "C1": { "status": "failed", "reason": "upstream" },
                "C2": { "status": "failed", "reason": "local" },
                "C3": { "status": "failed", "reason": "local" },
                "C4": { "status": "failed", "reason": "upstream" },
            },
        });
        assert_eq!(out, expected);
    }

    #[test]
    fn snapshot_preliminary_low_when_o4_quota_exhausted_but_the_rest_is_clean() {
        let mut r = full_clean();
        r.o4 = Outcome::Failed(Failure::QuotaExhausted);
        let out = report(&r);

        let mut expected = full_clean_json();
        expected["verdict"] = json!({ "stage": "preliminary", "level": "low" });
        expected["coverage"] = json!({ "done": 9, "failed": 1, "total": TOTAL_CHECKS });
        expected["signals"]["riskScore"] = Value::Null;
        expected["signals"]["anonymous"] = Value::Null;
        expected["signals"]["abuseListed"] = Value::Null;
        expected["checks"]["O4"] = json!({ "status": "failed", "reason": "quotaExhausted" });

        assert_eq!(out, expected);
    }

    #[test]
    fn snapshot_full_low_when_everything_is_clean() {
        let out = report(&full_clean());
        assert_eq!(out, full_clean_json());
    }

    #[test]
    fn snapshot_full_medium_when_ipv6_leaks() {
        let mut r = full_clean();
        r.o3 = Outcome::Done(ipify::Ipv6::Leaked("2001:db8::42".into()));
        let out = report(&r);

        let mut expected = full_clean_json();
        expected["verdict"] = json!({ "stage": "full", "level": "medium" });
        expected["signals"]["ipv6Leak"] = json!(true);
        expected["checks"]["O3"] = json!({
            "status": "done",
            "leak": true,
            "address": "2001:db8::42",
        });

        assert_eq!(out, expected);
    }

    #[test]
    fn snapshot_full_high_anonymous_at_51_keeps_the_raw_score_visible() {
        // 契约 3.1：anonymous=true 时综合结论从 51 起判高，但分项阈值仍是 76——
        // JSON 里的 riskScore 必须原样是 51，不能被综合结论「高」污染成 76 或某个等级字面量。
        let mut r = full_clean();
        r.o4 = Outcome::Done(Risk {
            risk: proxycheck::Risk {
                network_type: Some("Residential".into()),
                proxy: false,
                vpn: false,
                tor: true,
                scraper: false,
                risk_score: 51,
                anonymous: true,
            },
            abuse: Some(stopforumspam::Abuse {
                listed: false,
                frequency: 0,
                last_seen: None,
            }),
        });
        let out = report(&r);

        let mut expected = full_clean_json();
        expected["verdict"] = json!({ "stage": "full", "level": "high" });
        expected["signals"]["riskScore"] = json!(51);
        expected["signals"]["anonymous"] = json!(true);
        expected["checks"]["O4"] = json!({
            "status": "done",
            "networkType": "Residential",
            "proxy": false,
            "vpn": false,
            "tor": true,
            "scraper": false,
            "riskScore": 51,
            "anonymous": true,
            "abuseListed": false,
            "abuseFrequency": 0,
            "abuseLastSeen": null,
        });

        assert_eq!(out, expected);
    }

    #[test]
    fn snapshot_full_high_not_anonymous_at_76() {
        let mut r = full_clean();
        r.o4 = Outcome::Done(Risk {
            risk: proxycheck::Risk {
                network_type: Some("Residential".into()),
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: 76,
                anonymous: false,
            },
            abuse: Some(stopforumspam::Abuse {
                listed: false,
                frequency: 0,
                last_seen: None,
            }),
        });
        let out = report(&r);

        let mut expected = full_clean_json();
        expected["verdict"] = json!({ "stage": "full", "level": "high" });
        expected["signals"]["riskScore"] = json!(76);
        expected["checks"]["O4"] = json!({
            "status": "done",
            "networkType": "Residential",
            "proxy": false,
            "vpn": false,
            "tor": false,
            "scraper": false,
            "riskScore": 76,
            "anonymous": false,
            "abuseListed": false,
            "abuseFrequency": 0,
            "abuseLastSeen": null,
        });

        assert_eq!(out, expected);
    }

    #[test]
    fn snapshot_full_medium_when_abuse_listed_and_tun_off_both_fire() {
        let mut r = full_clean();
        r.o4 = Outcome::Done(Risk {
            risk: proxycheck::Risk {
                network_type: Some("Residential".into()),
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: 10,
                anonymous: false,
            },
            abuse: Some(stopforumspam::Abuse {
                listed: true,
                frequency: 5,
                last_seen: Some("2026-01-01".into()),
            }),
        });
        // 存在代理迹象（环境变量代理）且 TUN 明确未开启 ⇒ tunOff = Some(true)（契约 2.7）。
        r.c3 = Outcome::Done(proxy::Status {
            env_vars: vec!["HTTPS_PROXY".into()],
            system: proxy::State::Disabled,
            system_kinds: Vec::new(),
            tun: proxy::State::Disabled,
        });
        let out = report(&r);

        let mut expected = full_clean_json();
        expected["verdict"] = json!({ "stage": "full", "level": "medium" });
        expected["signals"]["abuseListed"] = json!(true);
        expected["signals"]["tunOff"] = json!(true);
        expected["checks"]["O4"] = json!({
            "status": "done",
            "networkType": "Residential",
            "proxy": false,
            "vpn": false,
            "tor": false,
            "scraper": false,
            "riskScore": 10,
            "anonymous": false,
            "abuseListed": true,
            "abuseFrequency": 5,
            "abuseLastSeen": "2026-01-01",
        });
        expected["checks"]["C3"] = json!({
            "status": "done",
            "envVars": ["HTTPS_PROXY"],
            "envProxy": "on",
            "systemProxy": "off",
            "systemProxyKinds": [],
            "tun": "off",
        });

        assert_eq!(out, expected);
    }

    #[test]
    fn dns_json_without_check_has_no_check_key() {
        let out = dns_servers(crate::domain::dns_servers::all(), None);
        let servers = out["servers"].as_array().unwrap();
        assert!(servers.len() == 27);
        // 不带 --check 时，每个条目都没有 "check" 键。
        for s in servers {
            assert!(s.get("check").is_none(), "不应有 check 键");
            assert!(s.get("ip").is_some());
            assert!(s.get("variant").is_some());
        }
    }

    #[test]
    fn dns_json_with_check_has_status_and_latency() {
        let entries = crate::domain::dns_servers::all();
        let results: Vec<dns_check::CheckResult> = entries
            .iter()
            .enumerate()
            .map(|(i, _)| dns_check::CheckResult {
                status: if i % 3 == 0 {
                    dns_check::Status::Ok
                } else if i % 3 == 1 {
                    dns_check::Status::Suspicious
                } else {
                    dns_check::Status::Unreachable
                },
                latency: if i % 3 == 0 {
                    Some(std::time::Duration::from_millis(12 * (i as u64 + 1)))
                } else {
                    None
                },
            })
            .collect();
        let out = dns_servers(entries, Some(&results));
        let servers = out["servers"].as_array().unwrap();
        let first = &servers[0];
        assert_eq!(first["check"]["status"], "ok");
        assert!(first["check"]["latency_ms"].as_u64().is_some());
        assert_eq!(first["check"]["reachable"], true);

        let second = &servers[1];
        assert_eq!(second["check"]["status"], "suspicious");
        assert_eq!(second["check"]["reachable"], true);

        let third = &servers[2];
        assert_eq!(third["check"]["status"], "unreachable");
        assert_eq!(third["check"]["reachable"], false);
    }
}
