//! 英文——全站**源语言**。其余语种按这里的结构对齐。
//!
//! 本文件必须完整：少写一个字段是编译错误，而不是运行时才发现某处掉回英文。
//!
//! O1–O4 的标题必须与 ipcheck Web 的 `src/locales/en.ts` **逐字一致**（契约 1.1）。

use super::{
    CheckText, ChecksText, ConfigText, CoverageText, ErrorText, FailureText, LangText, NoteText,
    Text, ValueText, VerdictText,
};

pub const EN: Text = Text {
    lang: LangText {
        partial_notice: "This language is only partially translated; untranslated items are shown in English.",
    },

    config: ConfigText {
        path_label: "Config file",
        key_state_set: "proxycheck key: set",
        // 只报「是否已设置」，绝不回显 key 本身。
        key_state_unset: "proxycheck key: not set",
        key_prompt: "proxycheck API key (input hidden):",
        key_saved: "Saved. Your daily proxycheck quota is now 1,000 instead of 100.",
        key_empty: "Nothing entered — the key was not changed.",
    },

    errors: ErrorText {
        config_read: "Cannot read the config file",
        config_parse: "The config file is invalid",
        config_write: "Cannot write the config file",
        lang_unknown: "Unknown language",
        // ar 是显式拒绝而不是静默回落：配了却看到英文，会让人以为工具坏了。
        lang_arabic_unsupported: "Arabic is not supported in the terminal: mixing right-to-left text with left-to-right values (IP addresses, IANA timezone names, ASN numbers) renders inconsistently across terminal emulators. The website does support Arabic.",
        checkup_not_implemented: "The checkup could not run.",
    },

    verdict: VerdictText {
        low: "Low risk",
        medium: "Medium risk",
        high: "High risk",
        insufficient: "No verdict yet",
        preliminary_badge: "Preliminary · IP risk score not included",
        full_badge: "Full · IP risk score included",
        summary_insufficient: "Nothing could be measured — no verdict. This is not a clean bill of health; retry the failed items below.",
        summary_preliminary_low: "No anomalies found so far. The IP risk score isn't included yet, so this verdict is preliminary.",
        summary_preliminary_medium: "Suspicious signals found — review the flagged items below.",
        summary_full_low: "No anomalies found in any check.",
        summary_full_medium: "Suspicious signals found — review the flagged items below.",
        summary_full_high: "Your exit IP is high risk. AI tools are quite likely to trigger anti-abuse controls right now.",
        exit_ip_label: "Exit IP",
    },

    coverage: CoverageText {
        done: "Done",
        failed: "Check failed",
    },

    checks: ChecksText {
        o1: CheckText {
            title: "Exit IP and ownership",
            description: "The public address your traffic leaves the proxy with, plus where the IP is registered. This is not the address behind the proxy.",
        },
        o2: CheckText {
            title: "System timezone match",
            description: "Compares your system timezone with the timezone of your exit IP. A mismatch is a visible inconsistency to anti-abuse systems.",
        },
        o3: CheckText {
            title: "IPv6 leak",
            description: "Most proxies only handle IPv4. If IPv6 is reachable, it can expose a second address from a different location.",
        },
        o4: CheckText {
            title: "IP type and risk",
            description: "Whether the exit IP is residential or a datacenter, whether it is flagged as a proxy, its risk score, and any abuse reports.",
        },
        c1: CheckText {
            title: "Real public IP",
            description: "Obtained from a domestic echo service that rule-based proxies route directly, so it reveals your real ISP exit even with a VPN running.",
        },
        c2: CheckText {
            title: "Local DNS and DNS leak",
            description: "Which DNS servers this machine uses. Domestic resolvers can reveal your real location.",
        },
        c3: CheckText {
            title: "Proxy detection",
            description: "Environment variables, system proxy and TUN/VPN. Only on/off is shown — never the addresses.",
        },
        c4: CheckText {
            title: "Claude Code CLI timezone match",
            description: "Compares $TZ with the timezone of your exit IP. This is the one Claude Code CLI actually reads.",
        },
        c5: CheckText {
            title: "Claude endpoint",
            description: "Whether ANTHROPIC_BASE_URL points at the official API, a domestic model provider, or a third-party relay.",
        },
    },

    values: ValueText {
        checking: "Checking…",
        unknown: "unknown",
        timezone_match: "match",
        timezone_mismatch: "mismatch",
        timezone_indeterminate: "cannot compare",
        ipv6_leaked: "leaking",
        ipv6_disabled: "disabled",
        proxy_env: "env vars",
        proxy_system: "system proxy",
        proxy_tun: "TUN/VPN",
        state_on: "on",
        state_off: "off",
        state_unsupported: "not supported on this platform",
        dns_router: "LAN router",
        dns_domestic: "domestic resolver",
        anonymous_flag: "currently used as an anonymising address — the high-risk threshold drops to 51 for this IP",
        abuse_listed: "abuse reports found",
        abuse_clean: "no abuse reports",
        abuse_unknown: "abuse reports unknown",
        endpoint_official: "official API",
        endpoint_domestic: "domestic model provider",
        endpoint_relay: "third-party relay — data leak and account risk",
        endpoint_not_installed: "Claude Code not installed",
        blacklist_hit: "on the Anthropic 147-domain list",
        blacklist_clear: "not on the Anthropic 147-domain list",
    },

    failures: FailureText {
        upstream: "third-party unavailable",
        quota_exhausted: "daily proxycheck quota used up",
        local: "could not read the local environment",
    },

    notes: NoteText {
        geo_source: "Ownership data comes from proxycheck. The website uses Cloudflare's database, so the two can disagree.",
        o2_desktop_only: "This one matches Claude Desktop. Claude Code CLI reads $TZ instead — that is C4 below.",
        quota_shared: "Without an API key proxycheck allows 100 queries per day, counted per exit IP — you share it with anyone else on the same proxy node. Run `ipcheck config set proxycheck-key` to raise it to 1,000.",
        blacklist_not_in_verdict: "The list is a frozen snapshot that can no longer be refreshed, so it warns but does not change the verdict.",
    },
};
