//! O5 的探测层：解析一个每次都重新生成的唯一子域，读回 ip-api 的 ECS 观测。
//!
//! 判定逻辑不在这里——它在 `domain/dns_egress.rs`，输入只是两个原样的 geo 字符串。
//! 这里刻意不切国家名、不查表：换 provider 时只需换这个文件。
//!
//! 走系统 resolver（命令行进程实际用的那条解析路径），与 Web 侧测的浏览器 DoH 路径
//! 不是同一个东西，两端因此可能得出相反的结论（契约 5.5）。

use crate::domain::dns_egress::Observation;

use super::http;

/// 通配符子域。`https://ip-api.com/json/` 免费版返回 403，只有这个子域支持 HTTPS。
fn endpoint(label: &str) -> String {
    format!("https://{label}.edns.ip-api.com/json")
}

/// 首次 + 最多重试 2 次。
const ATTEMPTS: usize = 3;

/// 唯一子域是为了绕开各级 DNS 缓存——命中缓存拿到的是**别人**的观测值。
/// 因此每次重试都必须换新前缀，重试同一个前缀等于在打自己刚种下的缓存。
///
/// **长度不是随便定的**：ip-api 只接受恰好 32 个 `[a-z0-9]` 字符的标签，别的长度一律 404
/// （实测 2026-08-13：31 位与 33 位都是 404，32 位的十六进制串是 200；直接请求
/// `https://edns.ip-api.com/json` 也只是 302 到它自己生成的一个 32 位标签）。
/// 写错长度的后果是 O5 **永远**落检测失败，而那看起来只是「第三方不稳定」。
fn random_label() -> String {
    format!("{:016x}{:016x}", super::random_u64(), super::random_u64())
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|geo| !geo.trim().is_empty())
}

/// `None` = 响应不是我们要的那个响应（不是合法 JSON，或形状不对）。
///
/// **`edns` 段缺失不算失败**——那是判定表里的一行正常输入（「响应中无 ECS 段」），
/// 由判定层记「已完成 · 无从比对」。
///
/// **但 `dns` 段缺失算失败**：真实响应里它恒存在，缺了说明对方换了 schema 或返回了
/// 一个 JSON 错误体（父计划把「ip-api 的 HTTPS 口子随时可能被收紧」列为已知风险）。
/// 那时若把 `{}` 当成「无 ECS 段」，呈现层会打出「你的 DNS 服务商不发送 ECS」——
/// 一句**假的**解释，比响亮失败难查得多。
pub fn parse(body: &str) -> Option<Observation> {
    #[derive(serde::Deserialize)]
    struct Section {
        geo: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct Payload {
        dns: Option<Section>,
        edns: Option<Section>,
    }

    let payload: Payload = serde_json::from_str(body).ok()?;
    let resolver = payload.dns?;

    Some(Observation {
        ecs_geo: non_empty(payload.edns.and_then(|section| section.geo)),
        resolver_geo: non_empty(resolver.geo),
    })
}

/// `None` = 探测失败（网络错误、响应不可解析），对应判定表最后一行的「O5 检测失败」。
pub fn probe(agent: &ureq::Agent) -> Option<Observation> {
    (0..ATTEMPTS).find_map(|_| {
        http::get_text(agent, &endpoint(&random_label())).and_then(|body| parse(&body))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_geo_strings_verbatim() {
        // 第 0 步实测的响应形状。判定层要的就是这两个原样字符串。
        let body = r#"{"dns":{"ip":"172.253.198.221","geo":"Japan - Google LLC"},
                       "edns":{"ip":"212.50.249.0","geo":"Japan - IT7 Networks Inc"}}"#;
        assert_eq!(
            parse(body),
            Some(Observation {
                ecs_geo: Some("Japan - IT7 Networks Inc".into()),
                resolver_geo: Some("Japan - Google LLC".into()),
            })
        );
    }

    #[test]
    fn a_missing_edns_section_is_a_valid_observation_not_a_failure() {
        // 不发 ECS 的服务商（Cloudflare 1.1.1.1 是明确的一家）走这条路。
        // 判成探测失败会诱导用户反复刷新一个永远不会变的结果（契约 2.5）。
        let observation = parse(r#"{"dns":{"geo":"United States - Cloudflare"}}"#);
        assert_eq!(
            observation,
            Some(Observation {
                ecs_geo: None,
                resolver_geo: Some("United States - Cloudflare".into()),
            })
        );
    }

    #[test]
    fn empty_strings_are_normalised_to_none() {
        assert_eq!(
            parse(r#"{"dns":{"geo":""},"edns":{"geo":"  "}}"#),
            Some(Observation {
                ecs_geo: None,
                resolver_geo: None,
            })
        );
    }

    #[test]
    fn an_unparsable_body_is_a_probe_failure() {
        // ip-api 的 HTTPS 口子随时可能被收紧，那时拿到的多半是一页 HTML。
        assert_eq!(parse("<html>403</html>"), None);
    }

    #[test]
    fn a_json_body_of_the_wrong_shape_fails_loudly_not_as_a_missing_ecs() {
        // 合法 JSON 但没有 `dns` 段 ⇒ 探测失败。判成「无 ECS 段」的话，
        // 呈现层会打出「你的 DNS 服务商不发送 ECS」——一句假的解释，
        // 比响亮失败难查得多，而且用户永远查不出真正的原因。
        assert_eq!(parse("{}"), None);
        assert_eq!(parse(r#"{"status":"fail","message":"quota"}"#), None);
        assert_eq!(parse(r#"{"edns":{"geo":"Japan - Foo"}}"#), None);
    }

    #[test]
    fn every_attempt_uses_a_fresh_label() {
        // 重试同一个前缀等于在打自己刚种下的缓存，拿回来的是上一次的观测值。
        assert_ne!(random_label(), random_label());
        // 32 位以外一律 404，整项探测会永远失败——这条钉住那个长度。
        let label = random_label();
        assert_eq!(label.len(), 32);
        assert!(
            label
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
        assert!(endpoint("abc").starts_with("https://abc.edns.ip-api.com/"));
    }
}
