// 全站文案集中在此，不散落到 JSX 里——第 4 个任务要把它整体抽成翻译资源（规格第 7 节）。
// 纯字符串模块，不引任何 DOM 依赖，因此可被单测直接断言（部分文案是 ADR 的硬性要求，不得被改版误删）。

export const COPY = {
  site: {
    title: "ipcheck · 网络环境体检",
    tagline: "开跑 Claude 之前，先看清自己的网络长什么样",
  },

  verdict: {
    level: {
      low: "低风险",
      medium: "中风险",
      high: "高风险",
    },
    /** 初步结论必须带此标注（ADR-0004 / ADR-0005，验收标准 4）。 */
    preliminaryBadge: "初步 · 未含 IP 风险评分",
    fullBadge: "完整 · 已含 IP 风险评分",
    summary: {
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
    badge: "需 CLI",
    /** 灰卡是终态，不提供重试（规格 4.1）。 */
    hint: "本项需要读取本机环境，装 CLI 后可测：",
  },

  footer: {
    privacy: "本站不存储任何检测结果。",
    thirdParty:
      "IPv6 检测由浏览器直接访问 ipify；IP 风险检测需你手动触发，届时出口 IP 会被发送至 proxycheck.io 与 StopForumSpam。",
  },

  errors: {
    network: "网络请求失败，请稍后重试。",
    rateLimited: "请求过于频繁，请稍后重试。",
    humanVerification: "人机验证未通过，请重试。",
    upstream: "数据源不可用，本项未能完成检测。",
    clientIp: "未能确定你的出口 IP。",
    unknown: "检测失败，请稍后重试。",
  },
} as const;
