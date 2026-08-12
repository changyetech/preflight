//! 呈现层：把一次体检渲染成人读的报告。
//!
//! 参考 ipcheck Web 的视觉结构，**不用表格边框**：
//! 结论区置顶（用户敲完命令第一眼看到结论，不用滚），其下是 10 张检测卡。
//! 网页的顶部导航与落地内容不移植——终端没有锚点，营销文案对已装用户没有意义。
//!
//! 没有固定列宽常量：`ai-ipcheck` 的 `COL_LABEL = 20` 是「多语言不存在」这个假设的化石。

use std::fmt::Write as _;

use crate::copy::Text;
use crate::domain::checks::{CheckId, Coverage, Failure, Outcome};
use crate::domain::verdict::{self, Level, PreliminaryLevel, Verdict};
use crate::domain::{dns_egress, udp_egress};
use crate::probe::{ExitInfo, Report, Risk, TimezoneCheck, dns, ipify, proxy};

/// 分项的颜色语义。与综合结论无关（契约 6）。
#[derive(Clone, Copy, PartialEq)]
enum Tone {
    Ok,
    Warn,
    Bad,
    Dim,
}

pub struct Style {
    color: bool,
}

impl Style {
    pub fn new(color: bool) -> Self {
        Self { color }
    }

    fn paint(&self, code: &str, body: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{body}\x1b[0m")
        } else {
            body.to_string()
        }
    }

    fn tone(&self, tone: Tone, body: &str) -> String {
        match tone {
            Tone::Ok => self.paint("32", body),
            Tone::Warn => self.paint("33", body),
            Tone::Bad => self.paint("31", body),
            Tone::Dim => self.paint("2", body),
        }
    }

    fn bold(&self, body: &str) -> String {
        self.paint("1", body)
    }
}

/// 状态符号。没有右边框之后，歧义宽度的符号不再破坏对齐——
/// `ai-ipcheck` 那条「禁用 ✓✗⚠」的禁令是为边框立的，这里不适用。
fn marker(tone: Tone) -> &'static str {
    match tone {
        Tone::Ok => "✔",
        Tone::Warn => "!",
        Tone::Bad => "✖",
        Tone::Dim => "·",
    }
}

fn verdict_tone(verdict: &Verdict) -> Tone {
    match verdict {
        // 数据不足是中性的：既不是绿也不是黄——它与「低」的区别必须同时体现在
        // 文案与配色两处，只改文案不改配色等于没改（契约 3.2）。
        Verdict::Insufficient => Tone::Dim,
        Verdict::Preliminary(PreliminaryLevel::Low) | Verdict::Full(Level::Low) => Tone::Ok,
        Verdict::Preliminary(PreliminaryLevel::Medium) | Verdict::Full(Level::Medium) => Tone::Warn,
        Verdict::Full(Level::High) => Tone::Bad,
    }
}

fn verdict_headline<'a>(verdict: &Verdict, text: &'a Text) -> (&'a str, Option<&'a str>, &'a str) {
    match verdict {
        Verdict::Insufficient => (
            text.verdict.insufficient,
            None,
            text.verdict.summary_insufficient,
        ),
        Verdict::Preliminary(PreliminaryLevel::Low) => (
            text.verdict.low,
            Some(text.verdict.preliminary_badge),
            text.verdict.summary_preliminary_low,
        ),
        Verdict::Preliminary(PreliminaryLevel::Medium) => (
            text.verdict.medium,
            Some(text.verdict.preliminary_badge),
            text.verdict.summary_preliminary_medium,
        ),
        Verdict::Full(Level::Low) => (
            text.verdict.low,
            Some(text.verdict.full_badge),
            text.verdict.summary_full_low,
        ),
        Verdict::Full(Level::Medium) => (
            text.verdict.medium,
            Some(text.verdict.full_badge),
            text.verdict.summary_full_medium,
        ),
        Verdict::Full(Level::High) => (
            text.verdict.high,
            Some(text.verdict.full_badge),
            text.verdict.summary_full_high,
        ),
    }
}

