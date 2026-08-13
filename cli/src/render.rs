//! 呈现层：把一次体检渲染成人读的报告。
//!
//! 参考 Preflight Web 的视觉结构，**不用表格边框**：
//! 结论区置顶（用户敲完命令第一眼看到结论，不用滚），其下是 10 张检测卡。
//! 网页的顶部导航与落地内容不移植——终端没有锚点，营销文案对已装用户没有意义。
//!
//! 没有固定列宽常量：`ai-ipcheck` 的 `COL_LABEL = 20` 是「多语言不存在」这个假设的化石。

use std::fmt::Write as _;

use crate::copy::Text;
use crate::domain::checks::{CheckId, Coverage, Failure, Outcome};
use crate::domain::dns_servers::{Entry, Variant};
use crate::domain::verdict::{self, Level, PreliminaryLevel, Verdict};
use crate::domain::{dns_egress, udp_egress};
use crate::probe::{
    ExitInfo, RealIp, Report, Risk, TimezoneCheck, dns, dns_check, ipify, proxy, proxycheck,
};

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
    /// 整幅宽度（列）。窗口宽度已经在构造时夹进 `[MIN_WIDTH, MAX_WIDTH]`。
    width: usize,
}

impl Style {
    /// 固定宽度渲染：拿不到窗口宽度（重定向、CI、测试）时用——管道里的输出必须可复现。
    pub fn new(color: bool) -> Self {
        Self {
            color,
            width: DEFAULT_WIDTH,
        }
    }

    /// 跟随终端窗口宽度。夹在 `[MIN_WIDTH, MAX_WIDTH]`：再窄要保住标签列，
    /// 再宽也不摊成一整屏——原型 `.screen{min-width:84ch;max-width:96ch}` 同一立场。
    pub fn sized(color: bool, columns: usize) -> Self {
        Self {
            color,
            width: columns.clamp(MIN_WIDTH, MAX_WIDTH),
        }
    }

    /// 整幅宽度：发丝线、右对齐的状态词、取值行都按它排。
    fn width(&self) -> usize {
        self.width
    }

