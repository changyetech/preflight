//! 文案层。与 ipcheck Web 的 `src/copy.ts` 同构：英文是**源语言**，两语种（en / zh-hans）
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
            $( $field:ident : $ty:ident ),* $(,)?
        }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name {
            $( pub $field: $ty, )*
        }
    };
}

mod en;
mod zh_hans;

use crate::lang::Lang;

copy_leaf! {
    /// `ipcheck config` 子命令的文案。
    ConfigText {
        path_label,
        key_state_set,
        key_state_unset,
        key_prompt,
        key_saved,
        key_empty,
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
        // 暂无调用点：删除回落机制前，clippy 的 dead-code 检查被 merge() 对全部
        // Patch 字段的模式匹配掩盖了，这条本就未被消费。留给 C1-C5 CLI 任务决定去留。
        #[allow(dead_code)]
        checkup_not_implemented,
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
        summary_preliminary_medium,
        summary_full_low,
        summary_full_medium,
        summary_full_high,
        exit_ip_label,
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

copy_node! {
    /// 10 个检测项。O1–O6 的标题必须与 ipcheck Web 逐字一致（契约 1.1）。
    ChecksText {
        o1: CheckText,
        o2: CheckText,
        o3: CheckText,
        o4: CheckText,
        o5: CheckText,
        o6: CheckText,
        c1: CheckText,
        c2: CheckText,
        c3: CheckText,
        c4: CheckText,
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
        // 暂无调用点，原因同上一条 checkup_not_implemented 的注释。
        #[allow(dead_code)]
        dns_domestic,
        anonymous_flag,
        abuse_listed,
        abuse_clean,
        abuse_unknown,
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
    /// 契约要求必须出现在屏幕上的说明，删掉就是回退。
    NoteText {
        /// 契约 5.4：归属来自 proxycheck，不是 Cloudflare，两端可能对不上。
        geo_source,
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
    }
}

copy_node! {
    /// 全部文案。
    Text {
        config: ConfigText,
        errors: ErrorText,
        verdict: VerdictText,
        coverage: CoverageText,
        checks: ChecksText,
        values: ValueText,
        failures: FailureText,
        notes: NoteText,
        dns_egress: DnsEgressText,
        udp_egress: UdpEgressText,
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