/// 一张检测卡的内容。
struct Card {
    id: CheckId,
    tone: Tone,
    title: &'static str,
    /// 主值，一行一条。
    values: Vec<String>,
    /// 契约要求必须出现的说明，与 `--verbose` 无关。
    notes: Vec<&'static str>,
    description: &'static str,
}

pub fn report(report: &Report, text: &Text, style: &Style, verbose: bool) -> String {
    let verdict = report.verdict();
    let coverage = report.coverage();
    debug_assert!(coverage.is_complete(), "覆盖度不变量被破坏：{coverage:?}");

    let mut out = String::new();
    let _ = writeln!(out);
    render_verdict(&mut out, &verdict, &coverage, report, text, style);
    let _ = writeln!(out);

    for card in cards(report, text) {
        render_card(&mut out, &card, style, verbose);
    }
    let _ = writeln!(out);

    out
}

fn render_verdict(
    out: &mut String,
    verdict: &Verdict,
    coverage: &Coverage,
    report: &Report,
    text: &Text,
    style: &Style,
) {
    let tone = verdict_tone(verdict);
    let (level, badge, summary) = verdict_headline(verdict, text);

    let headline = match badge {
        Some(badge) => format!("{}  {}", style.bold(level), style.tone(Tone::Dim, badge)),
        None => style.bold(level),
    };
    let _ = writeln!(out, "  {} {headline}", style.tone(tone, marker(tone)));
    let _ = writeln!(out, "    {summary}");

    if let Outcome::Done(info) = &report.o1 {
        let _ = writeln!(out, "    {}  {}", text.verdict.exit_ip_label, info.ip);
    }

    // 综合结论永远不得脱离覆盖度单独呈现（ADR-0004）。
    let _ = writeln!(
        out,
        "    {}",
        style.tone(
            Tone::Dim,
            &format!(
                "{} {} · {} {}",
                text.coverage.done, coverage.done, text.coverage.failed, coverage.failed
            )
        )
    );
}

fn render_card(out: &mut String, card: &Card, style: &Style, verbose: bool) {
    let _ = writeln!(
        out,
        "  {} {}  {}",
        style.tone(card.tone, marker(card.tone)),
        style.tone(Tone::Dim, card.id.as_str()),
        style.bold(card.title),
    );
    for value in &card.values {
        let _ = writeln!(out, "       {value}");
    }
    for note in &card.notes {
        let _ = writeln!(out, "       {}", style.tone(Tone::Dim, note));
    }
    if verbose {
        let _ = writeln!(out, "       {}", style.tone(Tone::Dim, card.description));
    }
}

fn failure_text(failure: Failure, text: &Text) -> &'static str {
    match failure {
        Failure::Upstream => text.failures.upstream,
        Failure::QuotaExhausted => text.failures.quota_exhausted,
        Failure::Local => text.failures.local,
    }
}

/// 失败的检测项统一渲染成一张灰卡。**不能不渲染**——覆盖度里数着它。
fn failed_card(
    id: CheckId,
    title: &'static str,
    description: &'static str,
    failure: Failure,
    text: &Text,
) -> Card {
    Card {
        id,
        tone: Tone::Dim,
        title,
        values: vec![failure_text(failure, text).to_string()],
        notes: if failure == Failure::QuotaExhausted {
            vec![text.notes.quota_shared]
        } else {
            Vec::new()
        },
        description,
    }
}

fn cards(report: &Report, text: &Text) -> Vec<Card> {
    vec![
        card_o1(&report.o1, text),
        card_timezone(CheckId::O2, &report.o2, text, &text.checks.o2, true),
        card_o3(&report.o3, text),
        card_o4(&report.o4, text),
        card_o5(&report.o5, text),
        card_o6(&report.o6, text),
        card_c1(&report.c1, text),
        card_c2(&report.c2, text),
        card_c3(&report.c3, text),
        card_timezone(CheckId::C4, &report.c4, text, &text.checks.c4, false),
    ]
}

