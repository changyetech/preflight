// 英文文案 —— 全站的**源语言**（规格第 7 节）。
// 其余语种按这里的结构对齐：漏译、多译一个字段都是编译错误，而不是运行时才发现某处掉回英文。
//
// 纯字符串模块，不引任何 DOM 依赖，因此可被单测直接断言（部分文案是 ADR 的硬性要求，不得被改版误删）。

export const EN = {
  site: {
    title: "ipcheck · Network Environment Checkup",
    tagline: "Know your network before you launch your AI CLI",
  },

  nav: {
    /** 语言切换器无障碍名称的前缀，后接目标语种自称（在 LOCALES 表里，不在文案里翻译），
     *  拼成「Switch language to 简体中文」（规格 §2 决策 8：单键直切）。 */
    switchLanguageTo: "Switch language to",
    /** 跳过导航链接（规格 §4 要点 6）：键盘用户可跳过顶栏 5 个锚点直达正文。 */
    skipToContent: "Skip to main content",
    /** 顶栏品牌名与锚点标签：短于 landing 各段标题，顶栏放不下长标题。 */
    brand: "ipcheck",
    checks: "Web checks",
    cliChecks: "CLI-only",
    why: "Why",
    install: "Install CLI",
    compare: "Comparison",
    /** 回顶按钮只有图标，标签给读屏用。 */
    backToTop: "Back to top",
    /** 主题切换器：三态下拉（浅色/深色/跟随系统），规格 §2 决策 1。 */
    theme: {
      label: "Theme",
      light: "Light",
      dark: "Dark",
      system: "Follow system",
    },
  },

  verdict: {
    level: {
      low: "Low risk",
      medium: "Medium risk",
      high: "High risk",
    },
    /** 数据不足时的档位文案。绝不能落到「低风险」——没测成不是安全。 */
    insufficientLabel: "No verdict yet",
    /** 初步结论必须带此标注（ADR-0004 / ADR-0005，验收标准 4）。 */
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
    /** 「真实 IP」一词禁用（CONTEXT.md）。 */
    exitIpNote:
      "This is the public address your traffic leaves the proxy with, not the machine address behind the proxy.",
    /** 控制台右栏 eyebrow（原型「综合结论」），非「分档」本身的文案。 */
    summaryLabel: "Overall verdict",
    /** 控制台顶部运行指示（原型 .live）：是否还有在线项在检测中。 */
    live: {
      checking: "Checking",
      ready: "Ready",
    },
  },

  coverage: {
    /** 控制台右栏覆盖度小节的 eyebrow。 */
    label: "Coverage",
    done: "Done",
    needCli: "Needs CLI",
    failed: "Check failed",
    pending: "Not run yet",
    total: "10 items total",
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
    /** 与 README「安装 CLI」的首选方式一致。`<owner>` 在第一个 `cli/v*` tag 前是占位。 */
    installCommand: "brew install <owner>/tap/ipcheck",
    meaningLabel: "What this means",
  },

  checks: {
    O1: {
      title: "Exit IP and Ownership",
      meaning:
        "This is what every service you connect to sees as you. The further this location is from where you actually are, the more likely your traffic gets flagged as anomalous.",
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
      /**
       * 必须显式区分两个时区来源（规格 2.2 / 验收标准 2）。
       * CLI 那一侧的完整说明归 C4 卡，这里只留最小限定 + 指路，避免两张卡讲同一段话。
       */
      scopeNote:
        "Covers GUI apps (browser = system timezone). Command-line tools read $TZ — see C4.",
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
      /**
       * O3 自动执行、无控件可挂披露，只能就地放在卡片说明位（终审修复波：ipify 无就地披露）。
       * O4 遵守了「重试按钮写明第三方」这条规则（consentButton 兼作 retryLabel），O3 之前没有。
       */
      thirdPartyNote:
        "This check runs directly from your browser against ipify (api.ipify.org / api6.ipify.org), so your exit address is visible to ipify. It never passes through, and is never stored by, this site.",
      retryLabel: "Retry (queries ipify from your browser again)",
    },
    O4: {
      title: "IP Type & Risk",
      idle: "On-demand check — you need to trigger it manually.",
      /** ADR-0008：第三方调用必须写在触发它的控件上。文案改动前先读 ADR-0008。 */
      consentButton: "Check IP risk (sends your exit IP to proxycheck.io)",
      /** 终审修复波：Turnstile 会加载 challenges.cloudflare.com 且把出口 IP 提交给 Cloudflare。 */
      consentNote:
        "Clicking first loads Cloudflare Turnstile (challenges.cloudflare.com) for bot verification; your exit IP is submitted to Cloudflare as part of that check. It then also queries StopForumSpam for abuse records on that IP. This site stores none of the query results.",
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
        /** 第三方不可用时必须显示「未知」，不得冒充「无收录」（docs/api.md 3.1）。 */
        unknown: "Unknown (data source unavailable)",
      },
      quotaExhausted:
        'Today\'s quota is exhausted. This item counts as "Check failed" toward coverage and resets after UTC midnight; other items and the preliminary verdict are unaffected.',
      turnstileMissing:
        "The human-verification widget isn't configured — this item is unavailable for now.",
    },
    O5: {
      title: "DNS Egress Leak",
      meaning:
        "If your DNS queries leave from a different country than your proxied traffic, whoever operates that resolver — and anyone watching that path — can see which sites you're resolving, even while your HTTP/TCP traffic looks proxied. This is one of the two most common split-tunnel leaks (the other is UDP, see O6).",
      /** 契约 §5.5 呈现约束：缺了这句，用户会拿浏览器 DoH 的绿色结论去为命令行工具背书。 */
      scopeNote:
        "This checks the DNS path your browser actually uses. If your browser has Secure DNS (DoH) enabled, its result can differ from your command-line tools, which use the system resolver instead — that's covered by the CLI. If the two disagree, trust the CLI.",
      resolverLabel: "Resolver location",
      /** 契约 §2.1／§2.5 硬约束 1：resolver 归属只展示，不参与判定。 */
      resolverNote:
        "Shown for reference only — it doesn't affect the verdict. Which country your resolver sits in depends on which DNS provider you picked, not on whether your traffic is proxied.",
      ecsLabel: "DNS client subnet country",
      exitLabel: "Exit IP country",
      leak: "Your DNS queries appear to leave from a different country than your exit IP — DNS may be bypassing the proxy.",
      noLeak:
        "Your DNS queries appear to leave from the same country as your exit IP — no DNS egress leak detected.",
      /** 三种「无从比对」各一条独立说明，都不得回退成「泄露」或「未泄露」（契约 §2.5 硬约束 3）。 */
      noEcs:
        "Your DNS provider doesn't send EDNS Client Subnet data, so this can't determine whether your DNS queries are proxied.",
      unmappedCountry:
        "Your resolver reported a client-subnet location we don't recognize, so this can't be compared.",
      unknownExitCountry:
        "The exit IP's country isn't available yet (see the Exit IP check above), so this can't be compared.",
      failed:
        "The DNS egress probe didn't complete — this item can't be determined. Retry once your network is back.",
      thirdPartyNote:
        "This check runs directly from your browser against ip-api.com (a freshly randomized subdomain each time), so your exit address is visible to it. It never passes through, and is never stored by, this site.",
      retryLabel: "Retry (queries ip-api.com from your browser again)",
    },
    O6: {
      title: "UDP Egress Consistency",
      meaning:
        "Most proxies only reliably capture TCP. If UDP traffic slips out through a different path, it can expose a different address than the one your other traffic shows — including to services that use UDP directly (WebRTC, some game and VoIP clients, and select AI tooling).",
      reflexiveLabel: "UDP reflexive address",
      exitLabel: "Exit IP",
      mismatch:
        "Your UDP traffic appears to exit from a different address than your exit IP — UDP may be bypassing the proxy.",
      noMismatch:
        "Your UDP traffic appears to exit from the same address as your exit IP — no UDP egress mismatch detected.",
      /** 三种「无从比对」各一条，且必须与命中／未命中在措辞上可区分（契约 §2.6 判定表第 2/3/4 行）。 */
      familyMismatch:
        "Fewer than two reflexive addresses in the same address family as your exit IP (IPv4/IPv6) came back, so there's nothing to compare on equal terms.",
      unknownExitIp:
        "The exit IP isn't available yet (see the Exit IP check above), so this can't be compared.",
      stunDisagree:
        "The two STUN servers reported different addresses, so there's no single reliable value to compare — this can happen with multi-exit clusters or symmetric NAT.",
      /** 契约 §5.6 呈现约束：必须区分「浏览器不允许」与「探测超时」，且前者绝不渲染成绿色。 */
      webrtcUnavailable:
        "Your browser has WebRTC disabled, so this item can't determine whether UDP is proxied. The CLI isn't affected by this — it uses a raw UDP socket, not your browser's WebRTC stack.",
      stunUnanswered:
        "Neither STUN server answered before the timeout — likely a transient network issue. Retry.",
      thirdPartyNote:
        "This check uses your browser's WebRTC to exchange STUN requests with stun.cloudflare.com and stun.l.google.com, so your UDP reflexive address is visible to them. It never passes through, and is never stored by, this site.",
      retryLabel:
        "Retry (probes stun.cloudflare.com and stun.l.google.com again)",
    },
    C1: {
      title: "Real Public IP",
      meaning:
        "Your real public IP, obtained via a direct-connect echo service in mainland China — rule-based proxies route it directly. A webpage can only see the exit IP, not the machine behind the proxy — that's a capability boundary, not something we skipped.",
    },
    C2: {
      /** 契约 §1 收缩：DNS 泄露判定拆到 O5，本项只剩「本地 DNS 服务器配置」（ADR-0014）。 */
      title: "Local DNS Server Configuration",
      meaning:
        "Which server handles your DNS queries, and how it's configured — a webpage structurally can't read this. Whether those queries are actually leaving through the proxy is a separate check, see O5.",
    },
    C3: {
      title: "Proxy Detection (env vars / system proxy / TUN)",
      meaning:
        'Requires reading local environment variables and the system proxy configuration. The CLI can catch cases like "you think your proxy is on but it isn\'t actually taking effect."',
    },
    C4: {
      title: "$TZ Timezone Consistency",
      meaning:
        "Command-line tools inherit the $TZ environment variable, which can differ from the system timezone. A webpage can't read $TZ — the O2 check above only tests the system timezone and doesn't cover this item.",
    },
  },

  /** 检测卡区的两个分区标题（规格第 4 节第 2 项）。 */
  sections: {
    online: {
      title: "What this page can check (6)",
      body: "Your browser and the Cloudflare edge cover these — nothing to install locally.",
    },
    cli: {
      title: "What only the CLI can check (4)",
      body: "These four require reading your local environment (local DNS, system proxy and TUN, $TZ). A webpage structurally cannot reach them — only the CLI, running on your machine, can.",
    },
  },

  /** 落地内容三段（规格第 4 节第 3 项）。 */
  landing: {
    why: {
      title: "Why you need a checkup",
      /** 四栏枚举前的引导句（原型 .why-lede，新增结构性文案）。 */
      lede: "AI tools are sensitive to your network environment. The most common pitfalls fall into four categories.",
      /** 四栏枚举，顺序与 O4/O2/O3/C2 对应（原型 .why-grid）。措辞取自对应检测项的 meaning 文案，不新造事实。 */
      items: [
        {
          title: "Exit IP type & abuse history",
          body: "Datacenter IPs, public proxies, and heavily abused IPs are the most direct trigger for anti-abuse controls.",
        },
        {
          title: "System vs. exit IP timezone",
          body: "A timezone mismatch is one of the most common proxy tells: the IP says United States, but the system clock says Beijing time — anti-abuse controls catch that instantly.",
        },
        {
          title: "IPv6 leak",
          body: "Most proxies only handle IPv4. If your machine has IPv6, some traffic slips past the proxy and goes out directly, exposing an address from a different location.",
        },
        {
          title: "Local DNS exposure",
          body: "Your local ISP can see which domains you resolve, even once your other traffic is proxied.",
        },
      ],
      /** 前三项标签后缀（原型「网页可测」）；第四项（CLI-only）复用 coverage.needCli，不新造近义词。 */
      checkedOnlineTag: "Checked online",
      /** 段尾收束句，逐字取自原 why.body 的后半句。 */
      foot: "This site can check the first three online; your local DNS server's configuration requires reading your local environment, which a webpage structurally cannot access — that's a CLI-only item.",
    },
    install: {
      title: "Install the CLI for all 10 checks",
      body: "The web version is a quick first look — it covers 6 items. The CLI covers all 10, including your real public IP, local DNS configuration, proxy/TUN detection, and the $TZ timezone check.",
      /** 安装命令的适用平台（原型 .install-meta .plat）。brew 安装仅覆盖 macOS/Linux，与 Cargo.toml 的 dist targets 一致。 */
      platforms: "macOS · Linux",
    },
    compare: {
      title: "Web vs. CLI: Full Feature Comparison",
      columnId: "ID",
      columnItem: "Check item",
      columnWeb: "Web",
      columnCli: "CLI",
      auto: "Automatic",
      onDemand: "On demand",
      /**
       * CLI 覆盖全部 10 项（CLI README 功能表），这一列没有「—」的取值（C1 修复）。
       * 用带文字的「支持」而不是裸符号「✓」——屏幕阅读器对孤立符号的朗读不稳定（N3）。
       */
      available: "Yes",
      dash: "—",
    },
  },

  /** 页脚双栏披露（原型 .footer-cols）：按「自动发起／需手动触发」拆开，原文一字未改，只是分段。 */
  footer: {
    privacy: "This site stores none of your check results.",
    autoLabel: "Runs automatically",
    autoBody:
      "Three checks run automatically as soon as the page loads, directly from your browser: the IPv6 check queries ipify, the DNS egress check queries ip-api.com, and the UDP egress check queries stun.cloudflare.com and stun.l.google.com.",
    onDemandLabel: "Triggered manually",
    onDemandBody:
      "The IP risk check requires you to trigger it manually, at which point Cloudflare Turnstile loads for bot verification first, then your exit IP is sent to proxycheck.io and StopForumSpam.",
  },

  errors: {
    network: "Network request failed — please retry shortly.",
    /** 1001 / 4001：前端请求本身不合法，重试不会好转，措辞里不给「稍后重试」的暗示。 */
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

/**
 * 把 EN 的字面量类型（`"Exit IP"` 这种具体字符串）拓宽成 `string`。
 * 各语种按 `Copy`（而非 `typeof EN`）类型检查，这样译文不必凑巧等于某个英文字面量，
 * 同时保留结构约束：漏译、多译一个字段照样是编译错误。
 */
type Widen<T> = T extends string
  ? string
  : T extends readonly (infer U)[]
    ? readonly Widen<U>[]
    : { [K in keyof T]: Widen<T[K]> };

export type Copy = Widen<typeof EN>;
