//! 简体中文——与英文同构的完整文案（ADR-0016：语种收缩为 en + zh-hans，
//! 结构体强制译全，不存在字段级回落）。

use super::{
    C4FixText, CheckText, ChecksText, ConfigText, CoverageText, DnsEgressText, ErrorText,
    FailureText, FooterText, GroupText, NoteText, O1FieldsText, Text, UdpEgressText, ValueText,
    VerdictText,
};

pub const ZH_HANS: Text = Text {
    config: ConfigText {
        path_label: "配置文件",
        key_state_set: "proxycheck key：已设置",
        key_state_unset: "proxycheck key：未设置",
        key_prompt: "proxycheck API key（输入不回显）：",
        key_saved: "已保存。proxycheck 日配额从 100 次提升到 1000 次。",
        key_empty: "未输入内容，key 未改动。",
        list_value_set: "已设置",
        list_value_unset: "未设置",
    },

    errors: ErrorText {
        config_read: "读不到配置文件",
        config_parse: "配置文件不合法",
        config_write: "写不了配置文件",
        lang_unknown: "不支持的语言",
    },

    verdict: VerdictText {
        low: "低风险",
        medium: "中风险",
        high: "高风险",
        insufficient: "暂无结论",
        preliminary_badge: "初步 · 未含 IP 风险评分",
        full_badge: "完整 · 已含 IP 风险评分",
        summary_insufficient: "一项都没测成，给不出结论。这不等于没问题——请重试下面失败的检测项。",
        summary_preliminary_low: "目前未发现异常。IP 风险评分尚未纳入，因此这是初步结论。",
        summary_preliminary_low_reminders: "目前未发现影响综合结论的异常；下面有仅作提醒的项。IP 风险评分尚未纳入，因此这是初步结论。",
        summary_preliminary_medium: "发现可疑信号，先看下面标出的项。",
        summary_full_low: "各项均未发现异常。",
        summary_full_low_reminders: "未发现影响综合结论的异常。下面有仅作提醒的项，它们不改变结论。",
        summary_full_medium: "发现可疑信号，先看下面标出的项。",
        summary_full_high: "你的出口 IP 风险很高，现在用对 IP 敏感的服务相当可能触发风控。",
        exit_ip_label: "出口 IP",
        risk_label: "风险评分",
        coverage_label: "覆盖度",
        attention_label: "需关注",
        attention_contributing: "参与综合结论判定",
        attention_reminder_only: "仅作提醒",
        attention_list_separator: "、",
        attention_list_connector: " 与 ",
        attention_prefix: "其中只有 ",
        attention_clause_separator: "，",
        attention_suffix: "。",
    },

    coverage: CoverageText {
        done: "已完成",
        failed: "检测失败",
    },

    groups: GroupText {
        online: "联网检测",
        local: "本机检测",
        items: "项",
        all_done: "全部完成",
        local_only: "网页版做不到",
    },

    checks: ChecksText {
        o1: CheckText {
            title: "出口 IP 与归属",
            description: "你的流量经代理后离开的公网地址，以及这个 IP 的归属地。这不是代理背后的地址。",
        },
        o1_fields: O1FieldsText {
            address: "地址",
            ownership: "归属",
            network: "网络",
        },
        o2: CheckText {
            title: "系统时区一致性",
            description: "比对系统时区与出口 IP 所在时区。不一致是风控能直接看到的破绽。",
        },
        o3: CheckText {
            title: "IPv6 泄露",
            description: "大部分代理只处理 IPv4。IPv6 若可达，会额外暴露一个来自不同地区的地址。",
        },
        o4: CheckText {
            title: "IP 类型与风险",
            description: "出口 IP 是住宅还是机房、是否被标记为代理、风险评分，以及滥用举报记录。",
        },
        o5: CheckText {
            title: "DNS 出口泄露",
            description: "你的 DNS 查询是否与出口 IP 从同一国家出网，判据是公共 resolver 回传的客户端子网（EDNS Client Subnet）。",
        },
        o6: CheckText {
            title: "UDP 出口一致性",
            description: "UDP 流量的出口地址是否与 TCP 观测到的出口 IP 一致，用两个独立的 STUN 探测互相印证。",
        },
        c1: CheckText {
            title: "本机真实 IP",
            description: "经中国大陆的直连回显服务取得。规则代理默认对大陆 IP 走直连，因此即使开着 VPN 也能露出真实 ISP 出口；分流规则不含大陆直连时，本项只会再次显示出口 IP。",
        },
        c2: CheckText {
            // 契约收缩：DNS 泄露判定拆到 O5，本项只剩「本地 DNS 服务器配置」（ADR-0014）。
            title: "本地 DNS 服务器配置",
            description: "本机在用的 DNS 服务器。国内 DNS 可能暴露真实位置。查询是否真的经代理出网是另一项检测，见 O5。",
        },
        c3: CheckText {
            title: "代理检测",
            description: "环境变量代理、系统代理与 TUN/VPN。只显示开关状态，不显示地址。",
        },
        c4: CheckText {
            title: "$TZ 时区一致性",
            description: "比对 $TZ 与出口 IP 所在时区。命令行工具实际跑在这个时区里。",
        },
        c4_fix: C4FixText {
            explain_prefix: "命令行工具实际跑在 ",
            explain_connector: "，风控看到的出口却在 ",
            explain_suffix: "。",
            fix_label: "建议",
            fix_command_prefix: "export TZ=",
        },
    },

    values: ValueText {
        checking: "检测中…",
        unknown: "未知",
        timezone_match: "一致",
        timezone_mismatch: "不一致",
        timezone_indeterminate: "无法比对",
        ipv6_leaked: "泄露",
        ipv6_disabled: "未启用",
        proxy_env: "环境变量",
        proxy_system: "系统代理",
        proxy_tun: "TUN/VPN",
        state_on: "已开启",
        state_off: "未开启",
        state_unsupported: "本平台未实现检测",
        dns_router: "局域网路由器",
        dns_domestic: "国内 DNS",
        anonymous_flag: "该 IP 正被用作匿名化地址 —— 判高的阈值对它降到 51",
        abuse_listed: "有滥用举报记录",
        abuse_clean: "无滥用举报记录",
        abuse_unknown: "滥用记录未知",
        obtained: "已取得",
        risk_scale_note: "26 起为中 · 76 起为高",
        risk_level_low: "低",
        risk_level_medium: "中",
        risk_level_high: "高",
        reference_only: "仅供参考",
        tz_system_label: "系统时区",
        tz_exit_label: "出口 IP 时区",
        network_type_label: "IP 类型",
        detections_label: "代理检出",
        abuse_label: "滥用举报",
    },

    failures: FailureText {
        upstream: "第三方不可用",
        quota_exhausted: "proxycheck 当日配额已用尽",
        local: "读不到本机环境",
    },

    notes: NoteText {
        geo_source: "归属数据来自 proxycheck；网页版用的是 Cloudflare 的地理库，两者可能对不上。",
        o2_desktop_only: "本项对应图形界面应用（跟随系统时区）。命令行工具认的是 $TZ，那是下面的 C4。",
        quota_shared: "无 key 时 proxycheck 每天 100 次，按出口 IP 计——与同一代理节点上的其他人共享。执行 `preflight config set proxycheck-key` 可提升到 1000 次。",
    },

    footer: FooterText {
        verbose_hint: "逐项说明",
        json_hint: "机器可读输出",
        quota_hint: "proxycheck 日配额 100 → 1000",
    },

    dns_egress: DnsEgressText {
        resolver_label: "resolver 归属",
        ecs_label: "DNS 客户端子网归属国",
        exit_label: "出口 IP 归属国",
        leak: "你的 DNS 查询似乎从与出口 IP 不同的国家出网，DNS 可能正在绕过代理。",
        no_leak: "你的 DNS 查询似乎与出口 IP 从同一国家出网，未检测到 DNS 出口泄露。",
        no_ecs: "你的 DNS 服务商不发送 ECS，无法判定 DNS 查询是否走代理。",
        unmapped_country: "resolver 返回了客户端子网归属地，但我们暂时认不出这个国家名，无法比对。",
        unknown_exit_country: "出口 IP 的归属国尚未取得（见上方「出口 IP 与归属」），暂时无法比对。",
        resolver_note: "仅供参考，不参与判定：resolver 在哪个国家取决于你选了哪家 DNS 服务商，与流量是否走代理无关。",
        state_leaked: "已泄露",
        state_not_leaked: "未泄露",
    },

    udp_egress: UdpEgressText {
        reflexive_label: "UDP 反射地址",
        exit_label: "出口 IP",
        mismatch: "你的 UDP 流量似乎从与出口 IP 不同的地址出网，UDP 可能正在绕过代理。",
        no_mismatch: "你的 UDP 流量似乎与出口 IP 从同一地址出网，未检测到 UDP 出口不一致。",
        family_mismatch: "与出口 IP 同协议族（IPv4/IPv6）的可比对反射地址不足两个，没有可以在同一基准上比较的对象。",
        unknown_exit_ip: "出口 IP 尚未取得（见上方「出口 IP 与归属」），暂时无法比对。",
        stun_disagree: "两个 STUN 服务器给出的地址不一致，没有一个可信的单一值可比——常见于多出口集群或对称 NAT。",
        state_match: "一致",
        state_mismatch: "不一致",
    },
};