fn card_o1(outcome: &Outcome<ExitInfo>, text: &Text) -> Card {
    let meta = &text.checks.o1;
    let Outcome::Done(info) = outcome else {
        return failed_card(
            CheckId::O1,
            meta.title,
            meta.description,
            outcome.failure().unwrap(),
            text,
        );
    };

    let mut values = vec![info.ip.clone()];
    let mut notes = Vec::new();
    if let Some(geo) = &info.geo {
        let place: Vec<&str> = [&geo.city_name, &geo.region_name, &geo.country_name]
            .into_iter()
            .filter_map(|f| f.as_deref())
            .collect();
        if !place.is_empty() {
            values.push(place.join(", "));
        }
        let network: Vec<&str> = [&geo.asn, &geo.organisation]
            .into_iter()
            .filter_map(|f| f.as_deref())
            .collect();
        if !network.is_empty() {
            values.push(network.join("  "));
        }
        // 契约 5.4：必须标明归属来自 proxycheck，否则用户拿两边结果对不上时
        // 会以为有一边算错了。
        notes.push(text.notes.geo_source);
    } else {
        values.push(text.values.unknown.to_string());
    }

    Card {
        id: CheckId::O1,
        tone: Tone::Ok,
        title: meta.title,
        values,
        notes,
        description: meta.description,
    }
}

fn card_timezone(
    id: CheckId,
    outcome: &Outcome<TimezoneCheck>,
    text: &Text,
    meta: &crate::copy::CheckText,
    desktop_note: bool,
) -> Card {
    let Outcome::Done(check) = outcome else {
        return failed_card(
            id,
            meta.title,
            meta.description,
            outcome.failure().unwrap(),
            text,
        );
    };

    let (tone, label) = match check.matches {
        Some(true) => (Tone::Ok, text.values.timezone_match),
        // O2 在 CLI 侧只是提醒（黄），C4 才进综合结论（红）。
        Some(false) if desktop_note => (Tone::Warn, text.values.timezone_mismatch),
        Some(false) => (Tone::Bad, text.values.timezone_mismatch),
        None => (Tone::Warn, text.values.timezone_indeterminate),
    };

    let detail = format!(
        "{}  {}  →  {}",
        label,
        check.local.as_deref().unwrap_or(text.values.unknown),
        check.exit.as_deref().unwrap_or(text.values.unknown),
    );

    Card {
        id,
        tone,
        title: meta.title,
        values: vec![detail],
        notes: if desktop_note {
            // 契约 5.1：缺了这句话，CLI 用户会误以为自己的 $TZ 已被检查。
            vec![text.notes.o2_desktop_only]
        } else {
            Vec::new()
        },
        description: meta.description,
    }
}

fn card_o3(outcome: &Outcome<ipify::Ipv6>, text: &Text) -> Card {
    let meta = &text.checks.o3;
    let Outcome::Done(result) = outcome else {
        return failed_card(
            CheckId::O3,
            meta.title,
            meta.description,
            outcome.failure().unwrap(),
            text,
        );
    };

    let (tone, value) = match result {
        ipify::Ipv6::Leaked(addr) => (Tone::Warn, format!("{}  {addr}", text.values.ipv6_leaked)),
        ipify::Ipv6::Disabled => (Tone::Ok, text.values.ipv6_disabled.to_string()),
        // Indeterminate 在组装时已被折成检测失败，走不到这里。
        ipify::Ipv6::Indeterminate => (Tone::Dim, text.values.unknown.to_string()),
    };

    Card {
        id: CheckId::O3,
        tone,
        title: meta.title,
        values: vec![value],
        notes: Vec::new(),
        description: meta.description,
    }
}

