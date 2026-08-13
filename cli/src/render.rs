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

    let all_cards = cards(report, text);

    let mut out = String::new();
    let _ = writeln!(out);
    render_verdict(
        &mut out, &verdict, &coverage, report, &all_cards, text, style,
    );
    let _ = writeln!(out);

    for card in &all_cards {
        render_card(&mut out, card, style, verbose);
    }
    let _ = writeln!(out);

    out
}

fn render_verdict(
    out: &mut String,
    verdict: &Verdict,
    coverage: &Coverage,
    report: &Report,
    all_cards: &[Card],
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
        let mut parts = vec![info.ip.clone()];
        if let Some(geo) = &info.geo {
            let place: Vec<&str> = [&geo.city_name, &geo.region_name, &geo.country_name]
                .into_iter()
                .filter_map(|f| f.as_deref())
                .collect();
            if !place.is_empty() {
                parts.push(place.join(", "));
            }
            let network: Vec<&str> = [&geo.asn, &geo.organisation]
                .into_iter()
                .filter_map(|f| f.as_deref())
                .collect();
            if !network.is_empty() {
                parts.push(network.join("  "));
            }
        }
        let _ = writeln!(
            out,
            "    {}  {}",
            text.verdict.exit_ip_label,
            parts.join("  ·  ")
        );
    }

    if let Outcome::Done(risk) = &report.o4 {
        let score = risk.risk.risk_score;
        let bar_tone = match verdict::risk_level(score) {
            Level::Low => Tone::Ok,
            Level::Medium => Tone::Warn,
            Level::High => Tone::Bad,
        };
        let _ = writeln!(
            out,
            "    {}  {score}/100  {}  {}",
            text.verdict.risk_label,
            risk_bar(score, bar_tone, style),
            style.tone(Tone::Dim, text.values.risk_scale_note)
        );
    }

    // 综合结论永远不得脱离覆盖度单独呈现（ADR-0004）。
    let _ = writeln!(
        out,
        "    {}  {}",
        text.verdict.coverage_label,
        style.tone(
            Tone::Dim,
            &format!(
                "{} {} · {} {}",
                text.coverage.done, coverage.done, text.coverage.failed, coverage.failed
            )
        )
    );

    render_attention(out, all_cards, report, verdict, text, style);
}

/// 风险分刻度条：20 格块字符，填充比例 = `score / 100`。
///
/// `--no-color` 下也能读懂——填充长度本身传递信息，不依赖颜色（点 8）。
fn risk_bar(score: u32, tone: Tone, style: &Style) -> String {
    const WIDTH: usize = 20;
    let filled = ((score as usize) * WIDTH + 50) / 100;
    let filled = filled.min(WIDTH);
    let empty = WIDTH - filled;
    format!(
        "{}{}",
        style.tone(tone, &"█".repeat(filled)),
        style.tone(Tone::Dim, &"░".repeat(empty))
    )
}

/// 「需关注」清单从卡片 tone 派生（spec 5.2），不写死检测项列表。
/// 无 warn/bad 项时整块（含 attention_scope 句）不出。
fn render_attention(
    out: &mut String,
    all_cards: &[Card],
    report: &Report,
    verdict: &Verdict,
    text: &Text,
    style: &Style,
) {
    let mut items: Vec<&Card> = all_cards
        .iter()
        .filter(|c| c.tone == Tone::Warn || c.tone == Tone::Bad)
        .collect();
    // bad 先于 warn，同级按 ALL_CHECKS（O1→C4）固定顺序——`cards()` 本就按此顺序
    // 构建，`sort_by_key` 是稳定排序，同级元素的相对顺序保持不变。
    items.sort_by_key(|c| if c.tone == Tone::Bad { 0 } else { 1 });

    if items.is_empty() {
        return;
    }

    let _ = writeln!(out, "    {}", style.bold(text.verdict.attention_label));
    for card in &items {
        let _ = writeln!(
            out,
            "      {} {}  {}",
            style.tone(card.tone, marker(card.tone)),
            style.tone(Tone::Dim, card.id.as_str()),
            card.title,
        );
    }

    // 判据是「信号是否实际被 compute() 消费并命中」，不是卡片 tone——一张
    // Warn/Bad 卡可能来自契约 2.1 明列的纯提醒信号。不在这里重算 51/76 阈值：
    // O4 是否贡献综合结论，直接读 compute() 的结果（只有 O4 的风险分能把结论
    // 判到 Full(High)，这是判级契约 3.2 的不变量，不是这里现算的）。
    let contributing = contributing_ids(&report.signals(), verdict);
    let contributing_ids: Vec<CheckId> = items
        .iter()
        .map(|c| c.id)
        .filter(|id| contributing.contains(id))
        .collect();
    let reminder_ids: Vec<CheckId> = items
        .iter()
        .map(|c| c.id)
        .filter(|id| !contributing.contains(id))
        .collect();

    if let Some(scope) = attention_scope(&contributing_ids, &reminder_ids, &text.verdict) {
        let _ = writeln!(out, "    {}", style.tone(Tone::Dim, &scope));
    }
}

