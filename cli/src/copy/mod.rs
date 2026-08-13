//! 文案层。与 Preflight Web 的 `src/copy.ts` 同构：英文是**源语言**，两语种（en / zh-hans）
//! 都由结构体**完整**填写——漏一个字段是编译错误，不存在"部分翻译回落英文"这回事
//! （ADR-0016：语种收缩为 en + zh-hans 后，删除了字段级回落机制）。
//!
//! 命名与 Web 的对应：`Text` ↔ `Copy`。这里不叫 `Copy` 是因为它会和
//! `std::marker::Copy` 撞名。
//!
//! 刻意不用 `rust-i18n` 之类的方案：它们的 key 是字符串，拼错或改名漏改只在运行时暴露，
//! 而 Web 侧明确花代价买下了"漏译是编译错误"这个性质（见 `src/locales/en.ts` 开头）。

/// 定义一组叶子文案：所有字段都是字符串。
macro_rules! copy_leaf {
    (
        $(#[$outer:meta])*
        $name:ident {
            $( $(#[$fm:meta])* $field:ident ),* $(,)?
        }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name {
            $( $(#[$fm])* pub $field: &'static str, )*
        }
    };
}

/// 定义一组由其他文案组合而成的节点。可任意嵌套。
macro_rules! copy_node {
    (
        $(#[$outer:meta])*
        $name:ident {
            $( $(#[$fm:meta])* $field:ident : $ty:ident ),* $(,)?
        }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name {
            $( $(#[$fm])* pub $field: $ty, )*
        }
    };
}

mod en;
mod zh_hans;

use crate::lang::Lang;

copy_leaf! {
    /// `preflight config` 子命令的文案。
    ConfigText {
        path_label,
        key_state_set,
        key_state_unset,
        key_prompt,
        key_saved,
        key_empty,
        /// `config list` 里 secret 值的状态词（如「已设置」）。键名是字面 TOML 键，
        /// 不随语种变化，写死在 main.rs；这里只放状态词。
        list_value_set,
        list_value_unset,
    }
}

copy_leaf! {
    /// 错误信息。动态细节（路径、键名、原始错误）由调用方追加在冒号之后，
    /// 不做插值——避免为几条消息发明一套模板语法。
    ErrorText {
        config_read,
        config_parse,
        config_write,
        lang_unknown,
    }
}

copy_leaf! {
    /// 结论区。档位文案与「数据不足」是契约的硬性要求，不得被改版误删。
    VerdictText {
        low,
        medium,
        high,
        /// 数据不足时的档位文案。**绝不能落到「低风险」**——没测成不是安全。
        insufficient,
        preliminary_badge,
        full_badge,
        summary_insufficient,
        summary_preliminary_low,
        /// 低档、但「需关注」清单里存在仅提醒项时的摘要。低档的默认摘要说「未发现异常」，
        /// 与同屏的仅提醒清单（O2 系统时区、C2 国内 DNS 这类契约 §2.1 明列不进综合结论
        /// 的项）自相矛盾——两句话的透镜不同：摘要看的是「有没有贡献信号」，清单看的是
        /// 「有没有值得看一眼的项」。矛盾出在文案，不在判级逻辑，因此这里给它自己的键。
        summary_preliminary_low_reminders,
        summary_preliminary_medium,
        summary_full_low,
        /// 同 `summary_preliminary_low_reminders`，用于完整形态的低档。
        summary_full_low_reminders,
        summary_full_medium,
        summary_full_high,
        exit_ip_label,
        /// facts 网格「风险评分」行的标签列（spec §5.2，原型 `.facts` 的 `<span class="k">`）。
        risk_label,
        /// facts 网格「覆盖度」行的标签列，同上——四行共用同一个标签列，缺了就不再是
        /// 对齐网格，是散行。
        coverage_label,
        /// 结论区「需关注」清单的标签（spec §5.1）。
        attention_label,
        /// 「需关注」清单里，贡献综合结论的项的标注词（vs 仅提醒项）。
        attention_contributing,
        /// 「需关注」清单里，仅提醒、不进综合结论的项的标注词。
        attention_reminder_only,
        /// 「需关注」清单里非最后一项之间的分隔符——中英文习惯不同，不能由渲染层写死
        /// （中文顿号 vs 英文逗号）。
        attention_list_separator,
        /// 「需关注」清单里最后一项前的连接词（中文" 与 "、英文" and "）。
        /// 与 `C4FixText` 的 `explain_connector` 同一约定：**空格由取值自带，渲染层裸拼接**。
        attention_list_connector,
        /// `attention_scope` 句的起始固定短语（中文"其中只有 "、英文"Only "），
        /// 只在有贡献项时使用。
        attention_prefix,
        /// `attention_scope` 句里，贡献分句与仅提醒分句之间的连接标点
        /// （中文"，"、英文"; "），只在两个分句都非空时使用。
        attention_clause_separator,
        /// `attention_scope` 句的收尾标点（中文"。"、英文"."）。
        attention_suffix,
    }
}

copy_leaf! {
    /// 覆盖度。综合结论永远不得脱离它单独呈现（ADR-0004）。
    CoverageText {
        done,
        failed,
    }
}

copy_leaf! {
    /// 一个检测项的标题与展开说明（`--verbose` 才出说明）。
    CheckText {
        title,
        description,
    }
}

copy_leaf! {
    /// O1 卡片值行的三个标签（地址 / 归属 / 网络）。只有 O1 需要，不塞进通用
    /// `CheckText`——那样另外 9 项都要陪它填一个用不上的值（spec §5.1）。
    O1FieldsText {
        address,
        ownership,
        network,
    }
}

copy_leaf! {
    /// C4 卡片专属：成因句与修复建议（spec §5.1／§5.4，决策 5：只有 C4 给修复命令）。
    /// 成因句与修复命令都含动态时区名，按仓库既有风格不做插值，
    /// 拆成固定片段，动态值由调用方拼接在中间（同 `ErrorText` 的注释）。
    C4FixText {
        /// 成因句里，本地时区值之前的部分。
        explain_prefix,
        /// 成因句里，本地时区值与出口 IP 时区值之间的部分。
        explain_connector,
        /// 成因句的收尾标点。
        explain_suffix,
        /// 修复建议行的标签（如「建议」）。仅当 tzMismatchCliEnv 命中且出口 IP
        /// 时区名已知时渲染——「无从比对」不给建议（spec §5.4）。
        fix_label,
        /// `export TZ=` 前缀，时区名由调用方追加在其后，不做插值。
        fix_command_prefix,
    }
}

copy_node! {
    /// 10 个检测项。O1–O6 的标题必须与 Preflight Web 逐字一致（契约 1.1）。
    ChecksText {
        o1: CheckText,
        // O1 值行标签，只服务 o1，见 `O1FieldsText`。
        o1_fields: O1FieldsText,
        o2: CheckText,
        o3: CheckText,
        o4: CheckText,
        o5: CheckText,
        o6: CheckText,
        c1: CheckText,
        c2: CheckText,
        c3: CheckText,
        c4: CheckText,
        // C4 成因句与修复建议，只服务 c4，见 `C4FixText`。
        c4_fix: C4FixText,
    }
}

copy_leaf! {
    /// 检测卡的两个分组标题（原型 `.group`）：联网可测 vs 仅 CLI 可测。
    /// 编号区间（`O1–O6`）由渲染层从卡片列表派生，不是文案。
    GroupText {
        online,
        local,
        /// 分组统计里的「项」量词，与项数拼接（`6 项` / `6 checks`）。
        items,
        /// 联网组全部完成时的右端说明；有失败项时改用 `coverage.failed` + 数量。
        all_done,
        /// 本机组的右端说明——这组存在的理由就是网页版做不到。
        local_only,
    }
}

copy_leaf! {
    /// 检测项的取值与提示。
    ValueText {
        /// 探测进行中的提示，只往 stderr 打。
        checking,
        unknown,
        timezone_match,
        timezone_mismatch,
        timezone_indeterminate,
        ipv6_leaked,
        ipv6_disabled,
        proxy_env,
        proxy_system,
        proxy_tun,
        state_on,
        state_off,
        state_unsupported,
        dns_router,
        dns_domestic,
        anonymous_flag,
        abuse_listed,
        abuse_clean,
        abuse_unknown,
        /// 标题行右端的通用「已完成」状态词，给没有 ok/warn/bad 分支的检测项用
        /// （目前只有 O1、C1，spec §5.1「各检测项标题行状态词」）。
        obtained,
        /// 风险分刻度说明，对应契约 §6 分项分级（`<26` 绿 / `<76` 黄 / `≥76` 红）。
        risk_scale_note,
        /// O4 标题行右端的风险分级短词（不带「风险」后缀，区别于 `verdict.low/medium/high`）。
        risk_level_low,
        risk_level_medium,
        risk_level_high,
        /// 「仅供参考」短标签（如 O5 resolver 归属旁的 pill），与整句
        /// `dns_egress.resolver_note` 并存，不是同一样东西的重复定义。
        reference_only,
        /// O2 比对行的本地侧标签（系统时区）。C4 的本地侧是字面量 `$TZ`，
        /// 是 shell 变量名不是词，写死在渲染层。
        tz_system_label,
        /// O2／C4 比对行的出口侧标签。两项共用——比对的是同一个东西。
        tz_exit_label,
        /// O4 值行标签：网络类型 / 代理检出 / 滥用举报。
        network_type_label,
        detections_label,
        abuse_label,
    }
}

copy_leaf! {
    /// 检测失败的四种原因。分开说是因为用户能采取的行动不同。
    FailureText {
        upstream,
        quota_exhausted,
        local,
    }
}

copy_leaf! {
    /// 页脚提示行（design：`.footer-hint`）。命令本身（`preflight --verbose` 等）
    /// 是字面 CLI 语法，不随语种变化，写死在 `render.rs`；这里只放随语种变化的
    /// 说明词。
    FooterText {
        /// `preflight --verbose` 之后的说明词。
        verbose_hint,
        /// `preflight --json` 之后的说明词。
        json_hint,
        /// `preflight config set proxycheck-key` 之后的配额提示。
        quota_hint,
    }
}

copy_leaf! {
    /// 契约要求必须出现在屏幕上的说明，删掉就是回退。
    NoteText {
        /// 契约 5.4：归属来自 proxycheck，不是 Cloudflare，两端可能对不上。
        geo_source,
        /// 同上，但用于 C1——它在网页版根本不存在，「与网页版对不上」那半句无从谈起，
        /// 只保留「必须标明数据源」这条约束。
        geo_source_local,
        /// 契约 5.1：O2 测的是系统时区，命令行进程认 `$TZ`（那是 C4）。
        o2_desktop_only,
        /// ADR-0012：无 key 配额按出口 IP 计，与同节点用户共享。
        quota_shared,
    }
}

copy_leaf! {
    /// O5 卡片专属文案。契约 2.5／5.4：resolver 归属与 ECS 判定必须同时展示，
    /// 且明确标出只有后者进综合结论——与 §5.1 里 CLI 同时展示 `$TZ` 与系统时区同构。
    DnsEgressText {
        resolver_label,
        ecs_label,
        exit_label,
        leak,
        no_leak,
        /// ECS 缺失时的说明，状态仍是「已完成」而非「检测失败」（契约 2.5）。
        no_ecs,
        unmapped_country,
        unknown_exit_country,
        /// 标明 resolver 归属只展示、不参与判定。
        resolver_note,
        /// 标题行右端的短状态词，区别于整句的 `leak`/`no_leak`（spec §5.1）。
        state_leaked,
        state_not_leaked,
    }
}

copy_leaf! {
    /// O6 卡片专属文案。「无从比对」的三种成因与「未命中」在措辞上必须可分（契约 2.6）。
    UdpEgressText {
        reflexive_label,
        exit_label,
        mismatch,
        no_mismatch,
        family_mismatch,
        unknown_exit_ip,
        stun_disagree,
        /// 标题行右端的短状态词，区别于整句的 `mismatch`/`no_mismatch`（spec §5.1）。
        state_match,
        state_mismatch,
    }
}

copy_leaf! {
    /// `preflight dns` 命令的文案。列名、variant 取值、--check 状态词与引导句。
    DnsCommandText {
        /// 列名：IP 地址。
        col_ip,
        /// 列名：提供商（品牌名）。
        col_provider,
        /// 列名：地区。
        col_region,
        /// 列名：是否国内。
        col_domestic,
        /// 列名：用途（过滤级别）。
        col_variant,
        /// 列名：延迟（仅 --check）。
        col_latency,
        /// 列名：状态（仅 --check）。
        col_status,
        /// variant = standard：普通解析。
        variant_standard,
        /// variant = security：拦恶意/钓鱼。
        variant_security,
        /// variant = family：在 security 基础上再拦成人内容。
        variant_family,
        /// variant = adblock：拦广告/追踪。
        variant_adblock,
        /// 国内标记词。
        domestic_yes,
        /// --check 状态：通。
        check_ok,
        /// --check 状态：应答可疑。
        check_suspicious,
        /// --check 状态：不通。
        check_unreachable,
        /// 页脚引导句：用 `preflight dns --check` 实测。
        footer_hint,
    }
}

copy_node! {
    /// 全部文案。
    Text {
        config: ConfigText,
        errors: ErrorText,
        verdict: VerdictText,
        coverage: CoverageText,
        groups: GroupText,
        checks: ChecksText,
        values: ValueText,
        failures: FailureText,
        notes: NoteText,
        footer: FooterText,
        dns_egress: DnsEgressText,
        udp_egress: UdpEgressText,
        dns_cmd: DnsCommandText,
    }
}

/// 取某语种的完整文案。两语种都是完整结构体，不存在回落。
pub const fn text(lang: Lang) -> Text {
    match lang {
        Lang::En => en::EN,
        Lang::ZhHans => zh_hans::ZH_HANS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_the_source_language() {
        assert_eq!(
            text(Lang::En).errors.config_parse,
            en::EN.errors.config_parse
        );
    }

    #[test]
    fn zh_hans_is_a_distinct_full_translation() {
        // 每个语种都是独立的完整结构体，不存在"补丁合并"，
        // 译文完整性由类型系统保证——漏一个字段就编译不过。
        let zh = text(Lang::ZhHans);
        assert_ne!(zh.config.path_label, en::EN.config.path_label);
    }
}