fn card_o4(outcome: &Outcome<Risk>, text: &Text) -> Card {
    let meta = &text.checks.o4;
    let Outcome::Done(result) = outcome else {
        return failed_card(
            CheckId::O4,
            meta.title,
            meta.description,
            outcome.failure().unwrap(),
            text,
        );
    };

    let score = result.risk.risk_score;
    let tone = match verdict::risk_level(score) {
        Level::Low => Tone::Ok,
        Level::Medium => Tone::Warn,
        Level::High => Tone::Bad,
    };

    let mut values = vec![format!("{score}/100")];
    if let Some(kind) = &result.risk.network_type {
        values.push(kind.clone());
    }
    let flags: Vec<&str> = [
        ("proxy", result.risk.proxy),
        ("vpn", result.risk.vpn),
        ("tor", result.risk.tor),
        ("scraper", result.risk.scraper),
    ]
    .into_iter()
    .filter_map(|(name, hit)| hit.then_some(name))
    .collect();
    if !flags.is_empty() {
        values.push(flags.join("  "));
    }

    // anonymous 决定判「高」的阈值（契约 3.1）。不显示它，用户就看不出同样的分数
    // 为什么这次判了高——那是把判据藏起来。
    if result.risk.anonymous {
        values.push(text.values.anonymous_flag.to_string());
    }

    values.push(match &result.abuse {
        // 未知不冒充「无收录」（契约 2.3）。
        None => text.values.abuse_unknown.to_string(),
        Some(abuse) if abuse.listed => {
            format!("{} ({})", text.values.abuse_listed, abuse.frequency)
        }
        Some(_) => text.values.abuse_clean.to_string(),
    });

    Card {
        id: CheckId::O4,
        tone,
        title: meta.title,
        values,
        notes: Vec::new(),
        description: meta.description,
    }
}

fn card_o5(outcome: &Outcome<dns_egress::DnsEgress>, text: &Text) -> Card {
    let meta = &text.checks.o5;
    let Outcome::Done(result) = outcome else {
        return failed_card(
            CheckId::O5,
            meta.title,
            meta.description,
            outcome.failure().unwrap(),
            text,
        );
    };

    let dt = &text.dns_egress;
    let (tone, mut values) = match &result.comparison {
        dns_egress::Comparison::Comparable {
            leak,
            ecs_country,
            exit_country,
        } => {
            let comparison_line = format!(
                "{}  {}  →  {}  {}",
                dt.ecs_label, ecs_country, dt.exit_label, exit_country,
            );
            let leak_message = if *leak { dt.leak } else { dt.no_leak };
            let tone = if *leak { Tone::Warn } else { Tone::Ok };
            (tone, vec![comparison_line, leak_message.to_string()])
        }
        dns_egress::Comparison::NotComparable(reason) => {
            // 三种「无从比对」各自的说明，绝不回退成「泄露」或「未泄露」（契约 2.5 硬约束 3）。
            let message = match reason {
                dns_egress::NotComparable::NoEcs => dt.no_ecs,
                dns_egress::NotComparable::UnmappedCountry => dt.unmapped_country,
                dns_egress::NotComparable::UnknownExitCountry => dt.unknown_exit_country,
            };
            (Tone::Dim, vec![message.to_string()])
        }
    };

    // resolver 归属始终展示（与 §5.1 里 CLI 同时展示 $TZ 与系统时区同构），
    // 但 notes 明确标出只有上面的 ECS 判定进综合结论。
    values.push(format!(
        "{}  {}",
        dt.resolver_label,
        result
            .resolver_geo
            .as_deref()
            .unwrap_or(text.values.unknown),
    ));

    Card {
        id: CheckId::O5,
        tone,
        title: meta.title,
        values,
        notes: vec![dt.resolver_note],
        description: meta.description,
    }
}

fn card_o6(outcome: &Outcome<udp_egress::UdpEgress>, text: &Text) -> Card {
    let meta = &text.checks.o6;
    let Outcome::Done(result) = outcome else {
        return failed_card(
            CheckId::O6,
            meta.title,
            meta.description,
            outcome.failure().unwrap(),
            text,
        );
    };

    let ut = &text.udp_egress;
    let (tone, values) = match result {
        udp_egress::UdpEgress::Comparable {
            mismatch,
            reflexive_ip,
            exit_ip,
        } => {
            let comparison_line = format!(
                "{}  {}  →  {}  {}",
                ut.reflexive_label, reflexive_ip, ut.exit_label, exit_ip,
            );
            let mismatch_message = if *mismatch {
                ut.mismatch
            } else {
                ut.no_mismatch
            };
            let tone = if *mismatch { Tone::Warn } else { Tone::Ok };
            (tone, vec![comparison_line, mismatch_message.to_string()])
        }
        udp_egress::UdpEgress::NotComparable(reason) => {
            // 「无从比对」的三种成因与「未命中」文案可分（契约 2.6）。
            let message = match reason {
                udp_egress::NotComparable::FamilyMismatch => ut.family_mismatch,
                udp_egress::NotComparable::UnknownExitIp => ut.unknown_exit_ip,
                udp_egress::NotComparable::StunDisagree => ut.stun_disagree,
            };
            (Tone::Dim, vec![message.to_string()])
        }
    };

    Card {
        id: CheckId::O6,
        tone,
        title: meta.title,
        values,
        notes: Vec::new(),
        description: meta.description,
    }
}