    /// 说明文字的宽度：整幅再宽也收在 `PROSE_WIDTH` 以内（原型 `.note{max-width:76ch}`）——
    /// 长行读起来费劲，这条上限与窗口无关。
    fn prose(&self) -> usize {
        self.width.min(PROSE_WIDTH)
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

    /// 结论档位的色块（原型 `.badge`：反白，档位色作底）。`--no-color` 下退回
    /// 原型同款的 `[ 中风险 ]`——方括号替色块承担「这是个徽章」的语义（点 8）。
    fn badge(&self, tone: Tone, body: &str) -> String {
        if !self.color {
            return format!("[ {body} ]");
        }
        let bg = match tone {
            Tone::Ok => "30;42",
            Tone::Warn => "30;43",
            Tone::Bad => "30;41",
            Tone::Dim => "30;47",
        };
        self.paint(bg, &format!(" {body} "))
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

/// 结论区的档位词、形态徽章与摘要句。
///
/// `has_reminder_only` = 「需关注」清单里存在仅提醒项。低档的默认摘要说「未发现异常」，
/// 与同屏的仅提醒清单自相矛盾（O2 系统时区、C2 国内 DNS 都是常态），因此低档两个形态
/// 各有一条专用摘要。判级逻辑一行不动——矛盾出在文案，不在判据。
fn verdict_headline<'a>(
    verdict: &Verdict,
    has_reminder_only: bool,
    text: &'a Text,
) -> (&'a str, Option<&'a str>, &'a str) {
    match verdict {
        Verdict::Insufficient => (
            text.verdict.insufficient,
            None,
            text.verdict.summary_insufficient,
        ),
        Verdict::Preliminary(PreliminaryLevel::Low) => (
            text.verdict.low,
            Some(text.verdict.preliminary_badge),
            if has_reminder_only {
                text.verdict.summary_preliminary_low_reminders
            } else {
                text.verdict.summary_preliminary_low
            },
        ),
        Verdict::Preliminary(PreliminaryLevel::Medium) => (
            text.verdict.medium,
            Some(text.verdict.preliminary_badge),
            text.verdict.summary_preliminary_medium,
        ),
        Verdict::Full(Level::Low) => (
            text.verdict.low,
            Some(text.verdict.full_badge),
            if has_reminder_only {
                text.verdict.summary_full_low_reminders
            } else {
                text.verdict.summary_full_low
            },
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

/// 卡片正文的一行（原型 `.cbody` 里的两种行）。
enum Row {
    /// 标签 + 取值。同一张卡里所有 `Kv`/`KvOwned` 行对齐到同一标签列（原型 `.kv`
    /// 网格），标签降 dim；取值折行时悬挂对齐到取值列，不回到标签列。
    Kv(&'static str, String),
    /// 标签本身是动态值（C2 的 DNS 服务器地址列），其余同 `Kv`。
    KvOwned(String, String),
    /// 比对行（原型 `.cmp`）：两侧各自带标签，中间是 `=`/`≠`。两个裸值读不出
    /// 谁是谁——O2/C4 的本地侧本就是两个不同的东西（系统时区 vs `$TZ`）。
    Cmp {
        left_label: &'static str,
        left: String,
        op: &'static str,
        right_label: &'static str,
        right: String,
    },
    /// 整行文本：整句说明，不参与标签列对齐。
    Plain(String),
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
    values: Vec<Row>,
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
    /// 修复行的标签（「建议」），与卡内标签列同样降 dim。
    label: &'static str,
    command: String,
}

/// `preflight dns` 的表格视图。
pub fn dns_table(
    entries: &[Entry],
    results: Option<&[dns_check::CheckResult]>,
    text: &Text,
    style: &Style,
) -> String {
    let has_check = results.is_some();
    let width = style.width();

    // 窄屏判定：估算展开后的宽度，超了就把地区折进提供商列。
    let longest_ip = entries
        .iter()
        .map(|e| display_width(&e.ip))
        .max()
        .unwrap_or(0);
    let longest_name = entries
        .iter()
        .map(|e| display_width(&e.name))
        .max()
        .unwrap_or(0);
    let unfolded_est = longest_ip
        + 2
        + longest_name
        + 2
        + 2
        + 2
        + display_width(text.dns_cmd.col_domestic)
        + 2
        + display_width(text.dns_cmd.variant_standard);
    let fold_region = unfolded_est > width;

    // 排序索引（--check 时按延迟升序，不通的排末尾）。
    let indices: Vec<usize> = if let Some(results) = results {
        let mut idx: Vec<usize> = (0..entries.len()).collect();
        idx.sort_by_key(|&i| {
            let r = &results[i];
            match r.status {
                dns_check::Status::Ok | dns_check::Status::Suspicious => {
                    (0usize, r.latency.map(|d| d.as_millis()))
                }
                dns_check::Status::Unreachable => (1, Some(u128::MAX)),
            }
        });
        idx
    } else {
        (0..entries.len()).collect()
    };

    // 构建 Matrix（header + data rows）。
    let mut matrix: Vec<Vec<String>> = Vec::new();

    // --- header ---
    let mut hdr = Vec::new();
    hdr.push(text.dns_cmd.col_ip.to_string());
    hdr.push(text.dns_cmd.col_provider.to_string());
    if !fold_region {
        hdr.push(text.dns_cmd.col_region.to_string());
    }
    hdr.push(text.dns_cmd.col_domestic.to_string());
    hdr.push(text.dns_cmd.col_variant.to_string());
    if has_check {
        hdr.push(text.dns_cmd.col_latency.to_string());
        hdr.push(text.dns_cmd.col_status.to_string());
    }
    matrix.push(hdr);

    // --- data ---
    for &i in &indices {
        let entry = &entries[i];
        let mut row = Vec::new();

        // IP
        row.push(entry.ip.clone());

        // Provider（折叠时把地区折进来）
        row.push(if fold_region {
            format!("{} ({})", entry.name, entry.region)
        } else {
            entry.name.clone()
        });

        // Region（折叠时省略）
        if !fold_region {
            row.push(entry.region.clone());
        }

        // Domestic
        row.push(if entry.domestic {
            text.dns_cmd.domestic_yes.to_string()
        } else {
            String::new()
        });

        // Variant
        row.push(variant_label(entry.variant, text).to_string());

        // Latency + Status（仅 --check）
        if has_check {
            let r = &results.unwrap()[i];
            row.push(match r.latency {
                Some(d) => format!("{} ms", d.as_millis()),
                None => "-".to_string(),
            });
            row.push(match r.status {
                dns_check::Status::Ok => style.tone(Tone::Ok, text.dns_cmd.check_ok),
                dns_check::Status::Suspicious => {
                    style.tone(Tone::Warn, text.dns_cmd.check_suspicious)
                }
                dns_check::Status::Unreachable => {
                    style.tone(Tone::Bad, text.dns_cmd.check_unreachable)
                }
            });
        }

        matrix.push(row);
    }

    // 列宽：取每列 display_width 的最大值。
    let num_cols = matrix[0].len();
    let col_widths: Vec<usize> = (0..num_cols)
        .map(|c| {
            matrix
                .iter()
                .map(|r| display_width(&r[c]))
                .max()
                .unwrap_or(0)
        })
        .collect();

    // 渲染。
    let mut out = String::new();
    let _ = writeln!(out);

    // 发丝线分隔表头。
    let sep = col_widths
        .iter()
        .map(|w| "─".repeat(*w))
        .collect::<Vec<_>>()
        .join("  ");
    let _ = writeln!(out, "  {sep}");

    for (row_idx, row) in matrix.iter().enumerate() {
        let cells: Vec<String> = (0..num_cols)
            .map(|c| pad_to(&row[c], col_widths[c]))
            .collect();
        let _ = writeln!(out, "  {}", cells.join("  "));

        // 表头后补一根发丝线。
        if row_idx == 0 {
            let _ = writeln!(out, "  {sep}");
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "  {}", style.tone(Tone::Dim, text.dns_cmd.footer_hint));
    let _ = writeln!(out);

    out
}

fn variant_label(v: Variant, text: &Text) -> &'static str {
    match v {
        Variant::Standard => text.dns_cmd.variant_standard,
        Variant::Security => text.dns_cmd.variant_security,
        Variant::Family => text.dns_cmd.variant_family,
        Variant::Adblock => text.dns_cmd.variant_adblock,
    }
}

pub fn report(report: &Report, text: &Text, style: &Style, verbose: bool) -> String {
    let verdict = report.verdict();
    let coverage = report.coverage();
    debug_assert!(coverage.is_complete(), "覆盖度不变量被破坏：{coverage:?}");

    let all_cards = cards(report, text, style);

    let mut out = String::new();
    let _ = writeln!(out);
    render_verdict(
        &mut out, &verdict, &coverage, report, &all_cards, text, style,
    );
    let _ = writeln!(out);

    // 分组只是呈现上的两段（联网可测 / 仅 CLI 可测），检测项顺序仍固定 O1→C4——
    // 编号是用户与文档引用的锚点（spec §5.2）。
    let (online, local) = all_cards.split_at(6);
    for (cards, group) in [(online, true), (local, false)] {
        render_group_header(&mut out, cards, group, text, style);
        let _ = writeln!(out);
        for card in cards {
            render_card(&mut out, card, style, verbose);
            let _ = writeln!(out);
        }
    }

    render_footer(&mut out, text, style);

    out
}

/// 分组标题：`联网检测  O1–O6 ────────  6 项 · 全部完成`（原型 `.group`）。
/// 发丝线把标题与右端的分组统计撑到同一行两端，占满整幅宽度。
fn render_group_header(out: &mut String, cards: &[Card], online: bool, text: &Text, style: &Style) {
    let g = &text.groups;
    let (first, last) = match (cards.first(), cards.last()) {
        (Some(first), Some(last)) => (first, last),
        _ => return,
    };
    let failed = cards.iter().filter(|c| c.tone == Tone::Dim).count();
    let name = if online { g.online } else { g.local };
    // 联网组的右端说的是「这组测完了没有」，本机组说的是「网页版做不到」——
    // 后者是这组存在的理由，覆盖度里已经报过失败数，不在这里报第二遍。
    let tail = if online && failed > 0 {
        format!("{} {failed}", text.coverage.failed)
    } else if online {
        g.all_done.to_string()
    } else {
        g.local_only.to_string()
    };
    let range = format!("{}–{}", first.id.as_str(), last.id.as_str());
    let meta = format!("{} {} · {tail}", cards.len(), g.items);

    let left = format!("  {name}  {}", style.tone(Tone::Dim, &range));
    let rule_width = style
        .width()
        .saturating_sub(display_width(&left) + display_width(&meta) + 2);
    if rule_width >= 1 {
        let _ = writeln!(
            out,
            "{left} {} {}",
            style.tone(Tone::Dim, &"─".repeat(rule_width)),
            style.tone(Tone::Dim, &meta)
        );
    } else {
        // 窄窗里连一格发丝线都排不下：统计换行右对齐，与标题行的状态词同样处理。
        let _ = writeln!(out, "{left}");
        let indent = style.width().saturating_sub(display_width(&meta)).max(2);
        let _ = writeln!(
            out,
            "{}{}",
            " ".repeat(indent),
            style.tone(Tone::Dim, &meta)
        );
    }
}

/// 页脚提示行（design：`.footer-hint`）。命令字面量以 `main.rs` 的 `Cli`/
/// `Command`/`ConfigAction`/`SettableKey` 定义为准：`--verbose`/`--json` 是
/// 顶层 flag，`config set proxycheck-key` 是 `SettableKey` 目前唯一枚举的键。
/// 只在 `render::report` 里调用——`--json` 走 `json::report`，完全不经过这里，
/// 提示行天然不会混进机器可读输出。
fn render_footer(out: &mut String, text: &Text, style: &Style) {
    let f = &text.footer;
    // 先上色再折行：`render_wrapped` 的宽度计算跳过转义序列（`display_width`），
    // 提示词的 dim 与命令字面量的常色都得以保留。
    render_wrapped(
        out,
        &format!(
            "preflight --verbose  {}",
            style.tone(Tone::Dim, f.verbose_hint)
        ),
        style,
        None,
        FOOTER_INDENT,
        style.width(),
    );
    render_wrapped(
        out,
        &format!("preflight --json  {}", style.tone(Tone::Dim, f.json_hint)),
        style,
        None,
        FOOTER_INDENT,
        style.width(),
    );
    render_wrapped(
        out,
        &format!(
            "preflight config set proxycheck-key  {}",
            style.tone(Tone::Dim, f.quota_hint)
        ),
        style,
        None,
        FOOTER_INDENT,
        style.width(),
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
    // 摘要句要知道清单里有没有仅提醒项，因此候选集合先算——`render_attention`
    // 复用同一份结果，不重算第二遍（两遍算法迟早分叉）。
    let (items, contributing) = attention_items(all_cards, report, verdict);
    let has_reminder_only = items.iter().any(|c| !contributing.contains(&c.id));
    let (level, badge, summary) = verdict_headline(verdict, has_reminder_only, text);

    let level_badge = style.badge(tone, level);
    match badge {
        // 形态徽章（初步／完整）跟在档位色块后面；窄窗里挤不下就落到下一行，
        // 不折断色块本身。
        Some(badge)
            if 2 + display_width(&level_badge) + 2 + display_width(badge) <= style.width() =>
        {
            let _ = writeln!(out, "  {level_badge}  {}", style.tone(Tone::Dim, badge));
        }
        Some(badge) => {
            let _ = writeln!(out, "  {level_badge}");
            render_wrapped(
                out,
                badge,
                style,
                Some(Tone::Dim),
                VERDICT_INDENT,
                style.prose(),
            );
        }
        None => {
            let _ = writeln!(out, "  {level_badge}");
        }
    }
    render_wrapped(out, summary, style, None, VERDICT_INDENT, style.prose());

    // facts 网格：四行共用一个标签列（原型 `.facts`），标签 dim、取值常色。
    let mut facts: Vec<(&str, String)> = Vec::new();

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
        facts.push((text.verdict.exit_ip_label, parts.join("  ·  ")));
    }

    if let Outcome::Done(risk) = &report.o4 {
        let score = risk.risk.risk_score;
        let bar_tone = match verdict::risk_level(score) {
            Level::Low => Tone::Ok,
            Level::Medium => Tone::Warn,
            Level::High => Tone::Bad,
        };
        facts.push((
            text.verdict.risk_label,
            format!(
                "{}  {}  {}",
                style.tone(bar_tone, &format!("{score}/100")),
                risk_bar(score, bar_tone, style),
                style.tone(Tone::Dim, text.values.risk_scale_note)
            ),
        ));
    }

    // 综合结论永远不得脱离覆盖度单独呈现（ADR-0004）。
    facts.push((
        text.verdict.coverage_label,
        format!(
            "{} {} · {} {}",
            text.coverage.done, coverage.done, text.coverage.failed, coverage.failed
        ),
    ));

    // 「需关注」与 facts 共用同一个标签列——它在原型里就是 facts 网格的第四行，
    // 单独算宽度会让标签列在这一行错位。
    let attention_width = if items.is_empty() {
        0
    } else {
        display_width(text.verdict.attention_label)
    };
    let label_width = facts
        .iter()
        .map(|(label, _)| display_width(label))
        .max()
        .unwrap_or(0)
        .max(attention_width);
    let value_indent = format!("{VERDICT_INDENT}{}", " ".repeat(label_width + 2));
    for (label, value) in &facts {
        let first = format!(
            "{VERDICT_INDENT}{}  ",
            style.tone(Tone::Dim, &pad_to(label, label_width))
        );
        render_wrapped_after(
            out,
            value,
            style,
            None,
            &first,
            &value_indent,
            style.width(),
        );
    }

    render_attention(
        out,
        &items,
        &contributing,
        label_width,
        &value_indent,
        text,
        style,
    );
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

/// 「需关注」清单的候选集合，以及其中实际贡献综合结论的检测项。
///
/// 判据是「信号是否实际被 compute() 消费并命中」，不是卡片 tone——一张
/// Warn/Bad 卡可能来自契约 2.1 明列的纯提醒信号。不在这里重算 51/76 阈值：
/// O4 是否贡献综合结论，直接读 compute() 的结果（只有 O4 的风险分能把结论
/// 判到 Full(High)，这是判级契约 3.2 的不变量，不是这里现算的）。
fn attention_items<'a>(
    all_cards: &'a [Card],
    report: &Report,
    verdict: &Verdict,
) -> (Vec<&'a Card>, Vec<CheckId>) {
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

    (items, contributing)
}

/// 「需关注」清单从卡片 tone 派生（spec 5.2），不写死检测项列表。
/// 无 warn/bad 项时整块（含 attention_scope 句）不出。
fn render_attention(
    out: &mut String,
    items: &[&Card],
    contributing: &[CheckId],
    label_width: usize,
    value_indent: &str,
    text: &Text,
    style: &Style,
) {
    if items.is_empty() {
        return;
    }

    let mut entries: Vec<String> = Vec::new();
    for card in items {
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
        entries.push(format!(
            "{} {}  {}",
            style.tone(list_tone, marker(list_tone)),
            style.tone(Tone::Dim, card.id.as_str()),
            card.title,
        ));
    }

    // 一项一行（**偏离原型**：`.attn` 是 flex wrap 横排）。横排在项数多时要靠
    // marker 去辨认项与项的边界，一行扫过去认不清几项；一行一项，编号列直接对齐。
    // 首行接在「需关注」标签后面，其余行落在取值列上；窄窗里单项装不下就自己折行。
    for (i, entry) in entries.iter().enumerate() {
        if i == 0 {
            let first = format!(
                "{VERDICT_INDENT}{}  ",
                style.tone(
                    Tone::Dim,
                    &pad_to(text.verdict.attention_label, label_width)
                )
            );
            render_wrapped_after(out, entry, style, None, &first, value_indent, style.width());
        } else {
            render_wrapped(out, entry, style, None, value_indent, style.width());
        }
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
        // scope 句留在取值列内（原型里它是 facts 网格的空标签行），不回到标签列。
        render_wrapped(
            out,
            &scope,
            style,
            Some(Tone::Dim),
            value_indent,
            style.prose(),
        );
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
/// 分隔符与连接词都**自带空格、原样拼接**——与 `C4FixText` 的 `explain_connector`
/// 同一约定。同一份 Copy 里不并存两套隐含空格规则：一处归一、一处裸拼，改文案的人
/// 无从判断自己该不该带空格。
fn join_ids(ids: &[CheckId], separator: &str, connector: &str) -> String {
    let strs: Vec<&str> = ids.iter().map(|id| id.as_str()).collect();
    match strs.split_last() {
        None => String::new(),
        Some((last, [])) => (*last).to_string(),
        Some((last, rest)) => format!("{}{connector}{last}", rest.join(separator)),
    }
}

/// 检测卡内的取值与说明文字（`values`/`note`/`description`/C4 成因句）用的缩进——
/// 折行后的续行悬挂对齐到同一列，不顶格。8 列 = 卡片缩进 2 + marker 1 + 空格 1 +
/// 编号 2 + 空格 2，正好落在标题的起始列上（原型 `.cbody` 与 `.ctitle` 同一条缩进线）。
const NOTE_INDENT: &str = "        ";
/// 结论区正文（摘要句、`attention_scope` 句）的缩进。
const VERDICT_INDENT: &str = "    ";
/// 页脚提示行的缩进。
const FOOTER_INDENT: &str = "  ";
/// 说明文字收进的总列宽（含缩进），spec §5.5。窗口再宽也不放开。
const PROSE_WIDTH: usize = 76;
/// 整幅宽度上限：取值行、发丝线、状态词右对齐用。
///
/// 原型是 96（`.screen{max-width:96ch}`），这里放宽到 110——常见的 120 列窗口
/// 基本铺满，右侧不留一条空白带。**上限本身不能取消**：整幅宽度决定的是标题行到
/// 右端状态词的视线距离，超宽屏上拉到 150 列，`✔ O3  IPv6 泄露 … 泄露` 就得横扫
/// 整屏才对得上。说明文字另有 `PROSE_WIDTH`，不受这条影响。
const MAX_WIDTH: usize = 110;
/// 整幅宽度下限：再窄的窗口也按这个排，否则标签列会被压没。
const MIN_WIDTH: usize = 40;
/// 拿不到窗口宽度时的固定宽度。
const DEFAULT_WIDTH: usize = PROSE_WIDTH;

/// 单个字符占的终端列数。CJK 全角字符占两列——不认这一点，中文行会折在 152 列
/// （屏幕外），标签列也对不齐（「地址」4 列 vs「风险评分」8 列，按字符数算都是 2/4）。
/// 只区分「宽/非宽」，不处理组合字符与 emoji 变体：报告里不出现这类字符。
fn char_width(ch: char) -> usize {
    let c = ch as u32;
    let wide = matches!(c,
        0x1100..=0x115F
        | 0x2E80..=0x303E
        | 0x3041..=0x33FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xA000..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x20000..=0x3FFFD);
    if wide { 2 } else { 1 }
}

/// 屏幕宽度（列数），跳过 ANSI 转义序列——`\x1b[2m` 这类控制串不占列。
/// 页脚是先上色再折行的（提示词 dim、命令字面量不 dim），拿原始字符数去折
/// 会把彩色输出提前折断。
fn display_width(text: &str) -> usize {
    let mut width = 0;
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            width += char_width(ch);
        }
    }
    width
}

/// 右补空格到 `width` 列（标签列对齐用）。已经超宽就原样返回。
fn pad_to(text: &str, width: usize) -> String {
    let mut out = text.to_string();
    for _ in display_width(text)..width {
        out.push(' ');
    }
    out
}

/// 按列宽折行，收进 `width - indent` 列内。断行机会有两处：词间空格，以及
/// 全角字符两侧——中文不用空格分词，只认空格的话「归属数据来自 proxycheck；网页版…」
/// 会被当成两个巨词，行尾大片留白（原型里 CSS 是按字断的）。
///
/// 词间空格**按原样保留**：取值行拿双空格做列内对齐（`Address  1.2.3.4`），
/// 归一成单空格等于把对齐拆了。
fn wrap_lines(text: &str, indent: &str, width: usize) -> Vec<String> {
    let avail = width.saturating_sub(display_width(indent)).max(1);
    let tokens = break_units(text);

    let mut lines = Vec::new();
    let mut current = String::new();
    for (word, spaces) in tokens {
        let chunks: Vec<String> = if display_width(&word) > avail {
            split_by_width(&word, avail)
        } else {
            vec![word]
        };
        for chunk in chunks {
            // `current` 的尾部空格已计入宽度，正是词与词之间的那一格。
            if !current.is_empty() && display_width(&current) + display_width(&chunk) > avail {
                let line = current.trim_end().to_string();
                lines.push(line);
                current.clear();
            }
            current.push_str(&chunk);
        }
        current.push_str(&spaces);
    }
    let last = current.trim_end();
    if !last.is_empty() {
        lines.push(last.to_string());
    }
    lines
}

/// 把一段文本切成「可断单元 + 其后的空格串」。单元内部不允许断行：
/// 连续的半角字符是一个单元（英文单词、IP、命令），每个全角字符自成单元。
/// ANSI 转义序列不占列，粘在**后一个**单元的前面——否则断行会把 dim 的起始
/// 序列留在上一行，颜色跨行泄漏。
/// 不能出现在行首的全角标点（句读、收尾括号引号）。
fn no_line_start(ch: char) -> bool {
    matches!(
        ch,
        '。' | '，'
            | '、'
            | '；'
            | '：'
            | '？'
            | '！'
            | '”'
            | '’'
            | '」'
            | '』'
            | '）'
            | '】'
            | '》'
            | '〉'
            | '…'
            | '·'
            | '～'
    )
}

/// 不能出现在行尾的全角标点（起始括号引号）。
fn no_line_end(ch: char) -> bool {
    matches!(ch, '“' | '‘' | '「' | '『' | '（' | '【' | '《' | '〈')
}

fn break_units(text: &str) -> Vec<(String, String)> {
    let mut units: Vec<(String, String)> = Vec::new();
    let mut current = String::new();
    let mut pending = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            pending.push(ch);
            for next in chars.by_ref() {
                pending.push(next);
                if next == 'm' {
                    break;
                }
            }
        } else if ch == ' ' {
            if !current.is_empty() {
                units.push((std::mem::take(&mut current), String::new()));
            }
            // 行首空格没有可挂靠的单元，直接丢掉——折行结果本就顶格。
            if let Some(last) = units.last_mut() {
                last.1.push(' ');
            }
        } else if char_width(ch) == 2 {
            // 标点禁则：句读不能顶行首、开括号不能落行尾——少了它，
            // 「…代理背后」后面会孤零零折出一行「。」。
            let after_open = current.chars().last().is_some_and(no_line_end);
            if no_line_start(ch) || after_open {
                if current.is_empty()
                    && let Some(last) = units.last_mut()
                    && last.1.is_empty()
                {
                    last.0.push_str(&std::mem::take(&mut pending));
                    last.0.push(ch);
                    continue;
                }
                current.push_str(&std::mem::take(&mut pending));
                current.push(ch);
                // 句读粘完就闭合单元；开括号继续张着，等下一个字符粘上来。
                if !no_line_end(ch) {
                    units.push((std::mem::take(&mut current), String::new()));
                }
                continue;
            }
            if !current.is_empty() {
                units.push((std::mem::take(&mut current), String::new()));
            }
            let mut unit = std::mem::take(&mut pending);
            unit.push(ch);
            if no_line_end(ch) {
                current = unit;
            } else {
                units.push((unit, String::new()));
            }
        } else {
            current.push_str(&std::mem::take(&mut pending));
            current.push(ch);
        }
    }
    current.push_str(&pending);
    if !current.is_empty() {
        units.push((current, String::new()));
    }
    units
}

/// 按列宽把一个无空格可断的长「词」切成多段（超长 URL/ASN 串走这条路）。
fn split_by_width(word: &str, avail: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut width = 0;
    for ch in word.chars() {
        let w = char_width(ch);
        if width + w > avail && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
            width = 0;
        }
        current.push(ch);
        width += w;
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// 折行后逐行输出，续行悬挂对齐到 `indent`。`tone` 为 `None` 时不上色
/// （C4 成因句：原型里它是决定性说明，不是次级提示，不降 dim）。
fn render_wrapped(
    out: &mut String,
    text: &str,
    style: &Style,
    tone: Option<Tone>,
    indent: &str,
    width: usize,
) {
    render_wrapped_after(out, text, style, tone, indent, indent, width);
}

/// 同 `render_wrapped`，但首行改用 `first`（标签列已经写在里面）。`first` 与
/// `indent` 必须等宽——取值行的折行宽度是按 `indent` 算的，首行更窄就会越界。
fn render_wrapped_after(
    out: &mut String,
    text: &str,
    style: &Style,
    tone: Option<Tone>,
    first: &str,
    indent: &str,
    width: usize,
) {
    let lines = wrap_lines(text, indent, width);
    if lines.is_empty() {
        let _ = writeln!(out, "{}", first.trim_end());
        return;
    }
    for (i, line) in lines.iter().enumerate() {
        let prefix = if i == 0 { first } else { indent };
        match tone {
            Some(t) => {
                let _ = writeln!(out, "{prefix}{}", style.tone(t, line));
            }
            None => {
                let _ = writeln!(out, "{prefix}{line}");
            }
        }
    }
}

fn render_card(out: &mut String, card: &Card, style: &Style, verbose: bool) {
    let head = format!(
        "  {} {}  {}",
        style.tone(card.tone, marker(card.tone)),
        style.tone(Tone::Dim, card.id.as_str()),
        style.bold(card.title),
    );
    match &card.state {
        // 状态词右端对齐（原型 `.ctitle{flex:1}` 把它顶到行尾）：扫一列就能读完
        // 十项结果。标题过长挤不下时退回两格间隔，不折行——状态词是扫读锚点。
        Some(state) => {
            let room = style
                .width()
                .saturating_sub(display_width(&head) + display_width(state));
            if room >= 2 {
                let _ = writeln!(
                    out,
                    "{head}{}{}",
                    " ".repeat(room),
                    style.tone(card.state_tone, state)
                );
            } else {
                // 窄窗里标题与状态词挤不下：状态词落到下一行，仍然右对齐——
                // 溢出到窗口外等于把它藏起来，那正是扫读要用的那一列。
                let _ = writeln!(out, "{head}");
                let indent = style
                    .width()
                    .saturating_sub(display_width(state))
                    .max(display_width(NOTE_INDENT));
                let _ = writeln!(
                    out,
                    "{}{}",
                    " ".repeat(indent),
                    style.tone(card.state_tone, state)
                );
            }
        }
        None => {
            let _ = writeln!(out, "{head}");
        }
    }

    // 卡内的标签列（原型 `.kv` 网格）：宽度取本卡最宽的标签，各卡自适应——
    // 固定列宽是「多语言不存在」那个假设的化石（见文件头）。
    let label_width = card
        .values
        .iter()
        .filter_map(|row| match row {
            Row::Kv(label, _) => Some(display_width(label)),
            Row::KvOwned(label, _) => Some(display_width(label)),
            Row::Cmp { .. } | Row::Plain(_) => None,
        })
        .max()
        .unwrap_or(0);
    let value_indent = format!("{NOTE_INDENT}{}", " ".repeat(label_width + 2));

    for row in &card.values {
        // 取值行同样折行：C2／C4／O5／O6 把整句说明放在 `values` 里，其中 O5 的
        // 泄露句是全报告最长的一行（原型改版点 05 点名的反面教材）。
        match row {
            Row::Kv(label, value) => {
                render_kv(out, label, value, label_width, &value_indent, style)
            }
            Row::KvOwned(label, value) => {
                render_kv(out, label, value, label_width, &value_indent, style)
            }
            Row::Cmp {
                left_label,
                left,
                op,
                right_label,
                right,
            } => {
                // 比对符取卡片自身的色调（原型 `.op` 挂状态类），标签降 dim。
                let line = format!(
                    "{}  {left}  {}  {}  {right}",
                    style.tone(Tone::Dim, left_label),
                    style.tone(card.tone, op),
                    style.tone(Tone::Dim, right_label),
                );
                render_wrapped(out, &line, style, None, NOTE_INDENT, style.width());
            }
            // 整句说明走说明宽度，取值行走整幅宽度——原型里也是两条不同的上限。
            Row::Plain(value) => {
                render_wrapped(out, value, style, None, NOTE_INDENT, style.prose())
            }
        }
    }
    for note in &card.notes {
        render_wrapped(
            out,
            note,
            style,
            Some(Tone::Dim),
            NOTE_INDENT,
            style.prose(),
        );
    }
    if let Some(fix) = &card.fix {
        render_wrapped(out, &fix.explain, style, None, NOTE_INDENT, style.prose());
        // 修复命令不折行——折断的命令复制粘贴过去就跑不了。窄窗里标签与命令
        // 同行放不下时，先出标签再出整条命令；命令本身宁可越界也不断开。
        let one_line = display_width(NOTE_INDENT) + display_width(fix.label) + 2
            <= style.width().saturating_sub(display_width(&fix.command));
        if one_line {
            let _ = writeln!(
                out,
                "{NOTE_INDENT}{}  {}",
                style.tone(Tone::Dim, fix.label),
                fix.command
            );
        } else {
            let _ = writeln!(out, "{NOTE_INDENT}{}", style.tone(Tone::Dim, fix.label));
            let _ = writeln!(out, "{NOTE_INDENT}{}", fix.command);
        }
    }
    if verbose {
        render_wrapped(
            out,
            card.description,
            style,
            Some(Tone::Dim),
            NOTE_INDENT,
            style.prose(),
        );
    }
}

/// 一条标签行：标签补齐到卡内标签列宽并降 dim，取值折行悬挂在取值列。
fn render_kv(
    out: &mut String,
    label: &str,
    value: &str,
    label_width: usize,
    value_indent: &str,
    style: &Style,
) {
    let first = format!(
        "{NOTE_INDENT}{}  ",
        style.tone(Tone::Dim, &pad_to(label, label_width))
    );
    render_wrapped_after(out, value, style, None, &first, value_indent, style.width());
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
        values: vec![Row::Plain(failure_text(failure, text).to_string())],
        notes: if failure == Failure::QuotaExhausted {
            vec![text.notes.quota_shared]
        } else {
            Vec::new()
        },
        fix: None,
        description,
    }
}

fn cards(report: &Report, text: &Text, style: &Style) -> Vec<Card> {
    vec![
        card_o1(&report.o1, text),
        card_timezone(CheckId::O2, &report.o2, text, &text.checks.o2, true),
        card_o3(&report.o3, text),
        card_o4(&report.o4, text, style),
        card_o5(&report.o5, text),
        card_o6(&report.o6, text),
        card_c1(&report.c1, text),
        card_c2(&report.c2, text),
        card_c3(&report.c3, text),
        card_timezone(CheckId::C4, &report.c4, text, &text.checks.c4, false),
    ]
}

/// 「地址 + 归属 + 网络」三行，O1 与 C1 共用——两处的归属同源（契约 1），
/// 展示形状也必须一致，否则用户没法逐行对照「真实 vs 出口」。
fn ip_with_geo_rows(
    ip: &str,
    geo: Option<&proxycheck::Geo>,
    source_note: &'static str,
    text: &Text,
) -> (Vec<Row>, Vec<&'static str>) {
    let fields = &text.checks.o1_fields;
    let mut values = vec![Row::Kv(fields.address, ip.to_string())];
    let mut notes = Vec::new();
    if let Some(geo) = geo {
        let place: Vec<&str> = [&geo.city_name, &geo.region_name, &geo.country_name]
            .into_iter()
            .filter_map(|f| f.as_deref())
            .collect();
        if !place.is_empty() {
            values.push(Row::Kv(fields.ownership, place.join(", ")));
        }
        let network: Vec<&str> = [&geo.asn, &geo.organisation]
            .into_iter()
            .filter_map(|f| f.as_deref())
            .collect();
        if !network.is_empty() {
            values.push(Row::Kv(fields.network, network.join("  ")));
        }
        // 契约 5.4：必须标明归属来自 proxycheck，否则用户拿两边结果对不上时
        // 会以为有一边算错了。
        notes.push(source_note);
    } else {
        values.push(Row::Kv(fields.ownership, text.values.unknown.to_string()));
    }
    (values, notes)
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

    let (values, notes) =
        ip_with_geo_rows(&info.ip, info.geo.as_ref(), text.notes.geo_source, text);

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
    // 比对行两侧都带标签（原型 `.cmp`）：两个裸时区名读不出谁是谁，而 O2/C4 的
    // 本地侧本就是两个不同的东西（系统时区 vs 命令行进程认的 $TZ）。
    // C4 的本地侧标签是字面量 `$TZ`——shell 变量名不随语种变化，同 `render_footer`
    // 里的命令字面量，不进 Copy。
    let local_label = if desktop_note {
        text.values.tz_system_label
    } else {
        "$TZ"
    };
    let detail = Row::Cmp {
        left_label: local_label,
        left: local.to_string(),
        op,
        right_label: text.values.tz_exit_label,
        right: exit.to_string(),
    };

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
                label: cf.fix_label,
                command: format!("{}{exit_tz}", cf.fix_command_prefix),
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
        // 标签是协议名 `IPv6`，两语种同形，不进 Copy（同 C4 的 `$TZ`）。
        values: vec![Row::Kv("IPv6", value)],
        notes: Vec::new(),
        fix: None,
        description: meta.description,
    }
}

fn card_o4(outcome: &Outcome<Risk>, text: &Text, style: &Style) -> Card {
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

    let mut values = vec![Row::Kv(
        text.verdict.risk_label,
        format!("{score}/100  {}", risk_bar(score, tone, style)),
    )];
    if let Some(kind) = &result.risk.network_type {
        values.push(Row::Kv(text.values.network_type_label, kind.clone()));
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
        values.push(Row::Kv(text.values.detections_label, flags.join("  ")));
    }

    values.push(Row::Kv(
        text.values.abuse_label,
        match &result.abuse {
            // 未知不冒充「无收录」（契约 2.3）。
            None => text.values.abuse_unknown.to_string(),
            Some(abuse) if abuse.listed => {
                format!("{} ({})", text.values.abuse_listed, abuse.frequency)
            }
            Some(_) => text.values.abuse_clean.to_string(),
        },
    ));

    // anonymous 决定判「高」的阈值（契约 3.1）。不显示它，用户就看不出同样的分数
    // 为什么这次判了高——那是把判据藏起来。整句排在标签列网格之后，不夹在中间
    // 把网格断成两截。
    if result.risk.anonymous {
        values.push(Row::Plain(text.values.anonymous_flag.to_string()));
    }

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
            let comparison_line = Row::Cmp {
                left_label: dt.ecs_label,
                left: ecs_country.clone(),
                op,
                right_label: dt.exit_label,
                right: exit_country.clone(),
            };
            let leak_message = if *leak { dt.leak } else { dt.no_leak };
            let state = if *leak {
                dt.state_leaked
            } else {
                dt.state_not_leaked
            };
            (
                tone,
                state,
                vec![comparison_line, Row::Plain(leak_message.to_string())],
            )
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
                vec![Row::Plain(message.to_string())],
            )
        }
    };

