//! 文案层。与 ipcheck Web 的 `src/copy.ts` 同构：
//!
//! - 英文是**源语言**，`Text` 由 `en.rs` 完整填写——漏一个字段是编译错误
//! - 其余语种写 `TextPatch`（字段全 `Option`），缺失项在 `merge` 时**逐字段回落英文**
//! - 回落发生在字段粒度而非整份文件，因此补译可以一条一条来
//!
//! 命名与 Web 的对应：`Text` ↔ `Copy`，`TextPatch` ↔ `PartialCopy`。这里不叫 `Copy`
//! 是因为它会和 `std::marker::Copy` 撞名。
//!
//! 刻意不用 `rust-i18n` 之类的方案：它们的 key 是字符串，拼错或改名漏改只在运行时暴露，
//! 而 Web 侧明确花代价买下了"漏译是编译错误"这个性质（见 `src/locales/en.ts` 开头）。

/// 定义一组叶子文案：所有字段都是字符串。
macro_rules! copy_leaf {
    (
        $(#[$outer:meta])*
        $name:ident / $patch:ident {
            $( $(#[$fm:meta])* $field:ident ),* $(,)?
        }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name {
            $( $(#[$fm])* pub $field: &'static str, )*
        }

        #[doc = concat!("`", stringify!($name), "` 的译文补丁：给了的用译文，没给的回落英文。")]
        #[derive(Debug, Clone, Copy)]
        pub struct $patch {
            $( pub $field: Option<&'static str>, )*
        }

        impl $patch {
            /// 全部未翻译。补译时用 `..XxxPatch::DEFAULT` 只写译好的字段。
            pub const DEFAULT: Self = Self { $( $field: None, )* };
        }

        impl $name {
            pub const fn merge(self, patch: $patch) -> Self {
                Self {
                    $( $field: match patch.$field {
                        Some(v) => v,
                        None => self.$field,
                    }, )*
                }
            }
        }
    };
}

/// 定义一组由其他文案组合而成的节点。可任意嵌套。
macro_rules! copy_node {
    (
        $(#[$outer:meta])*
        $name:ident / $patch:ident {
            $( $field:ident : $ty:ident / $pty:ident ),* $(,)?
        }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name {
            $( pub $field: $ty, )*
        }

        #[derive(Debug, Clone, Copy)]
        pub struct $patch {
            $( pub $field: $pty, )*
        }

        impl $patch {
            pub const DEFAULT: Self = Self { $( $field: $pty::DEFAULT, )* };
        }

        impl $name {
            pub const fn merge(self, patch: $patch) -> Self {
                Self { $( $field: self.$field.merge(patch.$field), )* }
            }
        }
    };
}

mod en;
mod ru;
mod zh_hans;
mod zh_hant;

use crate::lang::Lang;

copy_leaf! {
    /// 语言相关的提示。
    LangText / LangTextPatch {
        /// 当前语种尚未译全时打的一行提示。
        partial_notice,
    }
}

copy_leaf! {
    /// `ipcheck config` 子命令的文案。
    ConfigText / ConfigTextPatch {
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
    ErrorText / ErrorTextPatch {
        config_read,
        config_parse,
        config_write,
        lang_unknown,
        lang_arabic_unsupported,
        checkup_not_implemented,
    }
}

copy_leaf! {
    /// 结论区。档位文案与「数据不足」是契约的硬性要求，不得被改版误删。
    VerdictText / VerdictTextPatch {
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
    CoverageText / CoverageTextPatch {
        done,
        failed,
    }
}

copy_leaf! {
    /// 一个检测项的标题与展开说明（`--verbose` 才出说明）。
    CheckText / CheckTextPatch {
        title,
        description,
    }
}

copy_node! {
    /// 9 个检测项。O1–O4 的标题必须与 ipcheck Web 逐字一致（契约 1.1）。
    ChecksText / ChecksTextPatch {
        o1: CheckText / CheckTextPatch,
        o2: CheckText / CheckTextPatch,
        o3: CheckText / CheckTextPatch,
        o4: CheckText / CheckTextPatch,
        c1: CheckText / CheckTextPatch,
        c2: CheckText / CheckTextPatch,
        c3: CheckText / CheckTextPatch,
        c4: CheckText / CheckTextPatch,
        c5: CheckText / CheckTextPatch,
    }
}

copy_leaf! {
    /// 检测项的取值与提示。
    ValueText / ValueTextPatch {
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
        endpoint_official,
        endpoint_domestic,
        endpoint_relay,
        endpoint_not_installed,
        blacklist_hit,
        blacklist_clear,
    }
}

copy_leaf! {
    /// 检测失败的四种原因。分开说是因为用户能采取的行动不同。
    FailureText / FailureTextPatch {
        upstream,
        quota_exhausted,
        local,
    }
}

copy_leaf! {
    /// 契约要求必须出现在屏幕上的说明，删掉就是回退。
    NoteText / NoteTextPatch {
        /// 契约 5.4：归属来自 proxycheck，不是 Cloudflare，两端可能对不上。
        geo_source,
        /// 契约 5.1：O2 对应桌面版，CC CLI 认 `$TZ`（那是 C4）。
        o2_desktop_only,
        /// ADR-0012：无 key 配额按出口 IP 计，与同节点用户共享。
        quota_shared,
        /// ADR-0010：黑名单命中只告警，不改变档位。
        blacklist_not_in_verdict,
    }
}

copy_node! {
    /// 全部文案。
    Text / TextPatch {
        lang: LangText / LangTextPatch,
        config: ConfigText / ConfigTextPatch,
        errors: ErrorText / ErrorTextPatch,
        verdict: VerdictText / VerdictTextPatch,
        coverage: CoverageText / CoverageTextPatch,
        checks: ChecksText / ChecksTextPatch,
        values: ValueText / ValueTextPatch,
        failures: FailureText / FailureTextPatch,
        notes: NoteText / NoteTextPatch,
    }
}

/// 取某语种的完整文案。英文直接返回源语言，其余合并补丁。
pub const fn text(lang: Lang) -> Text {
    match lang {
        Lang::En => en::EN,
        Lang::ZhHans => en::EN.merge(zh_hans::ZH_HANS),
        Lang::ZhHant => en::EN.merge(zh_hant::ZH_HANT),
        Lang::Ru => en::EN.merge(ru::RU),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_the_source_language() {
        // 英文没有补丁可合并，取到的就是源语言本身。
        assert_eq!(
            text(Lang::En).errors.config_parse,
            en::EN.errors.config_parse
        );
    }

    #[test]
    fn translated_fields_win_over_english() {
        let zh = text(Lang::ZhHans);
        assert_ne!(zh.config.path_label, en::EN.config.path_label);
    }

    #[test]
    fn untranslated_fields_fall_back_to_english_field_by_field() {
        // ru 只译了极少数字段，其余必须逐字段回落英文——而不是整份文件回落。
        let ru = text(Lang::Ru);
        assert_eq!(ru.errors.config_read, en::EN.errors.config_read);
        assert_ne!(ru.lang.partial_notice, en::EN.lang.partial_notice);
    }
}