fn card_c1(outcome: &Outcome<String>, text: &Text) -> Card {
    let meta = &text.checks.c1;
    match outcome {
        Outcome::Done(ip) => Card {
            id: CheckId::C1,
            tone: Tone::Ok,
            title: meta.title,
            values: vec![ip.clone()],
            notes: Vec::new(),
            description: meta.description,
        },
        Outcome::Failed(failure) => {
            failed_card(CheckId::C1, meta.title, meta.description, *failure, text)
        }
    }
}

fn card_c2(outcome: &Outcome<Vec<dns::Server>>, text: &Text) -> Card {
    let meta = &text.checks.c2;
    let Outcome::Done(servers) = outcome else {
        return failed_card(
            CheckId::C2,
            meta.title,
            meta.description,
            outcome.failure().unwrap(),
            text,
        );
    };

    let domestic = servers.iter().any(|s| s.domestic);
    let values = servers
        .iter()
        .map(|server| {
            let mut line = server.address.clone();
            if let Some(label) = server.label {
                line.push_str("  ");
                line.push_str(label);
            } else if server.private {
                line.push_str("  ");
                line.push_str(text.values.dns_router);
            }
            line
        })
        .collect();

    Card {
        id: CheckId::C2,
        // 国内 DNS 只是提醒，不进综合结论（契约 2.1）。
        tone: if domestic { Tone::Warn } else { Tone::Ok },
        title: meta.title,
        values,
        notes: Vec::new(),
        description: meta.description,
    }
}