    // resolver 归属始终展示（与 §5.1 里 CLI 同时展示 $TZ 与系统时区同构），
    // 挂一个「仅供参考」pill 在值旁边（原型 refs/cli-report-redesign.html:392-394
    // 的位置关系）；notes 里的 resolver_note 整句保留不动——两者并存是原型的
    // 设计，不是待消除的重复：pill 挂在值本身供扫读，note 是解释句供细读。
    values.push(Row::Kv(
        dt.resolver_label,
        format!(
            "{}  {}",
            result
                .resolver_geo
                .as_deref()
                .unwrap_or(text.values.unknown),
            text.values.reference_only,
        ),
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
            let comparison_line = Row::Cmp {
                left_label: ut.reflexive_label,
                left: reflexive_ip.to_string(),
                op,
                right_label: ut.exit_label,
                right: exit_ip.to_string(),
            };
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
                vec![comparison_line, Row::Plain(mismatch_message.to_string())],
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
                vec![Row::Plain(message.to_string())],
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

fn card_c1(outcome: &Outcome<RealIp>, text: &Text) -> Card {
    let meta = &text.checks.c1;
    match outcome {
        Outcome::Done(real) => {
            let (values, notes) = ip_with_geo_rows(
                &real.ip,
                real.geo.as_ref(),
                text.notes.geo_source_local,
                text,
            );
            Card {
                id: CheckId::C1,
                tone: Tone::Ok,
                title: meta.title,
                // 同 O1：没有 ok/warn/bad 分支，「已取得」是提示词而非评价，固定 Dim。
                state: Some(text.values.obtained.to_string()),
                state_tone: Tone::Dim,
                values,
                notes,
                fix: None,
                description: meta.description,
            }
        }
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
    // 地址占标签列、说明占取值列（原型 `.kv`）——多台 DNS 时说明才对得齐。
    // 地址是动态值，`Row::Kv` 的标签是 `&'static str`，这里用 Plain 自己拼一列
    // 不成立（宽度要跨行统一），因此把说明放在取值列、地址放标签列的语义由
    // `Row::KvOwned` 承担。
    let values = servers
        .iter()
        .map(|server| {
            let note = if let Some(entry) = server.entry {
                Some(format!("{} ({})", entry.name, entry.region))
            } else if server.private {
                Some(text.values.dns_router.to_string())
            } else {
                None
            };
            Row::KvOwned(server.address.clone(), note.unwrap_or_default())
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
    let mut values = vec![Row::Kv(
        text.values.proxy_env,
        state_label(&status.env_state()).to_string(),
    )];
    let mut system = state_label(&status.system).to_string();
    if !status.system_kinds.is_empty() {
        system.push_str("  ");
        system.push_str(&status.system_kinds.join(" "));
    }
    values.push(Row::Kv(text.values.proxy_system, system));
    values.push(Row::Kv(
        text.values.proxy_tun,
        state_label(&status.tun).to_string(),
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

    /// 满配报告：十项全部完成，且把改版新增的着色元素一次凑齐——风险刻度条、
    /// 「需关注」清单、各卡状态词、O5 的 `reference_only` pill、`=`/`≠` 比对符。
    /// `blank()`（十项全失败）测不到其中任何一个，颜色断言只跑它等于没跑。
    fn full() -> Report {
        Report {
            o1: Outcome::Done(ExitInfo {
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
            }),
            o2: Outcome::Done(TimezoneCheck {
                local: Some("Asia/Shanghai".into()),
                exit: Some("Asia/Tokyo".into()),
                matches: Some(false),
            }),
            o3: Outcome::Done(ipify::Ipv6::Leaked("2001:db8::1".parse().unwrap())),
            o4: Outcome::Done(Risk {
                risk: crate::probe::proxycheck::Risk {
                    network_type: Some("Hosting".into()),
                    proxy: true,
                    vpn: true,
                    tor: false,
                    scraper: false,
                    risk_score: 85,
                    anonymous: true,
                },
                abuse: Some(crate::probe::stopforumspam::Abuse {
                    listed: true,
                    frequency: 3,
                    last_seen: None,
                }),
            }),
            o5: Outcome::Done(dns_egress::DnsEgress {
                resolver_geo: Some("Japan - Google LLC".into()),
                comparison: dns_egress::Comparison::Comparable {
                    leak: true,
                    ecs_country: "CN".into(),
                    exit_country: "JP".into(),
                },
            }),
            o6: Outcome::Done(udp_egress::UdpEgress::Comparable {
                mismatch: true,
                reflexive_ip: "198.51.100.20".parse().unwrap(),
                exit_ip: "212.50.249.204".parse().unwrap(),
            }),
            c1: Outcome::Done(RealIp {
                ip: "203.0.113.7".into(),
                // 与 O1 的出口归属刻意不同：这正是 C1 加归属要让用户看见的对照。
                geo: Some(crate::probe::proxycheck::Geo {
                    country_name: Some("China".into()),
                    country_code: Some("CN".into()),
                    region_name: Some("Guangdong".into()),
                    city_name: Some("Shenzhen".into()),
                    timezone: Some("Asia/Shanghai".into()),
                    asn: Some("AS4134".into()),
                    organisation: Some("Chinanet".into()),
                    provider: None,
                }),
            }),
            c2: Outcome::Done(vec![dns::Server {
                address: "114.114.114.114".into(),
                entry: None,
                private: false,
                domestic: true,
            }]),
            c3: Outcome::Done(proxy::Status {
                env_vars: vec!["HTTP_PROXY".into()],
                system: proxy::State::Enabled,
                system_kinds: vec!["HTTP".into()],
                tun: proxy::State::Disabled,
            }),
            c4: Outcome::Done(TimezoneCheck {
                local: Some("Asia/Shanghai".into()),
                exit: Some("Asia/Tokyo".into()),
                matches: Some(false),
            }),
        }
    }

    fn render(report: &Report, color: bool, verbose: bool) -> String {
        super::report(report, &copy::text(Lang::En), &Style::new(color), verbose)
    }

    /// 说明文字现在按 76 列折行、悬挂缩进（C4），原本整句的 `contains` 断言
    /// 会被拆到多行而失配。这里把渲染输出按行拼回一整段（去掉每行的缩进——
    /// 卡片内 7 格、结论区 4 格、页脚 2 格三种都要吃，所以直接 `trim_start`），
    /// 用于校验被折行打散的说明文字整体是否仍然完整存在——只在英文断言里使用，
    /// 英文按空格折行，拼回时补单空格能精确复原原文；中文长句会被强制按字符切分
    /// （无空格可依），拼回会插入原文没有的空格，因此不适用于中文断言。
    /// 断点落在**双空格**上的文本（页脚的对齐空格）同样复原不了，那里另有断言。
    /// 中文按字折行，断点两侧都是全角字符时**不补空格**——补了就复原不出原句。
    fn dewrap(out: &str) -> String {
        let mut flat = String::new();
        for line in out.lines() {
            let line = line.trim_start();
            let joins_cjk = flat.chars().last().is_some_and(|c| char_width(c) == 2)
                && line.chars().next().is_some_and(|c| char_width(c) == 2);
            if !flat.is_empty() && !joins_cjk {
                flat.push(' ');
            }
            flat.push_str(line);
        }
        flat
    }

    #[test]
    fn no_color_output_has_no_escape_sequences() {
        // 重定向到文件不该满屏转义序列。跑满配报告——空报告里刻度条、需关注清单、
        // 状态词、pill、比对符一个都不出现，只跑它等于把改版新增的着色路径全放过。
        for report in [blank(), full()] {
            for verbose in [false, true] {
                let out = render(&report, false, verbose);
                assert!(!out.contains('\x1b'), "{out}");
            }
        }
    }

    #[test]
    fn color_output_does_have_escape_sequences() {
        for report in [blank(), full()] {
            assert!(render(&report, true, false).contains('\x1b'));
        }
    }

    #[test]
    fn a_full_report_never_exceeds_76_columns() {
        // spec §5.5 的 76 列约束覆盖整份报告——结论区摘要、attention_scope 句与
        // 页脚都在内，不只是检测卡里的说明行。宽度按屏幕宽度算（跳过转义序列），
        // 彩色输出与 --no-color 应当折在同一处。
        for lang in [Lang::En, Lang::ZhHans] {
            let text = copy::text(lang);
            // 结论区的 facts 网格三行不在约束内：spec §5.5 收的是**说明文字**，
            // facts 是靠标签列对齐的数据行，折行会把网格拆散。出口 IP 那行在
            // ASN + 机构名都长时确实会越过 76 列，这一条留给人类裁定。
            let facts = [
                text.verdict.exit_ip_label,
                text.verdict.risk_label,
                text.verdict.coverage_label,
            ];
            for color in [false, true] {
                for verbose in [false, true] {
                    let out = super::report(&full(), &text, &Style::new(color), verbose);
                    for line in out
                        .lines()
                        .filter(|l| !facts.iter().any(|label| l.trim_start().starts_with(label)))
                    {
                        assert!(
                            display_width(line) <= PROSE_WIDTH,
                            "超出 76 列（{} 列）：{line:?}",
                            display_width(line)
                        );
                    }
                }
            }
        }
    }

    /// 分组标题行（发丝线那一行）。
    fn group_line<'a>(out: &'a str, name: &str) -> &'a str {
        out.lines()
            .find(|line| line.contains(name) && line.contains('─'))
            .unwrap_or_else(|| panic!("找不到分组标题：{out}"))
    }

    #[test]
    fn the_report_follows_the_window_width() {
        let text = copy::text(Lang::En);

        // 窄窗：整份报告收进窗口，发丝线跟着缩。
        let out = super::report(&full(), &text, &Style::sized(false, 50), true);
        for line in out.lines() {
            assert!(display_width(line) <= 50, "超出 50 列：{line:?}");
        }
        assert_eq!(
            display_width(group_line(&out, text.groups.online)),
            50,
            "{out}"
        );

        // 宽窗：整幅张到上限，**说明文字不跟着摊开**——两条上限是分开的
        // （原型 `.screen{max-width:96ch}` vs `.note{max-width:76ch}`）。
        let out = super::report(&full(), &text, &Style::sized(false, 120), true);
        for line in out.lines() {
            assert!(display_width(line) <= MAX_WIDTH, "超出整幅上限：{line:?}");
        }
        assert_eq!(
            display_width(group_line(&out, text.groups.online)),
            MAX_WIDTH,
            "{out}"
        );
        assert!(
            !out.contains(text.notes.geo_source),
            "说明文字整句没被折行，说明它跟着窗口摊开了：{out}"
        );
        assert!(dewrap(&out).contains(text.notes.geo_source), "{out}");

        // 比窗口下限还窄：夹到 MIN_WIDTH。发丝线此时排不下，分组统计换行右对齐，
        // 但没有任何一行溢出。
        let out = super::report(&full(), &text, &Style::sized(false, 10), false);
        for line in out.lines() {
            assert!(display_width(line) <= MIN_WIDTH, "超出下限：{line:?}");
        }
        assert!(
            out.lines().any(|l| l.contains(text.groups.online)),
            "分组标题不能因为窄窗消失：{out}"
        );
    }

    #[test]
    fn a_state_word_that_cannot_share_the_title_line_moves_down_instead_of_overflowing() {
        // 窄窗里「标题 + 状态词」挤不下时，状态词换行右对齐——溢出到窗口外
        // 等于把扫读用的那一列藏起来。
        let text = copy::text(Lang::En);
        let out = super::report(&full(), &text, &Style::sized(false, 44), false);
        let head = head_line(&out, "C2");
        assert!(!head.contains(text.values.dns_domestic), "{out}");
        let state_line = out
            .lines()
            .skip_while(|l| *l != head)
            .nth(1)
            .expect("状态词必须紧跟在标题行之后");
        assert!(
            state_line.trim_start().ends_with(text.values.dns_domestic),
            "{out}"
        );
        assert_eq!(display_width(state_line), 44, "{out}");
    }

    #[test]
    fn checks_are_split_into_an_online_and_a_local_group() {
        // 原型 `.group`：两条发丝线把十项分成「联网可测」与「仅 CLI 可测」，
        // 右端各自交代这组的量与性质。检测项顺序不变，分组只是呈现。
        for lang in [Lang::En, Lang::ZhHans] {
            let text = copy::text(lang);
            let out = super::report(&full(), &text, &Style::new(false), false);
            let online = out
                .lines()
                .find(|l| l.contains(text.groups.online))
                .expect("联网组标题必须在");
            let local = out
                .lines()
                .find(|l| l.contains(text.groups.local))
                .expect("本机组标题必须在");
            assert!(online.contains("O1–O6"), "{out}");
            assert!(online.contains(text.groups.all_done), "{out}");
            assert!(local.contains("C1–C4"), "{out}");
            assert!(local.contains(text.groups.local_only), "{out}");
            // 发丝线把两端撑满整幅宽度，两组的右端因此对齐在同一列。
            assert_eq!(display_width(online), PROSE_WIDTH, "{out}");
            assert_eq!(display_width(local), PROSE_WIDTH, "{out}");
            // 分组标题必须在它那组的第一张卡之前。
            let online_at = out.find(text.groups.online).unwrap();
            let local_at = out.find(text.groups.local).unwrap();
            assert!(online_at < out.find("O1").unwrap(), "{out}");
            assert!(out.find(" O6  ").unwrap() < local_at, "{out}");
            assert!(local_at < out.find(" C1  ").unwrap(), "{out}");
        }
    }

    #[test]
    fn a_failed_online_check_is_counted_in_its_group_header() {
        // 判别力对照：全测成才说「全部完成」，有失败就报失败数——分组统计不能是
        // 写死的装饰。
        let mut report = full();
        report.o3 = Outcome::Failed(Failure::Upstream);
        let text = copy::text(Lang::En);
        let out = super::report(&report, &text, &Style::new(false), false);
        let online = out
            .lines()
            .find(|l| l.contains(text.groups.online))
            .unwrap();
        assert!(
            online.contains(&format!("{} 1", text.coverage.failed)),
            "{out}"
        );
        assert!(!online.contains(text.groups.all_done), "{out}");
    }

    #[test]
    fn value_labels_inside_a_card_line_up_in_one_column() {
        // 原型 `.kv` 网格：O4 卡内标签宽度不一（「IP 类型」7 列 vs「风险评分」8 列），
        // 取值必须落在同一列上，否则就是散行。
        let text = copy::text(Lang::ZhHans);
        let out = super::report(&full(), &text, &Style::new(false), false);
        let labels = [
            text.verdict.risk_label,
            text.values.network_type_label,
            text.values.detections_label,
            text.values.abuse_label,
        ];
        let widest = labels.iter().map(|l| display_width(l)).max().unwrap();
        assert!(
            labels.iter().map(|l| display_width(l)).min().unwrap() < widest,
            "这几个标签必须宽度不一，否则这条测试证明不了补齐"
        );

        for label in labels {
            let line = out
                .lines()
                .find(|l| l.starts_with(&format!("{NOTE_INDENT}{label}")))
                .unwrap_or_else(|| panic!("{label} 行必须在：{out}"));
            let after_label = &line[NOTE_INDENT.len() + label.len()..];
            let padding = after_label.len() - after_label.trim_start_matches(' ').len();
            // 取值起始列 = 缩进 + 最宽标签 + 两格间隔，四行都一样。
            assert_eq!(
                display_width(NOTE_INDENT) + display_width(label) + padding,
                display_width(NOTE_INDENT) + widest + 2,
                "{label} 的取值没落在公共列上：{out}"
            );
        }
    }

    #[test]
    fn the_verdict_level_reads_as_a_badge_without_color() {
        // 原型 `.badge` 是反白色块；`--no-color` 下退回 `[ … ]`，语义不靠颜色
        // 承载（点 8）。
        let text = copy::text(Lang::En);
        let out = render(&full(), false, false);
        assert!(out.contains(&format!("[ {} ]", text.verdict.high)), "{out}");
    }

    #[test]
    fn cjk_lines_never_start_with_closing_punctuation() {
        // 中文按字折行后必须守标点禁则，否则会折出孤零零一行「。」。
        let out = super::report(&full(), &copy::text(Lang::ZhHans), &Style::new(false), true);
        for line in out.lines() {
            let first = line.trim_start().chars().next();
            assert!(
                !first.is_some_and(no_line_start),
                "行首出现了禁则标点：{line:?}"
            );
        }
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
        assert!(dewrap(&render(&report, false, false)).contains(text.values.anonymous_flag));
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
        assert!(
            dewrap(o5_card).contains(text.dns_egress.no_ecs),
            "{o5_card}"
        );
        assert!(
            !dewrap(o5_card).contains(text.failures.upstream),
            "{o5_card}"
        );
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
        assert!(
            dewrap(&out).contains(text.udp_egress.stun_disagree),
            "{out}"
        );
        assert!(!dewrap(&out).contains(text.udp_egress.no_mismatch), "{out}");

        let mut miss = blank();
        miss.o6 = Outcome::Done(udp_egress::UdpEgress::Comparable {
            mismatch: false,
            reflexive_ip: "198.51.100.20".parse().unwrap(),
            exit_ip: "198.51.100.20".parse().unwrap(),
        });
        let out = render(&miss, false, false);
        assert!(dewrap(&out).contains(text.udp_egress.no_mismatch), "{out}");
        assert!(
            !dewrap(&out).contains(text.udp_egress.stun_disagree),
            "{out}"
        );
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

    /// 低档 + 一个仅提醒项：O2 系统时区不一致（契约 §2.1 明列不进综合结论），
    /// O4 分数 10 且无滥用收录 ⇒ 综合结论 Full(Low)。
    fn low_verdict_with_one_reminder_only_item() -> Report {
        let mut report = blank();
        report.o2 = tz("Asia/Shanghai", Some("Asia/Tokyo"), Some(false));
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
                listed: false,
                frequency: 0,
                last_seen: None,
            }),
        });
        report
    }

    #[test]
    fn a_low_verdict_with_reminder_only_items_does_not_claim_that_nothing_was_found() {
        // 「各项均未发现异常」与同屏的「需关注」清单自相矛盾——两句话的透镜不同：
        // 摘要看「有没有贡献信号」，清单看「有没有值得看一眼的项」。纯提醒项
        // （O2 系统时区、C2 国内 DNS）是常态，不是边角情形。
        let report = low_verdict_with_one_reminder_only_item();
        assert_eq!(report.verdict(), Verdict::Full(Level::Low));

        for lang in [Lang::En, Lang::ZhHans] {
            let text = copy::text(lang);
            let out = super::report(&report, &text, &Style::new(false), false);
            let flat = dewrap(&out);
            assert!(out.contains(text.verdict.attention_label), "{out}");
            assert!(
                flat.contains(text.verdict.summary_full_low_reminders),
                "{out}"
            );
            assert!(!flat.contains(text.verdict.summary_full_low), "{out}");
        }
    }

    #[test]
    fn a_low_verdict_without_any_attention_item_keeps_the_plain_summary() {
        // 判别力对照：把仅提醒项拿掉，摘要必须换回「各项均未发现异常」——
        // 否则新键会变成低档的无条件文案，等于把原来的句子直接改掉。
        let mut report = low_verdict_with_one_reminder_only_item();
        report.o2 = Outcome::Failed(Failure::Upstream);
        assert_eq!(report.verdict(), Verdict::Full(Level::Low));

        for lang in [Lang::En, Lang::ZhHans] {
            let text = copy::text(lang);
            let out = super::report(&report, &text, &Style::new(false), false);
            let flat = dewrap(&out);
            assert!(!out.contains(text.verdict.attention_label), "{out}");
            assert!(flat.contains(text.verdict.summary_full_low), "{out}");
            assert!(
                !flat.contains(text.verdict.summary_full_low_reminders),
                "{out}"
            );
        }
    }

    #[test]
    fn the_preliminary_low_summary_has_the_same_pair() {
        // 初步形态同样可达这个组合：O2 不一致（仅提醒）+ O3 未启用（信号已知、未命中）
        // ⇒ 风险分未知 ⇒ Preliminary(Low)，清单里仍有 O2。
        let mut with_reminder = blank();
        with_reminder.o2 = tz("Asia/Shanghai", Some("Asia/Tokyo"), Some(false));
        with_reminder.o3 = Outcome::Done(ipify::Ipv6::Disabled);
        assert_eq!(
            with_reminder.verdict(),
            Verdict::Preliminary(PreliminaryLevel::Low)
        );

        let mut without = blank();
        without.o3 = Outcome::Done(ipify::Ipv6::Disabled);
        assert_eq!(
            without.verdict(),
            Verdict::Preliminary(PreliminaryLevel::Low)
        );

        for lang in [Lang::En, Lang::ZhHans] {
            let text = copy::text(lang);

            let out = super::report(&with_reminder, &text, &Style::new(false), false);
            let flat = dewrap(&out);
            assert!(
                flat.contains(text.verdict.summary_preliminary_low_reminders),
                "{out}"
            );
            assert!(
                !flat.contains(text.verdict.summary_preliminary_low),
                "{out}"
            );

            let out = super::report(&without, &text, &Style::new(false), false);
            let flat = dewrap(&out);
            assert!(flat.contains(text.verdict.summary_preliminary_low), "{out}");
            assert!(
                !flat.contains(text.verdict.summary_preliminary_low_reminders),
                "{out}"
            );
        }
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
        // scope 句超过 76 列会被折行，断言前拼回整段。
        let flat = dewrap(&out);
        assert!(out.contains(text.verdict.attention_label), "{out}");
        assert!(
            flat.contains(&format!("C4 {}", text.verdict.attention_contributing)),
            "{out}"
        );
        // 锁的是渲染结果里的空格，不是拿 Copy 原始取值去拼期望值。
        assert!(
            flat.contains(&format!(
                "O2 and O4 {}",
                text.verdict.attention_reminder_only
            )),
            "{out}"
        );
    }

    #[test]
    fn attention_scope_connector_gets_single_spaces_in_both_languages() {
        // 两个语种的 attention_list_connector 都自带两侧空格（" 与 " / " and "），
        // 渲染层裸拼接。锁的是渲染结果里的空格数——连接词取值若漏带或多带空格，
        // 这里当场变红。
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
            entry: None,
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
        // 比对行两侧各自带标签（原型 `.cmp`）：O2 的本地侧是系统时区，
        // C4 的是字面量 $TZ，两个裸时区名读不出谁是谁。
        let text = copy::text(Lang::En);
        let system = text.values.tz_system_label;
        let exit = text.values.tz_exit_label;

        let mut report = blank();
        report.o2 = tz("Asia/Shanghai", Some("Asia/Shanghai"), Some(true));
        let out = render(&report, false, false);
        assert!(
            out.contains(&format!(
                "{system}  Asia/Shanghai  =  {exit}  Asia/Shanghai"
            )),
            "{out}"
        );

        let mut report = blank();
        report.o2 = tz("Asia/Shanghai", Some("Asia/Tokyo"), Some(false));
        let out = render(&report, false, false);
        assert!(
            out.contains(&format!("{system}  Asia/Shanghai  ≠  {exit}  Asia/Tokyo")),
            "{out}"
        );
        assert!(!out.contains("  =  "), "{out}");

        // C4 的本地侧标签是 $TZ 字面量，不随语种变化。
        let mut report = blank();
        report.c4 = tz("Asia/Shanghai", Some("Asia/Tokyo"), Some(false));
        let out = render(&report, false, false);
        assert!(
            out.contains(&format!("$TZ  Asia/Shanghai  ≠  {exit}  Asia/Tokyo")),
            "{out}"
        );
    }

    #[test]
    fn timezone_indeterminate_uses_neither_equals_nor_not_equals() {
        // 「无从比对」（matches: None）两个比对符都不该出现——走 Dim marker「·」。
        let mut report = blank();
        report.o2 = tz("Asia/Shanghai", None, None);
        let out = render(&report, false, false);
        assert!(!out.contains("  =  "), "{out}");
        assert!(!out.contains("  ≠  "), "{out}");
        assert!(
            out.contains(&format!(
                "Asia/Shanghai  ·  {}  unknown",
                copy::text(Lang::En).values.tz_exit_label
            )),
            "{out}"
        );
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
        // 状态词顶到行尾（原型 `.ctitle{flex:1}`），标题与状态词之间是对齐空格，
        // 不是固定的两格——锁「同一行、标题在左、状态词收在 76 列」这三件事。
        assert!(
            head_line(&out, "O3").ends_with(text.values.ipv6_disabled),
            "{out}"
        );
        assert!(head_line(&out, "O3").contains("IPv6 Leak"), "{out}");
        assert_eq!(display_width(head_line(&out, "O3")), PROSE_WIDTH, "{out}");

        // O4：{分数}/100 {分级词}，不带「风险」后缀。33 分落在 26–75，判 medium。
        let report = risk_report(33, false);
        let out = render(&report, false, false);
        assert!(
            head_line(&out, "O4").ends_with(&format!("33/100 {}", text.values.risk_level_medium)),
            "{out}"
        );
    }

    /// 某张检测卡的标题行（`  ✔ O3  …`）。
    fn head_line<'a>(out: &'a str, id: &str) -> &'a str {
        // 「需关注」清单里也有 `! O4  …`，因此锁死行首两格 + marker + 编号的完整形状。
        let heads: Vec<String> = ["✔", "!", "✖", "·"]
            .iter()
            .map(|marker| format!("  {marker} {id}  "))
            .collect();
        out.lines()
            .find(|line| heads.iter().any(|head| line.starts_with(head)))
            .unwrap_or_else(|| panic!("找不到 {id} 的标题行：{out}"))
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
            assert!(line.chars().count() <= PROSE_WIDTH, "超出 76 列：{line:?}");
        }
        // geo_source（102 字符）必然被折成至少两行；续行悬挂缩进到 NOTE_INDENT，不顶格。
        assert!(
            out.contains(&format!(
                "\n{NOTE_INDENT}database, so the two can disagree."
            )),
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

    #[test]
    fn the_footer_hint_uses_the_real_cli_syntax_and_shows_in_every_language() {
        // 命令字面量以 main.rs 的 Cli/Command/ConfigAction/SettableKey 定义为准：
        // --verbose/--json 是顶层 flag，config set proxycheck-key 是 SettableKey
        // 目前唯一枚举的键（`#[value(name = "proxycheck-key")]`）。
        for lang in [Lang::En, Lang::ZhHans] {
            let text = copy::text(lang);
            let out = super::report(&blank(), &text, &Style::new(false), false);
            // 用完整行匹配而非松散 contains——命令字面量若被改错一个字符，
            // 单独的子串断言可能仍然因为是另一处的前缀而碰巧通过。
            // --verbose 与 --json 各占一行；期望值仍按同一折行规则拆开后逐行
            // 连着页脚缩进整行匹配：命令字面量、提示词、缩进列三者都被锁死。
            let hints = [
                format!("preflight --verbose  {}", text.footer.verbose_hint),
                format!("preflight --json  {}", text.footer.json_hint),
            ];
            for hint in &hints {
                for line in wrap_lines(hint, FOOTER_INDENT, PROSE_WIDTH) {
                    assert!(out.contains(&format!("\n{FOOTER_INDENT}{line}\n")), "{out}");
                }
            }
            assert!(
                out.contains(&format!(
                    "  preflight config set proxycheck-key  {}\n",
                    text.footer.quota_hint
                )),
                "{out}"
            );
        }
    }

    #[test]
    fn the_footer_hint_never_leaks_into_json_output() {
        // --json 走 json::report，完全不经过 render::report——这里断言机器可读
        // 输出里没有页脚的字面命令串，锁住这条天然屏障，不只是靠代码路径隔离。
        let payload = serde_json::to_string_pretty(&crate::json::report(&blank())).unwrap();
        assert!(!payload.contains("preflight --verbose"), "{payload}");
        assert!(!payload.contains("preflight --json"), "{payload}");
        assert!(
            !payload.contains("preflight config set proxycheck-key"),
            "{payload}"
        );
    }

    #[test]
    fn anonymous_51_to_75_shows_a_high_verdict_with_a_yellow_o4_card_and_its_explanation() {
        // 契约 §6：`anonymous: true` 且分数落在 51–75 时会出现「结论高 · 分项黄」。
        // 呈现层必须在 O4 卡片内说明这个 IP 正被用作匿名化地址、判高的阈值对它
        // 降到 51——否则用户看到高风险结论却找不到哪一项显红，会以为结论算错了。
        // 三件事一起断言：结论区档位是「高」、O4 卡片自身仍是黄（不是红），
        // 卡片里带着 anonymous_flag 这句解释——不能只看其中一件就当作满足契约。
        let report = risk_report(60, true);
        let text = copy::text(Lang::En);
        let out = render(&report, false, false);

        assert_eq!(report.verdict(), Verdict::Full(Level::High));
        assert!(out.contains(text.verdict.high), "{out}");

        // 卡片流以「· O1」定位（risk_report 只覆盖 o4，O1 仍是失败卡，Dim marker
        // 「·」），与既有测试 `low_score_but_abuse_listed_...` 同一手法——避开
        // 「需关注」清单里同样出现的 " O4  " 子串，只在卡片流本身里核对 O4。
        let cards_start = out.find("· O1").expect("O1 卡片必须存在（cards 流起点）");
        let cards_block = &out[cards_start..];
        let o4_start = cards_block.find(" O4  ").expect("O4 卡片必须存在");
        let o4_end = cards_block[o4_start..]
            .find("\n\n")
            .map(|i| o4_start + i)
            .unwrap_or(cards_block.len());
        let o4_card = &cards_block[o4_start..o4_end];
        let marker_before_o4 = cards_block[..o4_start].chars().last();
        assert_eq!(
            marker_before_o4,
            Some('!'),
            "O4 卡片自身应仍是黄色 marker（分项分级只看分数，60 分落在 26–75）：{o4_card}"
        );
        assert!(
            dewrap(o4_card).contains(text.values.anonymous_flag),
            "O4 卡片必须解释判高阈值降到 51 这件事：{o4_card}"
        );
    }
}
