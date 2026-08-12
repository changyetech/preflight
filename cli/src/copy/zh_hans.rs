//! 简体中文——v1 必须译全（`ai-ipcheck` 的平价比对基线跑在这个语种下）。

use super::{
    CheckTextPatch, ChecksTextPatch, ConfigTextPatch, CoverageTextPatch, ErrorTextPatch,
    FailureTextPatch, LangTextPatch, NoteTextPatch, TextPatch, ValueTextPatch, VerdictTextPatch,
};

pub const ZH_HANS: TextPatch = TextPatch {
    lang: LangTextPatch {
        partial_notice: Some("该语种尚未译全，未翻译的条目显示英文。"),
    },

    config: ConfigTextPatch {
        path_label: Some("配置文件"),
        key_state_set: Some("proxycheck key：已设置"),
        key_state_unset: Some("proxycheck key：未设置"),
        key_prompt: Some("proxycheck API key（输入不回显）："),
        key_saved: Some("已保存。proxycheck 日配额从 100 次提升到 1000 次。"),
        key_empty: Some("未输入内容，key 未改动。"),
    },

    errors: ErrorTextPatch {
        config_read: Some("读不到配置文件"),
        config_parse: Some("配置文件不合法"),
        config_write: Some("写不了配置文件"),
        lang_unknown: Some("未知的语言"),
        lang_arabic_unsupported: Some(
            "终端下不支持阿拉伯语：右向左文本与左向右内容（IP 地址、IANA 时区名、ASN 编号）混排时，各终端模拟器的渲染并不一致。网页版支持阿拉伯语。",
        ),
        checkup_not_implemented: Some("体检没能跑起来。"),
    },

    verdict: VerdictTextPatch {
        low: Some("低风险"),
        medium: Some("中风险"),
        high: Some("高风险"),
        insufficient: Some("暂无结论"),
        preliminary_badge: Some("初步 · 未含 IP 风险评分"),
        full_badge: Some("完整 · 已含 IP 风险评分"),
        summary_insufficient: Some(
            "一项都没测成，给不出结论。这不等于没问题——请重试下面失败的检测项。",
        ),
        summary_preliminary_low: Some("目前未发现异常。IP 风险评分尚未纳入，因此这是初步结论。"),
        summary_preliminary_medium: Some("发现可疑信号，先看下面标出的项。"),
        summary_full_low: Some("各项均未发现异常。"),
        summary_full_medium: Some("发现可疑信号，先看下面标出的项。"),
        summary_full_high: Some("你的出口 IP 风险很高，现在用 AI 工具相当可能触发风控。"),
        exit_ip_label: Some("出口 IP"),
    },

    coverage: CoverageTextPatch {
        done: Some("已完成"),
        failed: Some("检测失败"),
    },

    checks: ChecksTextPatch {
        o1: CheckTextPatch {
            title: Some("出口 IP 与归属"),
            description: Some(
                "你的流量经代理后离开的公网地址，以及这个 IP 的归属地。这不是代理背后的地址。",
            ),
        },
        o2: CheckTextPatch {
            title: Some("系统时区一致性"),
            description: Some("比对系统时区与出口 IP 所在时区。不一致是风控能直接看到的破绽。"),
        },
        o3: CheckTextPatch {
            title: Some("IPv6 泄露"),
            description: Some(
                "大部分代理只处理 IPv4。IPv6 若可达，会额外暴露一个来自不同地区的地址。",
            ),
        },
        o4: CheckTextPatch {
            title: Some("IP 类型与风险"),
            description: Some(
                "出口 IP 是住宅还是机房、是否被标记为代理、风险评分，以及滥用举报记录。",
            ),
        },
        c1: CheckTextPatch {
            title: Some("本机真实 IP"),
            description: Some(
                "经国内直连回显取得。规则代理对国内 IP 走直连，因此即使开着 VPN 也能露出真实 ISP 出口。",
            ),
        },
        c2: CheckTextPatch {
            title: Some("本地 DNS 与 DNS 泄露"),
            description: Some("本机在用的 DNS 服务器。国内 DNS 可能暴露真实位置。"),
        },
        c3: CheckTextPatch {
            title: Some("代理检测"),
            description: Some("环境变量代理、系统代理与 TUN/VPN。只显示开关状态，不显示地址。"),
        },
        c4: CheckTextPatch {
            title: Some("$TZ 时区一致性"),
            description: Some("比对 $TZ 与出口 IP 所在时区。命令行工具实际跑在这个时区里。"),
        },
    },

    values: ValueTextPatch {
        checking: Some("检测中…"),
        unknown: Some("未知"),
        timezone_match: Some("一致"),
        timezone_mismatch: Some("不一致"),
        timezone_indeterminate: Some("无法比对"),
        ipv6_leaked: Some("泄露"),
        ipv6_disabled: Some("未启用"),
        proxy_env: Some("环境变量"),
        proxy_system: Some("系统代理"),
        proxy_tun: Some("TUN/VPN"),
        state_on: Some("已开启"),
        state_off: Some("未开启"),
        state_unsupported: Some("本平台未实现检测"),
        dns_router: Some("局域网路由器"),
        dns_domestic: Some("国内 DNS"),
        anonymous_flag: Some("该 IP 正被用作匿名化地址 —— 判高的阈值对它降到 51"),
        abuse_listed: Some("有滥用举报记录"),
        abuse_clean: Some("无滥用举报记录"),
        abuse_unknown: Some("滥用记录未知"),
    },

    failures: FailureTextPatch {
        upstream: Some("第三方不可用"),
        quota_exhausted: Some("proxycheck 当日配额已用尽"),
        local: Some("读不到本机环境"),
    },

    notes: NoteTextPatch {
        geo_source: Some(
            "归属数据来自 proxycheck；网页版用的是 Cloudflare 的地理库，两者可能对不上。",
        ),
        o2_desktop_only: Some(
            "本项对应图形界面应用（跟随系统时区）。命令行工具认的是 $TZ，那是下面的 C4。",
        ),
        quota_shared: Some(
            "无 key 时 proxycheck 每天 100 次，按出口 IP 计——与同一代理节点上的其他人共享。执行 `ipcheck config set proxycheck-key` 可提升到 1000 次。",
        ),
    },
};
