// 全站文案集中在此，不散落到 JSX 里（规格第 7 节）。
// 纯字符串模块，不引任何 DOM 依赖，因此可被单测直接断言（部分文案是 ADR 的硬性要求，不得被改版误删）。
//
// 中英双语：ZH 是既有文案（默认版本），EN 是逐字段翻译，`typeof ZH` 约束 EN 结构必须完全对齐——
// 漏翻一个字段会直接报编译错误，而不是运行时才发现某处英文页面掉回中文。

const ZH = {
  site: {
    title: "ipcheck · 网络环境体检",
    tagline: "开跑 Claude 之前，先看清自己的网络长什么样",
  },

  nav: {
    /** 右上角语言切换（规格第 7 节）：链接文案是目标语言的自称，不是当前语言。 */
    switchTo: "English",
  },

  verdict: {
    level: {
      low: "低风险",
      medium: "中风险",
      high: "高风险",
    },
    /** 数据不足时的档位文案。绝不能落到「低风险」——没测成不是安全。 */
    insufficientLabel: "暂无结论",
    /** 初步结论必须带此标注（ADR-0004 / ADR-0005，验收标准 4）。 */
    preliminaryBadge: "初步 · 未含 IP 风险评分",
    fullBadge: "完整 · 已含 IP 风险评分",
    summary: {
      insufficient:
        "自动检测项尚未完成，或全部未能完成，暂时无法给出结论。请稍候，或重试下方标为「检测失败」的项。",
      preliminaryLow:
        "自动检测项未发现异常。IP 风险评分尚未纳入，结论仅供参考。",
      preliminaryMedium: "自动检测项发现可疑信号，建议先处理下方标黄的项。",
      fullLow: "各项检测均未发现异常。",
      fullMedium: "存在可疑信号，建议逐项核对下方标黄的项。",
      fullHigh: "出口 IP 风险很高，此时使用 AI 工具有较大概率触发风控。",
    },
    exitIpLabel: "出口 IP",
    exitIpUnknown: "未能取得",
    /** 「真实 IP」一词禁用（CONTEXT.md）。 */
    exitIpNote: "这是你流量离开代理后的公网地址，不是代理背后的本机地址。",
  },

  coverage: {
    done: "已完成",
    needCli: "需 CLI",
    failed: "检测失败",
    pending: "按需未测",
    total: "共 9 项",
    hint: "结论只覆盖已完成的项。需 CLI 的项本站结构性做不到，装 CLI 才能测。",
  },

  cardStatus: {
    idle: "未开始",
    running: "检测中",
    done: "已完成",
    failed: "检测失败",
    needCli: "需 CLI",
  },

  actions: {
    retry: "重试",
    copy: "复制",
    copied: "已复制",
    installCommand: "pip install ai-ipcheck",
    meaningLabel: "这意味着什么",
  },

  checks: {
    O1: {
      title: "出口 IP 与归属",
      meaning:
        "这是所有网站（包括 Claude）看到的你。归属地与你实际所在地差得越远，越容易被判为异常流量。",
      fields: {
        location: "归属地",
        asn: "运营商 / ASN",
        timezone: "IP 时区",
        colo: "接入节点",
      },
      unknown: "未知",
    },
    O2: {
      title: "系统时区一致性",
      match: "系统时区与出口 IP 时区一致。",
      mismatch: "系统时区与出口 IP 时区不一致。",
      unknown: "边缘未给出出口 IP 的时区，本项无法比对。",
      meaning:
        "时区对不上是最常见的代理特征之一：IP 显示在美国、系统却是北京时间，风控一眼就能看出来。",
      /** 必须显式区分两个时区来源（规格 2.2 / 验收标准 2）。 */
      scopeNote:
        "本项读的是浏览器时区，它跟随系统时区，因此对应的是 Claude 桌面版。Claude Code CLI 认的是环境变量 $TZ，网页读不到——那一项是 C4，需要装 CLI 才能测。",
      browserLabel: "系统（浏览器）时区",
      exitLabel: "出口 IP 时区",
    },
    O3: {
      title: "IPv6 泄露",
      leak: "检测到 IPv6 出口，你的 IPv6 流量正绕过代理直连。",
      disabled: "未检测到 IPv6 出口，不存在 IPv6 泄露。",
      meaning:
        "多数代理只接管 IPv4。一旦本机有 IPv6，部分流量会绕过代理直接出去，暴露另一个地区的地址——你以为在用美国 IP，对方却同时看到了你家宽带的 IPv6。",
      ipv6Label: "IPv6 出口地址",
      failed:
        "IPv6 对照探测未能完成，本项无法判定。这不代表你没有 IPv6——网络恢复后可重试。",
    },
    O4: {
      title: "IP 类型与风险",
      idle: "按需检测项，需要你手动触发。",
      /** ADR-0008：第三方调用必须写在触发它的控件上。文案改动前先读 ADR-0008。 */
      consentButton:
        "检测 IP 风险（将把你的出口 IP 发送至 proxycheck.io 查询）",
      consentNote:
        "同时会向 StopForumSpam 查询该 IP 是否有滥用收录。本站不存储任何查询结果。",
      meaning:
        "机房 IP、公开代理、被大量滥用过的 IP，是触发风控最直接的原因。风险分越高，越容易在登录或调用时被拦。",
      fields: {
        networkType: "网络类型",
        riskScore: "风险分",
        detections: "代理检出",
        abuse: "滥用收录",
      },
      networkType: {
        Residential: "住宅宽带",
        Business: "企业专线",
        Wireless: "移动网络",
        Hosting: "机房 / 云主机",
        unknown: "未知",
      },
      detectionLabels: {
        proxy: "代理",
        vpn: "VPN",
        tor: "Tor",
        scraper: "爬虫",
      },
      noDetection: "未检出",
      hostingNote:
        "机房 IP 属于分项提醒，本身不拉高综合结论，但确实更容易被风控盯上。",
      abuse: {
        listed: "有收录",
        clean: "无收录",
        /** 第三方不可用时必须显示「未知」，不得冒充「无收录」（docs/api.md 3.1）。 */
        unknown: "未知（数据源不可用）",
      },
      quotaExhausted:
        "今日额度已用尽。本项按「检测失败」计入覆盖度，明日 UTC 零点后恢复；其余各项与初步结论不受影响。",
      turnstileMissing: "人机验证组件未配置，本项暂不可用。",
    },
    C1: {
      title: "本机真实 IP",
      meaning:
        "国内直连回显得到的本机公网地址。网页只能看到出口 IP，看不到代理背后的这一个——这是能力边界，不是没做。",
    },
    C2: {
      title: "本地 DNS 服务器与 DNS 泄露",
      meaning:
        "DNS 查询走的是哪台服务器，网页拿不到查询日志，结构性测不了。DNS 泄露会把你访问过的域名暴露给本地运营商。",
    },
    C3: {
      title: "代理检测（环境变量 / 系统代理 / TUN）",
      meaning:
        "需要读取本机环境变量与系统代理配置。CLI 能看出「你以为开了代理其实没生效」这种情况。",
    },
    C4: {
      title: "Claude Code CLI 时区一致性",
      meaning:
        "CC CLI 认的是环境变量 $TZ，与系统时区可以不同。网页读不到 $TZ——上面的 O2 测的是系统时区，覆盖不了这一项。",
    },
    C5: {
      title: "Claude 端点检测",
      meaning:
        "官方直连 / 国产大模型 / 中转站及黑名单命中。需要读取本机的 ANTHROPIC_BASE_URL 配置，网页读不到。这是 CLI 里唯一能判「高风险」的项。",
    },
  },

  cli: {
    /** 灰卡是终态，不提供重试（规格 4.1）。 */
    hint: "本项需要读取本机环境，装 CLI 后可测：",
  },

  /** 落地内容三段（规格第 4 节第 3 项 / --content 计划步骤 1-3）。 */
  landing: {
    why: {
      title: "为什么需要体检",
      body: "AI 工具对访问环境很敏感，最容易踩雷的集中在四类：出口 IP 类型与历史滥用记录过高触发风控、系统与出口 IP 时区不一致露出破绽、IPv6 悄悄绕过代理暴露真实位置、本地 DNS 把你访问过的域名暴露给本地运营商。本站可在线检测前三类；DNS 泄露需要读取本机 DNS 查询日志，网页结构性拿不到，属于仅 CLI 项。",
    },
    install: {
      title: "安装 CLI 补全全部 9 项",
      body: "网页版是快速摸底，能测 4 项；CLI 覆盖全部 9 项，包括本机真实 IP、DNS 泄露、代理与 TUN 检测、Claude Code 时区一致性与端点检测。",
    },
    compare: {
      title: "Web 与 CLI 完整功能对照表",
      columnId: "编号",
      columnItem: "检测项",
      columnWeb: "网页",
      columnCli: "CLI",
      auto: "自动",
      onDemand: "按需",
      cliOnly: "仅 CLI",
      dash: "—",
    },
  },

  footer: {
    privacy: "本站不存储任何检测结果。",
    thirdParty:
      "IPv6 检测由浏览器直接访问 ipify；IP 风险检测需你手动触发，届时出口 IP 会被发送至 proxycheck.io 与 StopForumSpam。",
  },

  errors: {
    network: "网络请求失败，请稍后重试。",
    /** 1001 / 4001：前端请求本身不合法，重试不会好转，措辞里不给「稍后重试」的暗示。 */
    badRequest: "请求不合法，本项无法完成。若刷新后仍然如此，请反馈给我们。",
    malformed: "接口返回的数据不完整，本项按检测失败处理。",
    rateLimited: "请求过于频繁，请稍后重试。",
    humanVerification: "人机验证未通过，请重试。",
    upstream: "数据源不可用，本项未能完成检测。",
    clientIp: "未能确定你的出口 IP。",
    unknown: "检测失败，请稍后重试。",
  },
} as const;

