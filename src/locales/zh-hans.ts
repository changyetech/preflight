// 简体中文文案。结构由 `Copy`（源语言英文，见 en.ts）约束，漏译一个字段即编译错误。
//
// 这份是全站最早写成的文案，多条受 ADR 硬性约束（第三方披露、初步结论标注、零留存声明等），
// 改动前先读 tests/copy.test.ts 里对应的断言。

import type { Copy } from "./en";

export const ZH_HANS: Copy = {
  site: {
    title: "ipcheck · 网络环境体检",
    tagline: "开跑 Claude 之前，先看清自己的网络长什么样",
  },

  nav: {
    language: "语言",
    brand: "ipcheck",
    checks: "网页检测",
    cliChecks: "需 CLI",
    why: "为什么",
    install: "安装 CLI",
    compare: "对照表",
    backToTop: "回到顶部",
  },

  verdict: {
    level: {
      low: "低风险",
      medium: "中风险",
      high: "高风险",
    },
    insufficientLabel: "暂无结论",
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
      scopeNote:
        "本项对应 Claude 桌面版（浏览器时区＝系统时区）。Claude Code CLI 认 $TZ，见 C4。",
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
      thirdPartyNote:
        "本项由你的浏览器直接访问 ipify（api.ipify.org / api6.ipify.org）完成，你的出口地址对 ipify 可见；本站不经手、不存储。",
      retryLabel: "重试（将再次由浏览器直连 ipify）",
    },
    O4: {
      title: "IP 类型与风险",
      idle: "按需检测项，需要你手动触发。",
      consentButton:
        "检测 IP 风险（将把你的出口 IP 发送至 proxycheck.io 查询）",
      consentNote:
        "点击后会先加载 Cloudflare Turnstile 人机验证（challenges.cloudflare.com），验证时你的出口 IP 会一并提交给 Cloudflare。随后同时会向 StopForumSpam 查询该 IP 是否有滥用收录。本站不存储任何查询结果。",
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

  sections: {
    online: {
      title: "网页可测到的（4 项）",
      body: "浏览器与 Cloudflare 边缘就能完成，无需在本机装任何东西。",
    },
    cli: {
      title: "需要 CLI 才能测（5 项）",
      body: "这 5 项必须读取本机环境（本地 DNS、系统代理与 TUN、$TZ、Claude Code 配置），网页结构性拿不到，只能由 CLI 在你本机完成。",
    },
  },

  cli: {
    hint: "本项需要读取本机环境，装 CLI 后可测。",
  },

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
      available: "支持",
      dash: "—",
    },
  },

  footer: {
    privacy: "本站不存储任何检测结果。",
    thirdParty:
      "IPv6 检测由浏览器直接访问 ipify；IP 风险检测需你手动触发，届时会先加载 Cloudflare Turnstile 人机验证，随后出口 IP 会被发送至 proxycheck.io 与 StopForumSpam。",
  },

  errors: {
    network: "网络请求失败，请稍后重试。",
    badRequest: "请求不合法，本项无法完成。若刷新后仍然如此，请反馈给我们。",
    malformed: "接口返回的数据不完整，本项按检测失败处理。",
    rateLimited: "请求过于频繁，请稍后重试。",
    humanVerification: "人机验证未通过，请重试。",
    upstream: "数据源不可用，本项未能完成检测。",
    clientIp: "未能确定你的出口 IP。",
    unknown: "检测失败，请稍后重试。",
  },
};