/// `attention_scope` 句：固定短语（前缀/分句标点/收尾标点）+ 已有的 ID 列表拼接
/// 与贡献词/仅提醒词，不引入任何占位符替换或模板语法——与拼 ID 列表用的是
/// 同一套「固定片段 + 渲染层拼接」手法。两个子集都空时（理论上到不了这里，
/// 因为空清单已经在上一层被整块跳过）返回 `None`。
fn attention_scope(
    contributing_ids: &[CheckId],
    reminder_ids: &[CheckId],
    v: &crate::copy::VerdictText,
) -> Option<String> {
    let contributing_clause = (!contributing_ids.is_empty()).then(|| {
        format!(
            "{}{} {}",
            v.attention_prefix,
            join_ids(
                contributing_ids,
                v.attention_list_separator,
                v.attention_list_connector
            ),
            v.attention_contributing
        )
    });
    let reminder_clause = (!reminder_ids.is_empty()).then(|| {
        format!(
            "{} {}",
            join_ids(
                reminder_ids,
                v.attention_list_separator,
                v.attention_list_connector
            ),
            v.attention_reminder_only
        )
    });

    match (contributing_clause, reminder_clause) {
        (Some(c), Some(r)) => Some(format!(
            "{c}{}{r}{}",
            v.attention_clause_separator, v.attention_suffix
        )),
        (Some(c), None) => Some(format!("{c}{}", v.attention_suffix)),
        (None, Some(r)) => Some(format!("{r}{}", v.attention_suffix)),
        (None, None) => None,
    }
}

/// 哪些检测项的信号实际被 `verdict::compute()` 消费并命中（贡献综合结论）。
///
/// 只读 `Signals` 与 `Verdict` 已经算好的结果，不重算阈值：
/// - 除 O4 外的贡献信号都是「命中即贡献」的布尔量，直接对照 `Signals` 字段。
/// - O4（风险分）能否把结论抬到 `Full(High)`，取决于 `anonymous` 选择的
///   51/76 阈值——但契约 3.2 保证了"高档信号只能来自 O4"，所以只需要看
///   `verdict` 是否已经是 `Full(High)`，不必在这里重新比较分数与阈值。
///   O4 也可能通过 `abuse_listed` 贡献到「中」，与分数无关。
fn contributing_ids(signals: &verdict::Signals, verdict: &Verdict) -> Vec<CheckId> {
    let mut ids = Vec::new();
    if signals.tz_mismatch_cli_env == Some(true) {
        ids.push(CheckId::C4);
    }
    if signals.ipv6_leak == Some(true) {
        ids.push(CheckId::O3);
    }
    if signals.dns_egress_leak == Some(true) {
        ids.push(CheckId::O5);
    }
    if signals.udp_egress_mismatch == Some(true) {
        ids.push(CheckId::O6);
    }
    if signals.tun_off == Some(true) {
        ids.push(CheckId::C3);
    }
    if signals.abuse_listed == Some(true) || matches!(verdict, Verdict::Full(Level::High)) {
        ids.push(CheckId::O4);
    }
    ids
}