/**
 * 把 ZH 的字面量类型（`"出口 IP"` 这种具体字符串）拓宽成 `string`。
 * EN 按 `Copy`（而非 `typeof ZH`）类型检查，这样英文译文不必凑巧等于某个中文字面量，
 * 同时保留结构约束：EN 漏翻、多翻一个字段照样是编译错误。
 */
type Widen<T> = T extends string
  ? string
  : T extends readonly (infer U)[]
    ? readonly Widen<U>[]
    : { [K in keyof T]: Widen<T[K]> };

export type Copy = Widen<typeof ZH>;
export type Lang = "zh" | "en";

/** 英文版。术语对齐 CLI `README_EN.md`：Exit IP / Overall Verdict / Coverage（--content 计划步骤 7）。 */
const EN: Copy = {
  site: {
    title: "ipcheck · Network Environment Checkup",
    tagline: "Know your network before you launch Claude",
  },

  nav: {
    switchTo: "中文",
  },

  verdict: {
    level: {
      low: "Low risk",
      medium: "Medium risk",
      high: "High risk",
    },
    insufficientLabel: "No verdict yet",
    preliminaryBadge: "Preliminary · IP risk score not included",
    fullBadge: "Full · IP risk score included",
    summary: {
      insufficient:
        'Automated checks haven\'t finished, or all of them failed — no verdict yet. Please wait, or retry any item marked "Check failed" below.',
      preliminaryLow:
        "No anomalies found by the automated checks. IP risk score isn't included yet, so this verdict is preliminary.",
      preliminaryMedium:
        "The automated checks found suspicious signals — review the items marked yellow below first.",
      fullLow: "No anomalies found in any check.",
      fullMedium:
        "Suspicious signals found — review the items marked yellow below.",
      fullHigh:
        "Your exit IP is high risk. AI tools are quite likely to trigger anti-abuse controls right now.",
    },
    exitIpLabel: "Exit IP",
    exitIpUnknown: "Unavailable",
    exitIpNote:
      "This is the public address your traffic leaves the proxy with, not the machine address behind the proxy.",
  },

  coverage: {
    done: "Done",
    needCli: "Needs CLI",
    failed: "Check failed",
    pending: "Not run yet",
    total: "9 items total",
    hint: "The verdict only covers completed items. Items that need the CLI are structurally out of reach for a webpage — install the CLI to test them.",
  },

  cardStatus: {
    idle: "Not started",
    running: "Checking",
    done: "Done",
    failed: "Check failed",
    needCli: "Needs CLI",
  },

  actions: {
    retry: "Retry",
    copy: "Copy",
    copied: "Copied",
    installCommand: "pip install ai-ipcheck",
    meaningLabel: "What this means",
  },

  checks: {
    O1: {
      title: "Exit IP Info",
      meaning:
        "This is what every website — including Claude — sees as you. The further this location is from where you actually are, the more likely your traffic gets flagged as anomalous.",
      fields: {
        location: "Location",
        asn: "ISP / ASN",
        timezone: "IP timezone",
        colo: "Edge colo",
      },
      unknown: "Unknown",
    },
    O2: {
      title: "System Timezone Consistency",
      match: "System timezone matches the exit IP timezone.",
      mismatch: "System timezone does not match the exit IP timezone.",
      unknown:
        "The edge didn't return a timezone for the exit IP — can't compare.",
      meaning:
        "A timezone mismatch is one of the most common proxy tells: the IP says United States, but the system clock says Beijing time — anti-abuse controls catch that instantly.",
      scopeNote:
        "This item reads the browser timezone, which follows the system timezone — so it corresponds to the Claude desktop app. Claude Code CLI honors the $TZ environment variable, which a webpage cannot read — that's C4, and it needs the CLI installed to test.",
      browserLabel: "System (browser) timezone",
      exitLabel: "Exit IP timezone",
    },
    O3: {
      title: "IPv6 Leak",
      leak: "IPv6 exit detected — your IPv6 traffic is bypassing the proxy and connecting directly.",
      disabled: "No IPv6 exit detected — no IPv6 leak.",
      meaning:
        "Most proxies only handle IPv4. If your machine has IPv6, some traffic slips past the proxy and goes out directly, exposing an address from a different location — you think you're on a US IP, but the other side also sees your home broadband's IPv6.",
      ipv6Label: "IPv6 exit address",
      failed:
        "The IPv6 comparison probe didn't complete — this item can't be determined. This does not mean you have no IPv6 — retry once your network is back.",
    },
    O4: {
      title: "IP Type & Risk",
      idle: "On-demand check — you need to trigger it manually.",
      consentButton: "Check IP risk (sends your exit IP to proxycheck.io)",
      consentNote:
        "This also queries StopForumSpam for abuse records on that IP. This site stores none of the query results.",
      meaning:
        "Datacenter IPs, public proxies, and heavily abused IPs are the most direct trigger for anti-abuse controls. The higher the risk score, the more likely you are to get blocked at login or on requests.",
      fields: {
        networkType: "Network type",
        riskScore: "Risk score",
        detections: "Proxy detections",
        abuse: "Abuse records",
      },
      networkType: {
        Residential: "Residential",
        Business: "Business",
        Wireless: "Wireless",
        Hosting: "Datacenter / Hosting",
        unknown: "Unknown",
      },
      detectionLabels: {
        proxy: "Proxy",
        vpn: "VPN",
        tor: "Tor",
        scraper: "Scraper",
      },
      noDetection: "None detected",
      hostingNote:
        "A datacenter IP is a per-item flag — it doesn't raise the overall verdict by itself, but it does draw more attention from anti-abuse controls.",
      abuse: {
        listed: "Listed",
        clean: "Clean",
        unknown: "Unknown (data source unavailable)",
      },
      quotaExhausted:
        'Today\'s quota is exhausted. This item counts as "Check failed" toward coverage and resets after UTC midnight; other items and the preliminary verdict are unaffected.',
      turnstileMissing:
        "The human-verification widget isn't configured — this item is unavailable for now.",
    },
    C1: {
      title: "Real Public IP",
      meaning:
        "Your real public IP, obtained via a domestic direct-connect echo. A webpage can only see the exit IP, not the machine behind the proxy — that's a capability boundary, not something we skipped.",
    },
    C2: {
      title: "Local DNS Server & DNS Leak",
      meaning:
        "Which server handles your DNS queries — a webpage can't get query logs, so this is structurally untestable online. A DNS leak exposes the domains you've visited to your local ISP.",
    },
    C3: {
      title: "Proxy Detection (env vars / system proxy / TUN)",
      meaning:
        'Requires reading local environment variables and the system proxy configuration. The CLI can catch cases like "you think your proxy is on but it isn\'t actually taking effect."',
    },
    C4: {
      title: "Claude Code CLI Timezone Consistency",
      meaning:
        "Claude Code CLI honors the $TZ environment variable, which can differ from the system timezone. A webpage can't read $TZ — the O2 check above only tests the system timezone and doesn't cover this item.",
    },
    C5: {
      title: "Claude Endpoint Check",
      meaning:
        'Official direct / domestic LLM / relay, plus a known-endpoint blacklist match. Requires reading the local ANTHROPIC_BASE_URL configuration, which a webpage can\'t read. This is the only CLI item that can produce a "high risk" verdict on its own.',
    },
  },

  cli: {
    hint: "This item requires reading your local environment — install the CLI to test it:",
  },

  landing: {
    why: {
      title: "Why you need a checkup",
      body: "AI tools are sensitive to your network environment. The most common pitfalls fall into four categories: exit IP type or abuse history that triggers anti-abuse controls; a mismatch between system and exit IP timezone; IPv6 quietly bypassing the proxy and exposing your real location; and local DNS exposing the domains you visit to your local ISP. This site can check the first three online; a DNS leak requires reading local DNS query logs, which a webpage structurally cannot access — it's a CLI-only item.",
    },
    install: {
      title: "Install the CLI for all 9 checks",
      body: "The web version is a quick first look — it covers 4 items. The CLI covers all 9, including your real public IP, DNS leaks, proxy/TUN detection, and Claude Code timezone and endpoint checks.",
    },
    compare: {
      title: "Web vs. CLI: Full Feature Comparison",
      columnId: "ID",
      columnItem: "Check item",
      columnWeb: "Web",
      columnCli: "CLI",
      auto: "Automatic",
      onDemand: "On demand",
      cliOnly: "CLI only",
      dash: "—",
    },
  },

  footer: {
    privacy: "This site stores none of your check results.",
    thirdParty:
      "The IPv6 check is made directly from your browser to ipify; the IP risk check requires you to trigger it manually, at which point your exit IP is sent to proxycheck.io and StopForumSpam.",
  },

  errors: {
    network: "Network request failed — please retry shortly.",
    badRequest:
      "The request was invalid, so this item can't be completed. If this persists after a refresh, please let us know.",
    malformed:
      "The response was incomplete — this item is treated as a check failure.",
    rateLimited: "Too many requests — please retry shortly.",
    humanVerification: "Human verification failed — please retry.",
    upstream: "The data source is unavailable — this item couldn't be checked.",
    clientIp: "Couldn't determine your exit IP.",
    unknown: "Check failed — please retry shortly.",
  },
} as const;

/** 默认导出仍是中文（规格第 7 节：中文为默认），保持既有引用不受影响。 */
export const COPY: Copy = ZH;
export const COPY_EN: Copy = EN;

export function getCopy(lang: Lang): Copy {
  return lang === "en" ? EN : ZH;
}
