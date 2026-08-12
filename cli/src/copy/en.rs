//! 英文——全站**源语言**。其余语种按这里的结构对齐。
//!
//! 本文件必须完整：少写一个字段是编译错误，而不是运行时才发现某处掉回英文。
//!
//! O1–O6 的标题必须与 ipcheck Web 的 `src/locales/en.ts` **逐字一致**（契约 1.1）。

use super::{
    CheckText, ChecksText, ConfigText, CoverageText, DnsEgressText, ErrorText, FailureText,
    LangText, NoteText, Text, UdpEgressText, ValueText, VerdictText,
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
        o5: CheckText {
            title: "DNS Egress Leak",
            description: "Whether your DNS queries leave from the same country as your exit IP, based on the client subnet a public resolver reports back (EDNS Client Subnet).",
        },
        o6: CheckText {
            title: "UDP Egress Consistency",
            description: "Whether UDP traffic exits through the same address as the exit IP observed over TCP, using two independent STUN probes.",
        },
        c1: CheckText {
            title: "Real public IP",
            description: "Obtained from a domestic echo service that rule-based proxies route directly, so it reveals your real ISP exit even with a VPN running.",
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
    },

    failures: FailureText {
        upstream: "third-party unavailable",
        quota_exhausted: "daily proxycheck quota used up",
        local: "could not read the local environment",
    },

    notes: NoteText {
        geo_source: "Ownership data comes from proxycheck. The website uses Cloudflare's database, so the two can disagree.",
        o2_desktop_only: "This one covers GUI apps, which follow the system timezone. Command-line tools read $TZ instead — that is C4 below.",
        quota_shared: "Without an API key proxycheck allows 100 queries per day, counted per exit IP — you share it with anyone else on the same proxy node. Run `ipcheck config set proxycheck-key` to raise it to 1,000.",
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
    },

    udp_egress: UdpEgressText {
        reflexive_label: "UDP reflexive address",
        exit_label: "Exit IP",
        mismatch: "Your UDP traffic appears to exit from a different address than your exit IP — UDP may be bypassing the proxy.",
        no_mismatch: "Your UDP traffic appears to exit from the same address as your exit IP — no UDP egress mismatch detected.",
        family_mismatch: "Fewer than two reflexive addresses in the same address family as your exit IP (IPv4/IPv6) came back, so there's nothing to compare on equal terms.",
        unknown_exit_ip: "The exit IP isn't available yet (see the Exit IP check above), so this can't be compared.",
        stun_disagree: "The two STUN servers reported different addresses, so there's no single reliable value to compare — this can happen with multi-exit clusters or symmetric NAT.",
    },
};
