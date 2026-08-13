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
    /// 标题行右端的状态词（spec §5.1）。失败卡没有状态词——不给失败卡编造状态
    /// （brief 要点 7），失败原因已经在 `values` 里。
    state: Option<String>,
    /// 状态词的展示色调。多数情况下与 `tone` 相同；O1/C1 固定用 `Tone::Dim`——
    /// 「已取得」是纯提示词，不是契约 6 的分项分级评价，不该借用评价色。
    state_tone: Tone,
    /// 主值，一行一条。
    values: Vec<String>,
    /// 契约要求必须出现的说明，与 `--verbose` 无关。
    notes: Vec<&'static str>,
    /// 只有 C4 命中不一致时才有（决策 5：只有 C4 给修复命令）。
    fix: Option<CardFix>,
    description: &'static str,
}

/// C4 专属：成因句 + 修复命令。两者都含动态时区名，按仓库既有风格
/// （见 `ErrorText` 的注释）不做插值，由调用方拼接固定片段与动态值。
struct CardFix {
    explain: String,
    command: String,
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

    render_footer(&mut out, text, style);

    out
}

/// 页脚提示行（design：`.footer-hint`）。命令字面量以 `main.rs` 的 `Cli`/
/// `Command`/`ConfigAction`/`SettableKey` 定义为准：`--verbose`/`--json` 是
/// 顶层 flag，`config set proxycheck-key` 是 `SettableKey` 目前唯一枚举的键。
/// 只在 `render::report` 里调用——`--json` 走 `json::report`，完全不经过这里，
/// 提示行天然不会混进机器可读输出。
fn render_footer(out: &mut String, text: &Text, style: &Style) {
    let f = &text.footer;
    let _ = writeln!(
        out,
        "  ipcheck --verbose  {}  ·  ipcheck --json  {}",
        style.tone(Tone::Dim, f.verbose_hint),
        style.tone(Tone::Dim, f.json_hint)
    );
    let _ = writeln!(
        out,
        "  ipcheck config set proxycheck-key  {}",
        style.tone(Tone::Dim, f.quota_hint)
    );
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
    // 判据是「信号是否实际被 compute() 消费并命中」，不是卡片 tone——一张
    // Warn/Bad 卡可能来自契约 2.1 明列的纯提醒信号。不在这里重算 51/76 阈值：
    // O4 是否贡献综合结论，直接读 compute() 的结果（只有 O4 的风险分能把结论
    // 判到 Full(High)，这是判级契约 3.2 的不变量，不是这里现算的）。
    let contributing = contributing_ids(&report.signals(), verdict);

    // 候选集合不能只看 tone：O4 卡片的 tone 只读 risk_score（契约 6），完全不读
    // abuse_listed；但 abuse_listed 是独立于分数的贡献信号（滥用收录来自
    // StopForumSpam，风险分来自 proxycheck，两个第三方彼此解耦）。分数低、
    // tone=Ok 时 O4 卡片可能仍然贡献了综合结论——这种情形必须进候选集合，
    // 否则会把「真的贡献了」的项完全隐去，比误判成「参与」更糟。
    let mut items: Vec<&Card> = all_cards
        .iter()
        .filter(|c| c.tone == Tone::Warn || c.tone == Tone::Bad || contributing.contains(&c.id))
        .collect();
    // bad 先于 warn（含 tone=Ok 但贡献的项，按非 bad 处理），同级按 ALL_CHECKS
    // （O1→C4）固定顺序——`cards()` 本就按此顺序构建，`sort_by_key` 是稳定排序，
    // 同级元素的相对顺序保持不变。
    items.sort_by_key(|c| if c.tone == Tone::Bad { 0 } else { 1 });

    if items.is_empty() {
        return;
    }

    let _ = writeln!(out, "    {}", style.bold(text.verdict.attention_label));
    for card in &items {
        // 清单的 marker 取「清单自身的语义」，不取卡片 tone：卡片 tone 只读分项
        // 分级（契约 6，只看分数），清单要说的是「这项影响了结论/值得看一眼」。
        // 一张卡片能进这份清单，只有两种原因——tone 本身是 Warn/Bad，或者
        // tone=Ok 但因为其他信号（如 abuse_listed）贡献了综合结论；后一种
        // 情形如果继续显示卡片自己的绿色 marker，会在标题为「需关注」的清单里
        // 出现一行「没问题」，逻辑自相矛盾。两种情形在清单里统一按 warn 处理
        // （bad 除外，bad 仍是 bad）——不引入第三种视觉状态。
        // 卡片自身的渲染（`render_card`）不受影响，仍然按契约 6 显示真实 tone：
        // 「这项分数不高」和「这项影响了结论」是两句都对的话，各自在各自的位置上。
        let list_tone = if card.tone == Tone::Bad {
            Tone::Bad
        } else {
            Tone::Warn
        };
        let _ = writeln!(
            out,
            "      {} {}  {}",
            style.tone(list_tone, marker(list_tone)),
            style.tone(Tone::Dim, card.id.as_str()),
            card.title,
        );
    }

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

/// 说明文字（`note`/`description`/C4 成因句）用的缩进——与 `values` 现有的
/// 7 格缩进一致，折行后的续行悬挂对齐到同一列，不顶格。
const NOTE_INDENT: &str = "       ";
/// 说明文字收进的总列宽（含缩进），spec §5.5。
const WRAP_WIDTH: usize = 76;

/// 按空格折行，收进 `WRAP_WIDTH - NOTE_INDENT` 列内；不含空格的超宽「词」
/// （典型是中文长句，中文标点不分隔单词）按字符数强制切分。不处理 CJK
/// 视觉宽度——文案本身较短，够用即可（brief 要点 4）。
fn wrap_lines(text: &str) -> Vec<String> {
    let avail = WRAP_WIDTH.saturating_sub(NOTE_INDENT.len());

    let mut tokens: Vec<String> = Vec::new();
    for word in text.split(' ') {
        if word.is_empty() {
            continue;
        }
        let chars: Vec<char> = word.chars().collect();
        if chars.len() <= avail {
            tokens.push(word.to_string());
        } else {
            for chunk in chars.chunks(avail) {
                tokens.push(chunk.iter().collect());
            }
        }
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for token in tokens {
        let extra = if current.is_empty() { 0 } else { 1 };
        if current.chars().count() + extra + token.chars().count() > avail {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
        } else if extra == 1 {
            current.push(' ');
        }
        current.push_str(&token);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// 折行后逐行输出，续行悬挂对齐到 `NOTE_INDENT`。`tone` 为 `None` 时不上色
/// （C4 成因句：原型里它是决定性说明，不是次级提示，不降 dim）。
fn render_wrapped(out: &mut String, text: &str, style: &Style, tone: Option<Tone>) {
    for line in wrap_lines(text) {
        match tone {
            Some(t) => {
                let _ = writeln!(out, "{NOTE_INDENT}{}", style.tone(t, &line));
            }
            None => {
                let _ = writeln!(out, "{NOTE_INDENT}{line}");
            }
        }
    }
}

fn render_card(out: &mut String, card: &Card, style: &Style, verbose: bool) {
    let mut head = format!(
        "{} {}  {}",
        style.tone(card.tone, marker(card.tone)),
        style.tone(Tone::Dim, card.id.as_str()),
        style.bold(card.title),
    );
    if let Some(state) = &card.state {
        let _ = write!(head, "  {}", style.tone(card.state_tone, state));
    }
    let _ = writeln!(out, "  {head}");

    for value in &card.values {
        let _ = writeln!(out, "{NOTE_INDENT}{value}");
    }
    for note in &card.notes {
        render_wrapped(out, note, style, Some(Tone::Dim));
    }
    if let Some(fix) = &card.fix {
        render_wrapped(out, &fix.explain, style, None);
        let _ = writeln!(out, "{NOTE_INDENT}{}", fix.command);
    }
    if verbose {
        render_wrapped(out, card.description, style, Some(Tone::Dim));
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
        // 失败卡不编造状态词/比对符/修复行——brief 要点 7。
        state: None,
        state_tone: Tone::Dim,
        values: vec![failure_text(failure, text).to_string()],
        notes: if failure == Failure::QuotaExhausted {
            vec![text.notes.quota_shared]
        } else {
            Vec::new()
        },
        fix: None,
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

    let fields = &text.checks.o1_fields;
    let mut values = vec![format!("{}  {}", fields.address, info.ip)];
    let mut notes = Vec::new();
    if let Some(geo) = &info.geo {
        let place: Vec<&str> = [&geo.city_name, &geo.region_name, &geo.country_name]
            .into_iter()
            .filter_map(|f| f.as_deref())
            .collect();
        if !place.is_empty() {
            values.push(format!("{}  {}", fields.ownership, place.join(", ")));
        }
        let network: Vec<&str> = [&geo.asn, &geo.organisation]
            .into_iter()
            .filter_map(|f| f.as_deref())
            .collect();
        if !network.is_empty() {
            values.push(format!("{}  {}", fields.network, network.join("  ")));
        }
        // 契约 5.4：必须标明归属来自 proxycheck，否则用户拿两边结果对不上时
        // 会以为有一边算错了。
        notes.push(text.notes.geo_source);
    } else {
        values.push(format!("{}  {}", fields.ownership, text.values.unknown));
    }

    Card {
        id: CheckId::O1,
        tone: Tone::Ok,
        title: meta.title,
        // O1 没有 ok/warn/bad 分支，「已取得」是唯一状态——它是提示词，不是
        // 契约 6 的分项评价，固定用 Dim（design：`.cstate.dim`）。
        state: Some(text.values.obtained.to_string()),
        state_tone: Tone::Dim,
        values,
        notes,
        fix: None,
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

    let local = check.local.as_deref().unwrap_or(text.values.unknown);
    let exit = check.exit.as_deref().unwrap_or(text.values.unknown);
    // 比对结果为一致/不一致才用 =/≠；「无从比对」（matches: None）两者都不用，
    // 借用既有的 Dim marker「·」表示中性分隔（brief 要点 3）——这里的取值不
    // 上色，与其余 `values` 行的既有约定一致（见文件顶部「说明」的着色决策）。
    let op = match check.matches {
        Some(true) => "=",
        Some(false) => "≠",
        None => marker(Tone::Dim),
    };
    let detail = format!("{local}  {op}  {exit}");

    // 只有 C4（`desktop_note == false`）命中不一致且出口 IP 时区名已知时才给
    // 修复建议（决策 5：只有 C4 给修复命令；spec §5.4 的「已知」条件显式核对，
    // 不依赖 `timezone::compare` 「Some 必然双值齐全」这条隐含不变量）。
    let fix = if !desktop_note && check.matches == Some(false) {
        check.exit.as_deref().map(|exit_tz| {
            let cf = &text.checks.c4_fix;
            CardFix {
                explain: format!(
                    "{}{local}{}{exit_tz}{}",
                    cf.explain_prefix, cf.explain_connector, cf.explain_suffix
                ),
                command: format!("{}  {}{exit_tz}", cf.fix_label, cf.fix_command_prefix),
            }
        })
    } else {
        None
    };

    Card {
        id,
        tone,
        title: meta.title,
        state: Some(label.to_string()),
        state_tone: tone,
        values: vec![detail],
        notes: if desktop_note {
            // 契约 5.1：缺了这句话，CLI 用户会误以为自己的 $TZ 已被检查。
            vec![text.notes.o2_desktop_only]
        } else {
            Vec::new()
        },
        fix,
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

    let (tone, state, value) = match result {
        ipify::Ipv6::Leaked(addr) => (
            Tone::Warn,
            text.values.ipv6_leaked,
            format!("{}  {addr}", text.values.ipv6_leaked),
        ),
        ipify::Ipv6::Disabled => (
            Tone::Ok,
            text.values.ipv6_disabled,
            text.values.ipv6_disabled.to_string(),
        ),
        // Indeterminate 在组装时已被折成检测失败，走不到这里。
        ipify::Ipv6::Indeterminate => (
            Tone::Dim,
            text.values.unknown,
            text.values.unknown.to_string(),
        ),
    };

    Card {
        id: CheckId::O3,
        tone,
        title: meta.title,
        // 状态词复用现有 values 字段（brief 要点 2）：ipv6_disabled/ipv6_leaked
        // 已经是「未启用」/「泄露」这类短词，不需要 C2 另开一份同义文案。
        state: Some(state.to_string()),
        state_tone: tone,
        values: vec![value],
        notes: Vec::new(),
        fix: None,
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
    let level = verdict::risk_level(score);
    let (tone, level_word) = match level {
        Level::Low => (Tone::Ok, text.values.risk_level_low),
        Level::Medium => (Tone::Warn, text.values.risk_level_medium),
        Level::High => (Tone::Bad, text.values.risk_level_high),
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
        // 「{分数}/100 {分级词}」，分级词是不带「风险」后缀的短词，区别于
        // `verdict.low/medium/high`（design：「33/100 中」）。
        state: Some(format!("{score}/100 {level_word}")),
        state_tone: tone,
        values,
        notes: Vec::new(),
        fix: None,
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
    let (tone, state, mut values) = match &result.comparison {
        dns_egress::Comparison::Comparable {
            leak,
            ecs_country,
            exit_country,
        } => {
            let tone = if *leak { Tone::Warn } else { Tone::Ok };
            let op = if *leak { "≠" } else { "=" };
            let comparison_line = format!(
                "{}  {}  {op}  {}  {}",
                dt.ecs_label, ecs_country, dt.exit_label, exit_country,
            );
            let leak_message = if *leak { dt.leak } else { dt.no_leak };
            let state = if *leak {
                dt.state_leaked
            } else {
                dt.state_not_leaked
            };
            (tone, state, vec![comparison_line, leak_message.to_string()])
        }
        dns_egress::Comparison::NotComparable(reason) => {
            // 三种「无从比对」各自的说明，绝不回退成「泄露」或「未泄露」（契约 2.5 硬约束 3）。
            let message = match reason {
                dns_egress::NotComparable::NoEcs => dt.no_ecs,
                dns_egress::NotComparable::UnmappedCountry => dt.unmapped_country,
                dns_egress::NotComparable::UnknownExitCountry => dt.unknown_exit_country,
            };
            // 「无从比对」复用 O2/C4 已有的通用「无法比对」短词——它的字段名带
            // timezone 前缀，但取值本身是通用短语，不是时区专属内容，避免另开
            // 一份同义文案（brief 要点 2）。
            (
                Tone::Dim,
                text.values.timezone_indeterminate,
                vec![message.to_string()],
            )
        }
    };

    // resolver 归属始终展示（与 §5.1 里 CLI 同时展示 $TZ 与系统时区同构），
    // 挂一个「仅供参考」pill 在值旁边（原型 refs/cli-report-redesign.html:392-394
    // 的位置关系）；notes 里的 resolver_note 整句保留不动——两者并存是原型的
    // 设计，不是待消除的重复：pill 挂在值本身供扫读，note 是解释句供细读。
    values.push(format!(
        "{}  {}  {}",
        dt.resolver_label,
        result
            .resolver_geo
            .as_deref()
            .unwrap_or(text.values.unknown),
        text.values.reference_only,
    ));

    Card {
        id: CheckId::O5,
        tone,
        title: meta.title,
        state: Some(state.to_string()),
        state_tone: tone,
        values,
        notes: vec![dt.resolver_note],
        fix: None,
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
    let (tone, state, values) = match result {
        udp_egress::UdpEgress::Comparable {
            mismatch,
            reflexive_ip,
            exit_ip,
        } => {
            let tone = if *mismatch { Tone::Warn } else { Tone::Ok };
            let op = if *mismatch { "≠" } else { "=" };
            let comparison_line = format!(
                "{}  {}  {op}  {}  {}",
                ut.reflexive_label, reflexive_ip, ut.exit_label, exit_ip,
            );
            let mismatch_message = if *mismatch {
                ut.mismatch
            } else {
                ut.no_mismatch
            };
            let state = if *mismatch {
                ut.state_mismatch
            } else {
                ut.state_match
            };
            (
                tone,
                state,
                vec![comparison_line, mismatch_message.to_string()],
            )
        }
        udp_egress::UdpEgress::NotComparable(reason) => {
            // 「无从比对」的三种成因与「未命中」文案可分（契约 2.6）。
            let message = match reason {
                udp_egress::NotComparable::FamilyMismatch => ut.family_mismatch,
                udp_egress::NotComparable::UnknownExitIp => ut.unknown_exit_ip,
                udp_egress::NotComparable::StunDisagree => ut.stun_disagree,
            };
            // 同 O5：复用 O2/C4 已有的通用「无法比对」短词。
            (
                Tone::Dim,
                text.values.timezone_indeterminate,
                vec![message.to_string()],
            )
        }
    };

    Card {
        id: CheckId::O6,
        tone,
        title: meta.title,
        state: Some(state.to_string()),
        state_tone: tone,
        values,
        notes: Vec::new(),
        fix: None,
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
            // 同 O1：没有 ok/warn/bad 分支，「已取得」是提示词而非评价，固定 Dim。
            state: Some(text.values.obtained.to_string()),
            state_tone: Tone::Dim,
            values: vec![ip.clone()],
            notes: Vec::new(),
            fix: None,
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

    // 状态词：数量 + 命中时追加 dns_domestic。C2 未提供「未见国内 DNS」这个否定
    // 短语（留给本任务判断，见 report）——没有对应词就不编造，非命中时只显示
    // 数量，不是省略语义，是没有可用的既有文案可以拼出这句否定短语。
    let count = servers.len();
    let state = if domestic {
        format!("{count} · {}", text.values.dns_domestic)
    } else {
        count.to_string()
    };

    Card {
        id: CheckId::C2,
        // 国内 DNS 只是提醒，不进综合结论（契约 2.1）。
        tone: if domestic { Tone::Warn } else { Tone::Ok },
        title: meta.title,
        state: Some(state),
        state_tone: if domestic { Tone::Warn } else { Tone::Ok },
        values,
        notes: Vec::new(),
        fix: None,
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

    let tone = match status.tun_off() {
        Some(true) => Tone::Warn,
        Some(false) => Tone::Ok,
        // 未知不贡献信号，也不该染成警告色。
        None => Tone::Dim,
    };

    Card {
        id: CheckId::C3,
        tone,
        title: meta.title,
        // 状态词取 TUN/VPN 这一路——它是 tone（也是 contributing_ids 的
        // tun_off 信号）唯一依据的通道，env/system 不影响判定（design：
        // 「TUN/VPN 已开启」）。
        state: Some(format!(
            "{}  {}",
            text.values.proxy_tun,
            state_label(&status.tun)
        )),
        state_tone: tone,
        values,
        notes: Vec::new(),
        fix: None,
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

    /// 说明文字现在按 76 列折行、悬挂缩进（C4），原本整句的 `contains` 断言
    /// 会被拆到多行而失配。这里把渲染输出按行拼回一整段（去掉每行的悬挂缩进），
    /// 用于校验被折行打散的说明文字整体是否仍然完整存在——只在英文断言里使用，
    /// 英文按空格折行，拼回时补单空格能精确复原原文；中文长句会被强制按字符切分
    /// （无空格可依），拼回会插入原文没有的空格，因此不适用于中文断言。
    fn dewrap(out: &str) -> String {
        out.lines()
            .map(|line| line.strip_prefix(NOTE_INDENT).unwrap_or(line))
            .collect::<Vec<_>>()
            .join(" ")
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
        assert!(!dewrap(&render(&blank(), false, false)).contains(text.checks.o1.description));
        assert!(dewrap(&render(&blank(), false, true)).contains(text.checks.o1.description));
    }

    #[test]
    fn quota_exhausted_tells_the_user_how_to_fix_it() {
        let mut report = blank();
        report.o4 = Outcome::Failed(Failure::QuotaExhausted);
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);
        assert!(out.contains(text.failures.quota_exhausted));
        // 共享配额那句必须在——否则用户以为工具坏了。76 列折行会把它拆到多行，
        // 拼回一整段再断言完整性。
        assert!(dewrap(&out).contains(text.notes.quota_shared));
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
        assert!(dewrap(&render(&report, false, false)).contains(text.notes.o2_desktop_only));
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
        assert!(dewrap(&render(&report, false, false)).contains(text.notes.geo_source));
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
        assert!(
            dewrap(&out).contains(text.dns_egress.resolver_note),
            "{out}"
        );
    }

    #[test]
    fn the_o5_card_shows_the_reference_only_pill_next_to_resolver_geo_and_keeps_the_note() {
        // 设计权威 refs/cli-report-redesign.html:392-394：pill 与整句 note 并存，
        // 不是同一件事的重复——pill 挂在值本身（扫读），note 是解释句（细读）。
        let mut report = blank();
        report.o5 = Outcome::Done(dns_egress::DnsEgress {
            resolver_geo: Some("Japan - Google LLC".into()),
            comparison: dns_egress::Comparison::Comparable {
                leak: false,
                ecs_country: "JP".into(),
                exit_country: "JP".into(),
            },
        });
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);
        // pill 绑定在 resolver 归属值这一行，具体文案字符串。
        assert!(
            out.contains(&format!(
                "{}  Japan - Google LLC  {}",
                text.dns_egress.resolver_label, text.values.reference_only
            )),
            "{out}"
        );
        // resolver_note 整句仍然在——两者并存，不是二选一。
        assert!(
            dewrap(&out).contains(text.dns_egress.resolver_note),
            "{out}"
        );
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
    fn low_score_but_abuse_listed_still_puts_o4_in_the_attention_list_as_contributing() {
        // O4 卡片的 tone 只读 risk_score（契约 6），完全不读 abuse_listed；
        // 但 abuse_listed 是独立于分数的贡献信号（StopForumSpam vs proxycheck，
        // 两个解耦的第三方）。risk_score 10 → risk_level(10) = Low → tone = Ok，
        // 卡片是绿色的，但 abuse_listed: true 仍然把综合结论拉到 Full(Medium)。
        // 候选集合如果只看 tone，这张真的贡献了综合结论的卡会被完全漏掉。
        let mut report = blank();
        report.o4 = Outcome::Done(Risk {
            risk: crate::probe::proxycheck::Risk {
                network_type: None,
                proxy: false,
                vpn: false,
                tor: false,
                scraper: false,
                risk_score: 10,
                anonymous: false,
            },
            abuse: Some(crate::probe::stopforumspam::Abuse {
                listed: true,
                frequency: 3,
                last_seen: None,
            }),
        });
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);

        assert_eq!(report.verdict(), Verdict::Full(Level::Medium));
        assert!(out.contains(text.verdict.attention_label), "{out}");
        assert!(
            out.contains(&format!("O4 {}", text.verdict.attention_contributing)),
            "{out}"
        );
        assert!(
            !out.contains(&format!("O4 {}", text.verdict.attention_reminder_only)),
            "{out}"
        );
        // 清单里 O4 这一行不能显示卡片自己的绿色 ✔——一个标题写着「需关注」的
        // 清单里出现「这项没问题」的符号是逻辑矛盾。清单 marker 取「需要关注」
        // 的语义（bad 除外，这里没有 bad 项，统一按 warn 处理），不取卡片 tone。
        // 卡片自身的渲染（下面的检测卡列表）不受影响，仍按契约 6 显示真实 tone
        // （分数 10 分，risk_level 判 Low，绿色 ✔）——用 O1 卡片行的位置把两段
        // 切开，分别断言，而不是对整份输出做一次性 contains（O4 的「✔ ...」
        // 子串本就会因为卡片本身而存在，一次性断言会把两件事混在一起）。
        let cards_start = out.find("· O1").expect("O1 卡片必须存在");
        let attention_block = &out[..cards_start];
        let cards_block = &out[cards_start..];
        assert!(
            attention_block.contains("! O4  IP Type & Risk"),
            "list line should use the warn marker: {attention_block}"
        );
        assert!(
            !attention_block.contains("✔ O4  IP Type & Risk"),
            "list line must not reuse the card's own ok marker: {attention_block}"
        );
        assert!(
            cards_block.contains("✔ O4  IP Type & Risk"),
            "the card itself must still show its real (green) tone: {cards_block}"
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

    fn tz(local: &str, exit: Option<&str>, matches: Option<bool>) -> Outcome<TimezoneCheck> {
        Outcome::Done(TimezoneCheck {
            local: Some(local.into()),
            exit: exit.map(String::from),
            matches,
        })
    }

    #[test]
    fn o2_uses_equals_for_a_match_and_not_equals_for_a_mismatch() {
        let mut report = blank();
        report.o2 = tz("Asia/Shanghai", Some("Asia/Shanghai"), Some(true));
        assert!(
            render(&report, false, false).contains("Asia/Shanghai  =  Asia/Shanghai"),
            "{}",
            render(&report, false, false)
        );

        let mut report = blank();
        report.o2 = tz("Asia/Shanghai", Some("Asia/Tokyo"), Some(false));
        let out = render(&report, false, false);
        assert!(out.contains("Asia/Shanghai  ≠  Asia/Tokyo"), "{out}");
        assert!(!out.contains("Asia/Shanghai  =  Asia/Tokyo"), "{out}");
    }

    #[test]
    fn timezone_indeterminate_uses_neither_equals_nor_not_equals() {
        // 「无从比对」（matches: None）两个比对符都不该出现——走 Dim marker「·」。
        let mut report = blank();
        report.o2 = tz("Asia/Shanghai", None, None);
        let out = render(&report, false, false);
        assert!(!out.contains("  =  "), "{out}");
        assert!(!out.contains("  ≠  "), "{out}");
        assert!(out.contains("Asia/Shanghai  ·  unknown"), "{out}");
    }

    #[test]
    fn o5_and_o6_use_equals_for_a_match_and_not_equals_for_a_mismatch() {
        let mut report = blank();
        report.o5 = Outcome::Done(dns_egress::DnsEgress {
            resolver_geo: None,
            comparison: dns_egress::Comparison::Comparable {
                leak: false,
                ecs_country: "JP".into(),
                exit_country: "JP".into(),
            },
        });
        assert!(
            render(&report, false, false).contains("JP  =  "),
            "{}",
            render(&report, false, false)
        );

        let mut report = blank();
        report.o5 = Outcome::Done(dns_egress::DnsEgress {
            resolver_geo: None,
            comparison: dns_egress::Comparison::Comparable {
                leak: true,
                ecs_country: "JP".into(),
                exit_country: "US".into(),
            },
        });
        assert!(
            render(&report, false, false).contains("JP  ≠  "),
            "{}",
            render(&report, false, false)
        );

        let mut report = blank();
        report.o6 = Outcome::Done(udp_egress::UdpEgress::Comparable {
            mismatch: false,
            reflexive_ip: "1.2.3.4".parse().unwrap(),
            exit_ip: "1.2.3.4".parse().unwrap(),
        });
        assert!(
            render(&report, false, false).contains("1.2.3.4  =  Exit IP  1.2.3.4"),
            "{}",
            render(&report, false, false)
        );

        let mut report = blank();
        report.o6 = Outcome::Done(udp_egress::UdpEgress::Comparable {
            mismatch: true,
            reflexive_ip: "1.2.3.4".parse().unwrap(),
            exit_ip: "5.6.7.8".parse().unwrap(),
        });
        assert!(
            render(&report, false, false).contains("1.2.3.4  ≠  Exit IP  5.6.7.8"),
            "{}",
            render(&report, false, false)
        );
    }

    #[test]
    fn not_comparable_o5_and_o6_render_no_comparison_symbol() {
        let mut report = blank();
        report.o5 = Outcome::Done(dns_egress::DnsEgress {
            resolver_geo: None,
            comparison: dns_egress::Comparison::NotComparable(dns_egress::NotComparable::NoEcs),
        });
        report.o6 = Outcome::Done(udp_egress::UdpEgress::NotComparable(
            udp_egress::NotComparable::StunDisagree,
        ));
        let out = render(&report, false, false);
        assert!(!out.contains("  =  "), "{out}");
        assert!(!out.contains("  ≠  "), "{out}");
    }

    #[test]
    fn the_title_line_carries_a_right_hand_state_word() {
        // O3：状态词复用现有 values.ipv6_disabled，不是新造的同义词。
        let mut report = blank();
        report.o3 = Outcome::Done(ipify::Ipv6::Disabled);
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);
        assert!(
            out.contains(&format!("IPv6 Leak  {}", text.values.ipv6_disabled)),
            "{out}"
        );

        // O4：{分数}/100 {分级词}，不带「风险」后缀。33 分落在 26–75，判 medium。
        let report = risk_report(33, false);
        let out = render(&report, false, false);
        assert!(
            out.contains(&format!(
                "IP Type & Risk  33/100 {}",
                text.values.risk_level_medium
            )),
            "{out}"
        );
    }

    #[test]
    fn failed_cards_have_no_state_word() {
        // 失败卡不编造状态词——标题行后直接换行。
        let out = render(&blank(), false, false);
        let o1_start = out.find(" O1  ").expect("O1 必须存在");
        let o1_line_end = out[o1_start..].find('\n').unwrap() + o1_start;
        assert_eq!(
            out[o1_start..o1_line_end].trim(),
            "O1  Exit IP and Ownership",
            "{out}"
        );
    }

    #[test]
    fn c4_gives_a_fix_command_only_when_it_mismatches_with_a_known_exit_timezone() {
        let text = copy::text(Lang::En);

        // (a) C4 不一致 + 出口 IP 时区名已知 → 有修复行。
        let mut report = blank();
        report.c4 = tz("Asia/Shanghai", Some("Asia/Tokyo"), Some(false));
        let out = render(&report, false, false);
        assert!(out.contains(text.checks.c4_fix.fix_label), "{out}");
        assert!(out.contains("export TZ=Asia/Tokyo"), "{out}");

        // (b) C4 无从比对 → 无修复行，即使有本地时区名。
        let mut report = blank();
        report.c4 = tz("Asia/Shanghai", None, None);
        let out = render(&report, false, false);
        assert!(!out.contains(text.checks.c4_fix.fix_label), "{out}");
        assert!(!out.contains("export TZ="), "{out}");

        // (c) O2 不一致（非 C4）→ 无修复行——只有 C4 给修复命令（决策 5）。
        let mut report = blank();
        report.o2 = tz("Asia/Shanghai", Some("Asia/Tokyo"), Some(false));
        let out = render(&report, false, false);
        assert!(!out.contains(text.checks.c4_fix.fix_label), "{out}");
        assert!(!out.contains("export TZ="), "{out}");
    }

    #[test]
    fn long_notes_wrap_within_76_columns_with_a_hanging_indent() {
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
        let out = render(&report, false, false);
        // 只检查检测卡内被本任务折行的说明行（以 NOTE_INDENT 起始）——结论区的
        // 摘要句不属于 C4 的范围（render_verdict 是 C3 的地盘，本任务不动）。
        for line in out.lines().filter(|l| l.starts_with(NOTE_INDENT)) {
            assert!(line.chars().count() <= WRAP_WIDTH, "超出 76 列：{line:?}");
        }
        // geo_source（102 字符）必然被折成至少两行；续行悬挂缩进到 NOTE_INDENT，不顶格。
        assert!(
            out.contains("\n       database, so the two can disagree."),
            "{out}"
        );
    }

    #[test]
    fn o1_and_o2_pair_up_and_both_keep_their_contract_notes() {
        // 契约呈现约束抽查：O1 标明 geo_source，O2 标明只覆盖桌面应用（那句提醒
        // C4 才是命令行认的 $TZ）——O2/C4 双条展示，各自的 note 都要在。
        let mut report = blank();
        report.o1 = Outcome::Done(ExitInfo {
            ip: "212.50.249.204".into(),
            geo: Some(crate::probe::proxycheck::Geo {
                country_name: Some("Japan".into()),
                country_code: Some("JP".into()),
                region_name: Some("Osaka".into()),
                city_name: Some("Osaka".into()),
                timezone: Some("Asia/Tokyo".into()),
                asn: Some("AS25820".into()),
                organisation: Some("IT7 Networks Inc".into()),
                provider: None,
            }),
        });
        report.o2 = tz("Asia/Shanghai", Some("Asia/Tokyo"), Some(false));
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);
        assert!(dewrap(&out).contains(text.notes.geo_source), "{out}");
        assert!(dewrap(&out).contains(text.notes.o2_desktop_only), "{out}");
    }
}