/// 用分隔符/连接词把编号列表拼成一句（如「O2 与 O4」「O2、O4 与 O6」）。
/// 中英文标点不同，两个词都来自 Copy（spec 5.2 的裁定），不写死在这里。
///
/// 连接词两侧的空格由渲染层统一补上（`connector.trim()` 后包一层单空格），
/// 不进 Copy——Copy 里的连接词保持裸词（`与`/`and`），带空格的文案值是脆弱写法，
/// 复制粘贴、trim、对齐时都会出问题。分隔符（顿号/逗号）不受影响，中英文本就
/// 各自吸收了自己的空格习惯，C2 给的取值原样使用。
fn join_ids(ids: &[CheckId], separator: &str, connector: &str) -> String {
    let strs: Vec<&str> = ids.iter().map(|id| id.as_str()).collect();
    let connector = format!(" {} ", connector.trim());
    match strs.split_last() {
        None => String::new(),
        Some((last, [])) => (*last).to_string(),
        Some((last, rest)) => format!("{}{connector}{last}", rest.join(separator)),
    }
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

    fn risk_report(score: u32, anonymous: bool) -> Report {
        let mut report = blank();
        report.o4 = Outcome::Done(Risk {
            risk: crate::probe::proxycheck::Risk {
                network_type: None,
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: score,
                anonymous,
            },
            abuse: None,
        });
        report
    }

    // ---- 判别力测试（brief 验证项 5，成对，缺一不可）----
    // 同一分数（60，O4 分项黄），只翻转 anonymous，综合结论方向相反：
    // - anonymous: false → 综合结论判高的阈值是 76，60 没过线，O4 仅作提醒。
    // - anonymous: true  → 综合结论判高的阈值降到 51，60 过线，O4 参与判定。
    // 判据必须来自 verdict::compute() 的结果，不能是卡片 tone（两次卡片 tone 都是黄）。

    #[test]
    fn not_anonymous_score_60_does_not_count_o4_toward_the_verdict() {
        let report = risk_report(60, false);
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);
        assert_eq!(report.verdict(), Verdict::Full(Level::Low));
        assert!(
            out.contains(&format!("O4 {}", text.verdict.attention_reminder_only)),
            "{out}"
        );
        assert!(
            !out.contains(&format!("O4 {}", text.verdict.attention_contributing)),
            "{out}"
        );
    }

    #[test]
    fn anonymous_score_60_does_count_o4_toward_the_verdict() {
        let report = risk_report(60, true);
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);
        assert_eq!(report.verdict(), Verdict::Full(Level::High));
        assert!(
            out.contains(&format!("O4 {}", text.verdict.attention_contributing)),
            "{out}"
        );
        assert!(
            !out.contains(&format!("O4 {}", text.verdict.attention_reminder_only)),
            "{out}"
        );
    }

    #[test]
    fn attention_block_is_absent_when_no_card_is_warn_or_bad() {
        // blank() 全部检测失败——失败卡是 Dim，不是 Warn/Bad。
        let text = copy::text(Lang::En);
        let out = render(&blank(), false, false);
        assert!(!out.contains(text.verdict.attention_label), "{out}");
    }

    #[test]
    fn attention_scope_splits_contributing_from_reminder_only_items() {
        // 混合场景：C4（tzMismatchCliEnv 命中，贡献）+ O2（系统时区不一致，契约 2.1
        // 明列的纯提醒信号）+ O4（分项黄但分数没过线，仅提醒）。
        let mut report = blank();
        report.c4 = Outcome::Done(TimezoneCheck {
            local: Some("Asia/Shanghai".into()),
            exit: Some("America/New_York".into()),
            matches: Some(false),
        });
        report.o2 = Outcome::Done(TimezoneCheck {
            local: Some("Asia/Shanghai".into()),
            exit: Some("America/New_York".into()),
            matches: Some(false),
        });
        report.o4 = Outcome::Done(Risk {
            risk: crate::probe::proxycheck::Risk {
                network_type: None,
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: 40,
                anonymous: false,
            },
            abuse: None,
        });

        let text = copy::text(Lang::En);
        let out = render(&report, false, false);

        // 需关注清单三项都在，且都在结论区（O1 卡片之后才是下面的卡片流）。
        assert!(out.contains(text.verdict.attention_label), "{out}");
        assert!(
            out.contains(&format!("C4 {}", text.verdict.attention_contributing)),
            "{out}"
        );
        // 连接词两侧的空格由渲染层统一补上，不是 Copy 里 en/zh 各自凑巧带对——
        // 这里锁的是渲染结果，不是拿 Copy 原始取值去拼期望值。
        assert!(
            out.contains(&format!(
                "O2 and O4 {}",
                text.verdict.attention_reminder_only
            )),
            "{out}"
        );
    }

    #[test]
    fn attention_scope_connector_gets_single_spaces_in_both_languages() {
        // zh_hans 的 attention_list_connector 是裸词"与"（不带空格），en 是" and "
        // （C2 给的取值本就带空格）——渲染层对两者一视同仁地 trim 再包一层单空格，
        // 结果都应该是单空格，不依赖 Copy 里连接词本身带不带空格。
        let mut report = blank();
        report.o2 = Outcome::Done(TimezoneCheck {
            local: Some("Asia/Shanghai".into()),
            exit: Some("America/New_York".into()),
            matches: Some(false),
        });
        report.o4 = Outcome::Done(Risk {
            risk: crate::probe::proxycheck::Risk {
                network_type: None,
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: 40,
                anonymous: false,
            },
            abuse: None,
        });

        let out_en = render(&report, false, false);
        assert!(
            out_en.contains("O2 and O4 flagged for awareness only"),
            "{out_en}"
        );
        assert!(!out_en.contains("O2  and"), "double space leaked: {out_en}");

        let out_zh = super::report(
            &report,
            &copy::text(Lang::ZhHans),
            &Style::new(false),
            false,
        );
        assert!(out_zh.contains("O2 与 O4 仅作提醒"), "{out_zh}");
    }

    #[test]
    fn join_ids_uses_the_separator_before_the_last_connector_for_three_or_more_items() {
        // 三项及以上：分隔符（顿号/逗号）与连接词（与/and）都要正确出现一次，
        // 且连接词两侧仍是单空格。
        let mut report = blank();
        report.o2 = Outcome::Done(TimezoneCheck {
            local: Some("Asia/Shanghai".into()),
            exit: Some("America/New_York".into()),
            matches: Some(false),
        });
        report.o4 = Outcome::Done(Risk {
            risk: crate::probe::proxycheck::Risk {
                network_type: None,
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: 40,
                anonymous: false,
            },
            abuse: None,
        });
        report.c2 = Outcome::Done(vec![dns::Server {
            address: "114.114.114.114".into(),
            label: None,
            private: false,
            domestic: true,
        }]);

        let out_en = render(&report, false, false);
        assert!(
            out_en.contains("O2, O4 and C2 flagged for awareness only"),
            "{out_en}"
        );

        let out_zh = super::report(
            &report,
            &copy::text(Lang::ZhHans),
            &Style::new(false),
            false,
        );
        assert!(out_zh.contains("O2、O4 与 C2 仅作提醒"), "{out_zh}");
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
