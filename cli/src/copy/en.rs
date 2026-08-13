//! 英文——全站**源语言**。其余语种按这里的结构对齐。
//!
//! 本文件必须完整：少写一个字段是编译错误，而不是运行时才发现某处掉回英文。
//!
//! O1–O6 的标题必须与 Preflight Web 的 `src/locales/en.ts` **逐字一致**（契约 1.1）。

use super::{
    C4FixText, CheckText, ChecksText, ConfigText, CoverageText, DnsCommandText, DnsEgressText,
    ErrorText, FailureText, FooterText, GroupText, NoteText, O1FieldsText, Text, UdpEgressText,
    ValueText, VerdictText,
};

pub const EN: Text = Text {
    config: ConfigText {
        path_label: "Config file",
        key_state_set: "proxycheck key: set",
        // 只报「是否已设置」，绝不回显 key 本身。
        key_state_unset: "proxycheck key: not set",
        key_prompt: "proxycheck API key (input hidden):",
        key_saved: "Saved. Your daily proxycheck quota is now 1,000 instead of 100.",
        key_empty: "Nothing entered — the key was not changed.",
        list_value_set: "set",
        list_value_unset: "not set",
    },

    errors: ErrorText {
        config_read: "Cannot read the config file",
        config_parse: "The config file is invalid",
        config_write: "Cannot write the config file",
        lang_unknown: "Unsupported language",
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
        summary_preliminary_low_reminders: "Nothing that counts toward the verdict so far; some items below are flagged for awareness only. The IP risk score isn't included yet, so this verdict is preliminary.",
        summary_preliminary_medium: "Suspicious signals found — review the flagged items below.",
        summary_full_low: "No anomalies found in any check.",
        summary_full_low_reminders: "Nothing that counts toward the verdict was found. Some items below are flagged for awareness only.",
        summary_full_medium: "Suspicious signals found — review the flagged items below.",
        summary_full_high: "Your exit IP is high risk. IP-sensitive services are quite likely to flag you right now.",
        exit_ip_label: "Exit IP",
        risk_label: "Risk score",
        coverage_label: "Coverage",
        attention_label: "Needs attention",
        // 分词形式，与 `attention_reminder_only` 同构：ID 列表可以是一项也可以是多项，
        // 限定动词（counts / count）会在其中一侧写错主谓一致。
        attention_contributing: "counted toward the verdict",
        attention_reminder_only: "flagged for awareness only",
        attention_list_separator: ", ",
        attention_list_connector: " and ",
        attention_prefix: "Only ",
        attention_clause_separator: "; ",
        attention_suffix: ".",
    },

    coverage: CoverageText {
        done: "Done",
        failed: "Check failed",
    },

    groups: GroupText {
        online: "Online checks",
        local: "Local checks",
        items: "checks",
        all_done: "all complete",
        local_only: "not possible in the browser",
    },

    checks: ChecksText {
        o1: CheckText {
            title: "Exit IP and Ownership",
            description: "The public address your traffic leaves the proxy with, plus where the IP is registered. This is not the address behind the proxy.",
        },
        o1_fields: O1FieldsText {
            address: "Address",
            ownership: "Ownership",
            network: "Network",
        },
        o2: CheckText {
            title: "System Timezone Consistency",
            description: "Compares your system timezone with the timezone of your exit IP. A mismatch is a visible inconsistency to anti-abuse systems.",
        },
        o3: CheckText {
            title: "IPv6 Leak",
            description: "Most proxies only handle IPv4. If IPv6 is reachable, it can expose a second address from a different location.",
        },
        o4: CheckText {
            title: "IP Type & Risk",
            description: "Whether the exit IP is residential or a datacenter, whether it is flagged as a proxy, its risk score, and any abuse reports.",
        },
        o5: CheckText {
            title: "DNS Egress Leak",
            description: "Whether your DNS queries leave from the same country as your exit IP, based on the client subnet a public resolver reports back (EDNS Client Subnet).",
        },
        o6: CheckText {
            title: "UDP Egress Consistency",
            description: "Whether UDP traffic exits through the same address as the exit IP observed over TCP, using two independent STUN probes.",
        },
        c1: CheckText {
            title: "Real Public IP and Ownership",
            description: "Obtained from an echo service in mainland China, which rule-based proxies route directly — so it reveals your real ISP exit even with a VPN running. If your proxy rules don't route mainland China directly, this simply shows your exit IP again.",
        },
        c2: CheckText {
            // 契约收缩：DNS 泄露判定拆到 O5，本项只剩「本地 DNS 服务器配置」（ADR-0014）。
            title: "Local DNS Server Configuration",
            description: "Which DNS servers this machine uses. Domestic resolvers can reveal your real location. Whether those queries actually leave through the proxy is a separate check, see O5.",
        },
        c3: CheckText {
            title: "Proxy detection",
            description: "Environment variables, system proxy and TUN/VPN. Only on/off is shown — never the addresses.",
        },
        c4: CheckText {
            title: "$TZ timezone match",
            description: "Compares $TZ with the timezone of your exit IP. This is the timezone command-line tools actually run in.",
        },
        c4_fix: C4FixText {
            explain_prefix: "Command-line tools actually run in ",
            explain_connector: ", but anti-abuse systems see your exit IP's timezone as ",
            explain_suffix: ".",
            fix_label: "Suggested fix",
            fix_command_prefix: "export TZ=",
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
        obtained: "Obtained",
        risk_scale_note: "Medium from 26 · High from 76",
        risk_level_low: "low",
        risk_level_medium: "medium",
        risk_level_high: "high",
        reference_only: "Reference only",
        tz_system_label: "System timezone",
        tz_exit_label: "Exit IP timezone",
        network_type_label: "IP type",
        detections_label: "Proxy detections",
        abuse_label: "Abuse reports",
    },

    failures: FailureText {
        upstream: "third-party unavailable",
        quota_exhausted: "daily proxycheck quota used up",
        local: "could not read the local environment",
    },

    notes: NoteText {
        geo_source: "Ownership data comes from proxycheck. The website uses Cloudflare's database, so the two can disagree.",
        geo_source_local: "Ownership data comes from proxycheck, the same source as O1 above — so the two are directly comparable.",
        o2_desktop_only: "This one covers GUI apps, which follow the system timezone. Command-line tools read $TZ instead — that is C4 below.",
        quota_shared: "Without an API key proxycheck allows 100 queries per day, counted per exit IP — you share it with anyone else on the same proxy node. Run `preflight config set proxycheck-key` to raise it to 1,000.",
    },

    footer: FooterText {
        verbose_hint: "explain every check",
        json_hint: "machine-readable output",
        quota_hint: "proxycheck quota 100/day → 1,000/day",
    },

    dns_egress: DnsEgressText {
        resolver_label: "Resolver location",
        ecs_label: "DNS client subnet country",
        exit_label: "Exit IP country",
        leak: "Your DNS queries appear to leave from a different country than your exit IP — DNS may be bypassing the proxy.",
        no_leak: "Your DNS queries appear to leave from the same country as your exit IP — no DNS egress leak detected.",
        no_ecs: "Your DNS provider doesn't send EDNS Client Subnet data, so this can't determine whether your DNS queries are proxied.",
        unmapped_country: "Your resolver reported a client-subnet location we don't recognize, so this can't be compared.",
        unknown_exit_country: "The exit IP's country isn't available yet (see the Exit IP check above), so this can't be compared.",
        // 契约 2.1／2.5 硬约束 1：resolver 归属只展示，不参与判定——只有 ECS 判定进综合结论。
        resolver_note: "Shown for reference only — it doesn't affect the verdict. Which country your resolver sits in depends on which DNS provider you picked, not on whether your traffic is proxied.",
        state_leaked: "leaked",
        state_not_leaked: "not leaked",
    },

    udp_egress: UdpEgressText {
        reflexive_label: "UDP reflexive address",
        exit_label: "Exit IP",
        mismatch: "Your UDP traffic appears to exit from a different address than your exit IP — UDP may be bypassing the proxy.",
        no_mismatch: "Your UDP traffic appears to exit from the same address as your exit IP — no UDP egress mismatch detected.",
        family_mismatch: "Fewer than two reflexive addresses in the same address family as your exit IP (IPv4/IPv6) came back, so there's nothing to compare on equal terms.",
        unknown_exit_ip: "The exit IP isn't available yet (see the Exit IP check above), so this can't be compared.",
        stun_disagree: "The two STUN servers reported different addresses, so there's no single reliable value to compare — this can happen with multi-exit clusters or symmetric NAT.",
        state_match: "match",
        state_mismatch: "mismatch",
    },

    dns_cmd: DnsCommandText {
        col_ip: "IP",
        col_provider: "Provider",
        col_region: "Region",
        col_domestic: "CN",
        col_variant: "Filter",
        col_latency: "Latency",
        col_status: "Status",
        variant_standard: "Standard",
        variant_security: "Security",
        variant_family: "Family",
        variant_adblock: "Ad-block",
        domestic_yes: "CN",
        check_ok: "OK",
        check_suspicious: "Suspicious",
        check_unreachable: "Unreachable",
        footer_hint: "Run `preflight dns --check` to test which servers actually work from here.",
    },
};