fn card_c3(outcome: &Outcome<proxy::Status>, text: &Text) -> Card {
    let meta = &text.checks.c3;
    let Outcome::Done(status) = outcome else {
        return failed_card(
            CheckId::C3,
            meta.title,
            meta.description,
            outcome.failure().unwrap(),
            text,
        );
    };

    let state_label = |state: &proxy::State| match state {
        proxy::State::Enabled => text.values.state_on,
        proxy::State::Disabled => text.values.state_off,
        proxy::State::Unsupported => text.values.state_unsupported,
    };

    // 只报开关状态，绝不显示地址——把 127.0.0.1:7890 打出来等于替用户泄露配置。
    let mut values = vec![format!(
        "{}  {}",
        text.values.proxy_env,
        state_label(&status.env_state())
    )];
    let mut system = format!(
        "{}  {}",
        text.values.proxy_system,
        state_label(&status.system)
    );
    if !status.system_kinds.is_empty() {
        system.push_str("  ");
        system.push_str(&status.system_kinds.join(" "));
    }
    values.push(system);
    values.push(format!(
        "{}  {}",
        text.values.proxy_tun,
        state_label(&status.tun)
    ));

    Card {
        id: CheckId::C3,
        tone: match status.tun_off() {
            Some(true) => Tone::Warn,
            Some(false) => Tone::Ok,
            // 未知不贡献信号，也不该染成警告色。
            None => Tone::Dim,
        },
        title: meta.title,
        values,
        notes: Vec::new(),
        description: meta.description,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copy;
    use crate::domain::checks::Failure;
    use crate::lang::Lang;
    use crate::probe::TimezoneCheck;

    fn blank() -> Report {
        Report {
            o1: Outcome::Failed(Failure::Upstream),
            o2: Outcome::Failed(Failure::Upstream),
            o3: Outcome::Failed(Failure::Upstream),
            o4: Outcome::Failed(Failure::Upstream),
            o5: Outcome::Failed(Failure::Upstream),
            o6: Outcome::Failed(Failure::Upstream),
            c1: Outcome::Failed(Failure::Upstream),
            c2: Outcome::Failed(Failure::Local),
            c3: Outcome::Failed(Failure::Local),
            c4: Outcome::Failed(Failure::Upstream),
        }
    }

    fn render(report: &Report, color: bool, verbose: bool) -> String {
        super::report(report, &copy::text(Lang::En), &Style::new(color), verbose)
    }

    #[test]
    fn no_color_output_has_no_escape_sequences() {
        // 重定向到文件不该满屏转义序列。
        let out = render(&blank(), false, false);
        assert!(!out.contains('\x1b'), "{out}");
    }

    #[test]
    fn color_output_does_have_escape_sequences() {
        assert!(render(&blank(), true, false).contains('\x1b'));
    }

    #[test]
    fn every_check_is_rendered_even_when_it_failed() {
        // 覆盖度里数着它们，屏幕上就必须有它们。
        let out = render(&blank(), false, false);
        for id in ["O1", "O2", "O3", "O4", "O5", "O6", "C1", "C2", "C3", "C4"] {
            assert!(out.contains(id), "缺少 {id}：{out}");
        }
    }

    #[test]
    fn total_failure_never_renders_as_low_risk() {
        let text = copy::text(Lang::En);
        let out = render(&blank(), false, false);
        assert!(out.contains(text.verdict.insufficient));
        assert!(!out.contains(text.verdict.low));
    }

    #[test]
    fn coverage_is_always_next_to_the_verdict() {
        let text = copy::text(Lang::En);
        let out = render(&blank(), false, false);
        assert!(out.contains(&format!("{} 0", text.coverage.done)));
        assert!(out.contains(&format!("{} 10", text.coverage.failed)));
    }

    #[test]
    fn descriptions_only_appear_with_verbose() {
        let text = copy::text(Lang::En);
        assert!(!render(&blank(), false, false).contains(text.checks.o1.description));
        assert!(render(&blank(), false, true).contains(text.checks.o1.description));
    }

    #[test]
    fn quota_exhausted_tells_the_user_how_to_fix_it() {
        let mut report = blank();
        report.o4 = Outcome::Failed(Failure::QuotaExhausted);
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);
        assert!(out.contains(text.failures.quota_exhausted));
        // 共享配额那句必须在——否则用户以为工具坏了。
        assert!(out.contains(text.notes.quota_shared));
    }

    #[test]
    fn the_o2_card_says_it_only_covers_the_desktop_app() {
        // 契约 5.1 的呈现约束：缺了这句，CLI 用户会误以为 $TZ 已被检查。
        let mut report = blank();
        report.o2 = Outcome::Done(TimezoneCheck {
            local: Some("Asia/Shanghai".into()),
            exit: Some("Asia/Shanghai".into()),
            matches: Some(true),
        });
        let text = copy::text(Lang::En);
        assert!(render(&report, false, false).contains(text.notes.o2_desktop_only));
    }

    #[test]
    fn the_o1_card_names_its_geo_source() {
        // 契约 5.4：两端归属可能对不上，用户得看得出是数据源不同。
        let mut report = blank();
        report.o1 = Outcome::Done(ExitInfo {
            ip: "203.0.113.7".into(),
            geo: Some(crate::probe::proxycheck::Geo {
                country_name: Some("Australia".into()),
                country_code: Some("AU".into()),
                region_name: None,
                city_name: Some("Sydney".into()),
                timezone: Some("Australia/Sydney".into()),
                asn: Some("AS13335".into()),
                organisation: Some("Cloudflare, Inc.".into()),
                provider: None,
            }),
        });
        let text = copy::text(Lang::En);
        assert!(render(&report, false, false).contains(text.notes.geo_source));
    }

    #[test]
    fn the_o4_card_surfaces_the_anonymous_flag() {
        // 它决定判「高」的阈值。藏起来的话，用户看不出同样的分数为什么判了高。
        let mut report = blank();
        report.o4 = Outcome::Done(Risk {
            risk: crate::probe::proxycheck::Risk {
                network_type: None,
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: 60,
                anonymous: true,
            },
            abuse: None,
        });
        let text = copy::text(Lang::En);
        assert!(render(&report, false, false).contains(text.values.anonymous_flag));
    }

    #[test]
    fn the_o5_card_shows_both_the_ecs_verdict_and_the_resolver_ownership() {
        // 呈现层硬约束：O5 必须同时展示 ECS 判定结果与 resolver 归属，
        // 并标明只有前者进综合结论——与 §5.1 里 CLI 同时展示 $TZ 与系统时区同构。
        let mut report = blank();
        report.o5 = Outcome::Done(dns_egress::DnsEgress {
            resolver_geo: Some("Japan - Google LLC".into()),
            comparison: dns_egress::Comparison::Comparable {
                leak: true,
                ecs_country: "JP".into(),
                exit_country: "US".into(),
            },
        });
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);
        assert!(out.contains("JP"), "{out}");
        assert!(out.contains("US"), "{out}");
        assert!(out.contains("Japan - Google LLC"), "{out}");
        assert!(out.contains(text.dns_egress.resolver_note), "{out}");
    }

    #[test]
    fn the_o5_card_names_the_missing_ecs_and_stays_done() {
        // 呈现层硬约束：ECS 缺失时明写「你的 DNS 服务商不发送 ECS」，状态仍是「已完成」
        // 而不是「third-party unavailable」——O5 卡片本身必须不落到失败态的措辞里
        // （其余卡片仍失败，因此不能对整份输出断言，只看 O5 到 O6 之间那一段）。
        let mut report = blank();
        report.o5 = Outcome::Done(dns_egress::DnsEgress {
            resolver_geo: Some("Japan - Cloudflare, Inc.".into()),
            comparison: dns_egress::Comparison::NotComparable(dns_egress::NotComparable::NoEcs),
        });
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);
        let o5_start = out.find(" O5  ").expect("必须有 O5 卡片");
        let o6_start = out.find(" O6  ").expect("必须有 O6 卡片");
        let o5_card = &out[o5_start..o6_start];
        assert!(o5_card.contains(text.dns_egress.no_ecs), "{o5_card}");
        assert!(!o5_card.contains(text.failures.upstream), "{o5_card}");
    }

    #[test]
    fn the_o6_card_tells_a_not_comparable_reason_apart_from_a_miss() {
        // 呈现层硬约束：O6「无从比对」与「未命中」的文案必须可分。
        let text = copy::text(Lang::En);

        let mut disagree = blank();
        disagree.o6 = Outcome::Done(udp_egress::UdpEgress::NotComparable(
            udp_egress::NotComparable::StunDisagree,
        ));
        let out = render(&disagree, false, false);
        assert!(out.contains(text.udp_egress.stun_disagree), "{out}");
        assert!(!out.contains(text.udp_egress.no_mismatch), "{out}");

        let mut miss = blank();
        miss.o6 = Outcome::Done(udp_egress::UdpEgress::Comparable {
            mismatch: false,
            reflexive_ip: "198.51.100.20".parse().unwrap(),
            exit_ip: "198.51.100.20".parse().unwrap(),
        });
        let out = render(&miss, false, false);
        assert!(out.contains(text.udp_egress.no_mismatch), "{out}");
        assert!(!out.contains(text.udp_egress.stun_disagree), "{out}");
    }

    #[test]
    fn proxy_card_never_prints_addresses() {
        let mut report = blank();
        report.c3 = Outcome::Done(proxy::Status {
            env_vars: vec!["HTTP_PROXY".into()],
            system: proxy::State::Enabled,
            system_kinds: vec!["HTTP".into(), "HTTPS".into()],
            tun: proxy::State::Disabled,
        });
        let out = render(&report, false, false);
        assert!(!out.contains("127.0.0.1"), "{out}");
        assert!(!out.contains("7890"), "{out}");
    }
}
