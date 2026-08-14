// 英文文案 —— 全站的**源语言**（规格第 7 节）。
// 其余语种按这里的结构对齐：漏译、多译一个字段都是编译错误，而不是运行时才发现某处掉回英文。
//
// 纯字符串模块，不引任何 DOM 依赖，因此可被单测直接断言（部分文案是 ADR 的硬性要求，不得被改版误删）。

export const EN = {
  site: {
    title: "Preflight · Network Environment Checkup",
    tagline: "Know your network before you use IP-sensitive tools",
  },

  nav: {
    /** 语言切换器无障碍名称的前缀，后接目标语种自称（在 LOCALES 表里，不在文案里翻译），
     *  拼成「Switch language to 简体中文」（规格 §2 决策 8：单键直切）。 */
    switchLanguageTo: "Switch language to",
    /** 跳过导航链接（规格 §4 要点 6）：键盘用户可跳过顶栏 5 个锚点直达正文。 */
    skipToContent: "Skip to main content",
    /** 顶栏品牌名与锚点标签：短于 landing 各段标题，顶栏放不下长标题。 */
    brand: "Preflight",
    checks: "Web checks",
    cliChecks: "CLI-only",
    why: "Why",
    install: "Install CLI",
    compare: "Comparison",
    /** 跨页导航项（其余都是首页锚点），因此顶栏与锚点组分开排布。
     *  短标签：顶栏容不下「Public DNS list」那样的完整标题。 */
    dns: "DNS list",
    /** 第二个跨页导航项：CLI 使用手册（spec docs/specs/2026-08-14-cli-guide-page.md）。 */
    guide: "CLI guide",
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
        "Your exit IP is high risk. IP-sensitive services are quite likely to flag you right now.",
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
    /**
     * 两条安装命令与 README「安装 CLI」一致——用户会直接复制执行，改这里要同步 README。
     * 没有 Homebrew 那条：那需要独立的 tap 仓库，已按 docs/deployment.md 第 5 节摘掉。
     * 命令本身是命令，不翻译；两个语种共用同一份字面量。
     */
    installCommand:
      "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/changyetech/preflight/releases/latest/download/preflight-installer.sh | sh",
    /** Windows 是一等支持（dist 有 x86_64-pc-windows-msvc，探测侧有 cfg(windows) 分支）。 */
    installCommandWindows:
      'powershell -c "irm https://github.com/changyetech/preflight/releases/latest/download/preflight-installer.ps1 | iex"',
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
      /**
       * 契约 §6 的呈现义务：`anonymous: true` 且分数 51–75 时会出现「结论高 · 分项黄」，
       * 缺了这句解释，用户看到高风险结论却找不到哪一项显红，会以为结论算错了。
       * 与 CLI 的 `values.anonymous_flag` 说同一件事，措辞对齐。
       */
      anonymousNote:
        "This IP is currently used as an anonymising address — the high-risk threshold drops to 51 for it.",
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
      lede: "Tools and services that judge you by your IP are sensitive to your network environment. The most common pitfalls fall into four categories.",
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
      /** 安装命令的适用平台（原型 .install-meta .plat），与 Cargo.toml 的 dist targets 一致。 */
      platforms: "macOS · Linux · Windows",
      /** 安装区块通往 /guide/ 的入口（spec docs/specs/2026-08-14-cli-guide-page.md 决策 3）。 */
      guideLink: "Read the full CLI guide →",
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

  dns: {
    title: "Public DNS Servers · Preflight",
    description:
      "A curated list of public DNS resolvers — Cloudflare, Google, Quad9, AliDNS and more — with their region and filtering level.",
    heading: "Public DNS Servers",
    tableHeaders: {
      ip: "IP",
      provider: "Provider",
      region: "Region",
      filter: "Filter",
    },
    variants: {
      standard: "Standard",
      security: "Security",
      family: "Family",
      adblock: "Ad-block",
    },
    cliHint:
      "Want to test which servers actually work from your machine? Run preflight dns --check in the CLI.",
  },

  /**
   * CLI 使用手册页（/guide/，spec docs/specs/2026-08-14-cli-guide-page.md）。
   * 底稿是 docs/cli-guide.md——只谈「怎么用」，判级规则不复述（契约红线）。
   * `sections` 每项为统一形状（paras / code / table / after，空数组表示该块缺席）：
   * 统一形状让 en/zh 的结构约束经得起 `Widen` 类型的展开。
   */
  guide: {
    title: "CLI Guide · Preflight",
    description:
      "How to use the Preflight CLI: install, commands and options, exit codes, configuration, JSON output, and common scenarios.",
    heading: "CLI Guide",
    lede: "Preflight CLI performs a complete network environment checkup in your terminal — all 10 checks, concluding with an overall verdict and its coverage. It stores no results, and its probes contact third-party services directly, never through this site's servers. This page covers usage only; the meaning of each check is explained on the home page.",
    install: {
      heading: "Installation",
      intro:
        "One command per platform — the installer script downloads the matching binary from GitHub Releases:",
      linuxLabel: "Linux / macOS",
      windowsLabel: "Windows (PowerShell)",
      verify: "After installation, verify:",
    },
    sections: [
      {
        heading: "Quick start",
        paras: [
          "Running preflight with no arguments starts the checkup — no subcommand is required:",
        ],
        code: ["preflight"],
        table: { headers: [], rows: [] },
        after: [
          "All probes run concurrently; once they complete, the report is printed in a single pass: the overall verdict (low / medium / high risk, or “insufficient data”) with its coverage (n/10), followed by the detailed result of each check.",
          "Progress indicators are written to stderr, and only on an interactive terminal — stdout always carries a clean report, so output redirected to a file never contains progress lines.",
        ],
      },
      {
        heading: "Commands and options",
        paras: ["There are four commands; the checkup is the default:"],
        code: [
          "preflight [OPTIONS]            # checkup (default command)",
          "preflight dns [--check]        # public DNS server list",
          "preflight config <ACTION>      # view and change configuration",
          "preflight uninstall [--purge]  # uninstall (--purge also removes the config)",
        ],
        table: {
          headers: ["Option", "Notes"],
          rows: [
            [
              "--lang <LANG>",
              "Interface language: en / zh-hans. A global option, accepted in any position.",
            ],
            [
              "--json",
              "Machine-readable JSON output. Must precede the subcommand.",
            ],
            [
              "-v, --verbose",
              "Include each check's explanation in the report.",
            ],
            [
              "-V / --version",
              "Print the short version number / full version information.",
            ],
          ],
        },
        after: [
          "When no language is specified explicitly, it is resolved in the following order: the --lang option, the language configuration key, the system locale, then English.",
        ],
      },
      {
        heading: "The checkup and its exit codes",
        paras: ["Exit codes are the contract for script integration:"],
        code: [],
        table: {
          headers: ["Exit code", "Meaning"],
          rows: [
            [
              "0",
              "The checkup completed, regardless of risk level. Read the level from the report or the JSON output, not from the exit code — encoding risk in the exit code would leave scripts unable to distinguish “high risk” from “tool failure”.",
            ],
            [
              "1",
              "The tool itself failed (invalid configuration, unsupported language, etc.).",
            ],
            [
              "2",
              "The checkup ran but produced no contributing signal; the verdict is “insufficient data”. The report is still printed.",
            ],
          ],
        },
        after: [
          "Colored output is disabled automatically when stdout is not a terminal (pipes, redirects); the NO_COLOR environment variable or config set no-color true also disables it. Report width is measured from the terminal window once before rendering; redirected output uses a fixed width.",
        ],
      },
      {
        heading: "preflight dns",
        paras: [
          "Lists the built-in public DNS servers (IP / provider / region / filtering level — the same dataset as this site's DNS list page). With --check, a real DNS query is sent to each server to measure reachability and latency:",
        ],
        code: [
          "preflight dns",
          "preflight dns --check",
          "preflight --json dns --check",
        ],
        table: {
          headers: ["Status", "Criteria"],
          rows: [
            [
              "ok",
              "A response was received with a matching TXID, an RCODE of NOERROR, and at least one non-private A record.",
            ],
            [
              "suspicious",
              "A response was received but fails one or more of the above criteria — possible hijacking or pollution.",
            ],
            [
              "unreachable",
              "No valid response was received before the timeout.",
            ],
          ],
        },
        after: [],
      },
      {
        heading: "preflight config",
        paras: [
          "config path prints the configuration file location; config list and config get print effective values (after merging all sources); config set and config unset write to the file:",
        ],
        code: [
          "preflight config set language zh-hans",
          "preflight config set timeout 20",
          "preflight config set proxycheck-key   # prompts interactively, never echoed",
          "preflight config unset timeout",
        ],
        table: {
          headers: ["Key", "Values", "Default", "Notes"],
          rows: [
            ["language", "en / zh-hans", "system locale", "Interface language"],
            [
              "proxycheck-key",
              "interactive prompt",
              "unset",
              "proxycheck.io API key — see below",
            ],
            ["timeout", "1–120 (seconds)", "10", "Network probe timeout"],
            ["no-color", "true / false", "false", "Disable colored output"],
          ],
        },
        after: [
          "The configurable keys form a whitelist — verdict thresholds and check toggles are not configurable.",
          "The API key deliberately has no plaintext option: it would be written to shell history and appear in ps process listings.",
          "If a key you have just set is overridden by a higher-priority source (--lang, PROXYCHECK_API_KEY, NO_COLOR), the command reports this on stderr.",
        ],
      },
      {
        heading: "Configuration sources and precedence",
        paras: [
          "Precedence is fixed: command-line option > environment variable > configuration file > built-in default.",
          "The configuration file is TOML with underscore-separated keys (proxycheck_key, no_color). Unknown keys are an error rather than being silently ignored — a misspelled key never appears to be “configured but ineffective”. On Unix the file is written with 600 permissions.",
        ],
        code: [],
        table: {
          headers: ["Platform", "Config file path"],
          rows: [
            [
              "Linux / macOS",
              "$XDG_CONFIG_HOME/preflight/config.toml, or ~/.config/preflight/config.toml",
            ],
            ["Windows", "%APPDATA%\\preflight\\config.toml"],
            [
              "Any",
              "The PREFLIGHT_CONFIG environment variable points anywhere and wins over both.",
            ],
          ],
        },
        after: [],
      },
      {
        heading: "Environment variables",
        paras: [],
        code: [],
        table: {
          headers: ["Variable", "Effect"],
          rows: [
            [
              "PROXYCHECK_API_KEY",
              "proxycheck key; takes precedence over the configuration file. An empty value is treated as unset.",
            ],
            [
              "NO_COLOR",
              "Disables colored output whenever present and non-empty (per the no-color.org convention; the value itself is not inspected).",
            ],
            ["PREFLIGHT_CONFIG", "Overrides the config file path."],
          ],
        },
        after: [],
      },
      {
        heading: "proxycheck API key",
        paras: [
          "The IP type & risk check and ownership lookups are performed via proxycheck.io. Without a key, the CLI uses the anonymous allowance (100 queries per day); a free key raises this to 1,000:",
        ],
        code: [
          "preflight config set proxycheck-key    # or: export PROXYCHECK_API_KEY=…",
          "preflight config get proxycheck-key    # only reports set / unset",
        ],
        table: { headers: [], rows: [] },
        after: [
          "The key never appears in any output — the report, --json, or error messages. The CLI contacts proxycheck directly using your own quota; requests never pass through this site and do not consume the web version's shared quota.",
        ],
      },
      {
        heading: "--json output",
        paras: [
          "preflight --json prints a single JSON object: verdict (stage and level), coverage (done / failed / total), signals (tri-state: true / false / null — null means unknown, which is distinct from false), and one entry per check under checks (completed entries carry their fields; failed entries carry a reason of upstream, quotaExhausted, or local).",
        ],
        code: [
          "{",
          '  "verdict":  { "stage": "final", "level": "low" },',
          '  "coverage": { "done": 10, "failed": 0, "total": 10 },',
          '  "signals":  { "ipv6Leak": false, "dnsEgressLeak": false, … },',
          '  "checks":   { "O1": { "status": "done", … }, … }',
          "}",
        ],
        table: { headers: [], rows: [] },
        after: [
          'preflight --json dns uses a separate schema: { "servers": [ … ] }, each entry carrying ip / name / region / variant, with an additional check block when --check was run.',
        ],
      },
    ],
    scenarios: {
      heading: "Typical scenarios",
      items: [
        {
          title: "After switching proxy nodes",
          body: "Run a checkup before performing IP-sensitive operations — confirm that DNS, UDP, and IPv6 traffic is not bypassing the proxy, and that the exit IP's risk score is within a normal range.",
          code: ["preflight"],
        },
        {
          title: "Scripts and automation",
          body: "Combine --json with the exit codes. The risk level is read from the JSON output; the exit code only indicates whether the tool ran to completion.",
          code: [
            "out=$(preflight --json) || exit 1",
            "level=$(echo \"$out\" | jq -r '.verdict.level')",
            '[ "$level" = "high" ] && echo "high-risk exit, aborting" >&2 && exit 1',
          ],
        },
        {
          title: "Proxy enabled, yet still flagged",
          body: "Review O5 (DNS egress leak), O6 (UDP egress consistency), O3 (IPv6 leak), and C3 (TUN off) in the report; add -v for each check's explanation.",
          code: ["preflight -v"],
        },
        {
          title: "Choosing a usable public DNS",
          body: "Measure the reachability and latency of every server in the list from your machine, identifying suspicious responses along the way.",
          code: ["preflight dns --check"],
        },
        {
          title: "Probes time out on a slow network",
          body: "Increase the probe timeout (1–120 seconds).",
          code: ["preflight config set timeout 30"],
        },
      ],
    },
  },

  /**
   * 隐私说明与使用条款（spec docs/specs/2026-08-14-legal-pages.md）。
   * 这里的每一句都是对现网行为的陈述——第三方清单、p=0/tag=0、localStorage 只存主题——
   * 行为改了，这段文案必须同步改，否则页面就成了与事实不符的声明。
   */
  legal: {
    privacyLink: "Privacy",
    termsLink: "Terms",
    updated: "Last updated: 2026-08-14",
    privacy: {
      title: "Privacy · Preflight",
      heading: "Privacy",
      lede: "Preflight has no accounts, no database, and no analytics. Nothing you check here is stored.",
      sections: [
        {
          heading: "What we store",
          body: "Nothing. Every check is either computed in your browser or passed straight through to a third party and returned to you. No check result, no IP address, and no report is written to any store we keep.",
        },
        {
          heading: "Third parties your browser contacts",
          body: "Three checks run automatically as soon as the page loads, directly from your browser: the IPv6 check queries ipify, the DNS egress check queries ip-api.com, and the UDP egress check queries stun.cloudflare.com and stun.l.google.com. The IP risk check only runs when you trigger it: Cloudflare Turnstile loads first for bot verification, then your exit IP is sent to proxycheck.io and StopForumSpam. Each of these services sees your exit IP and handles it under its own privacy policy.",
        },
        {
          heading: "Cookies and local storage",
          body: "This site sets no cookies and runs no analytics. The only thing kept on your device is your theme preference (light / dark / system), stored under the key preflight-theme in localStorage; it never leaves your browser. Your language is decided by the URL path and is not remembered anywhere.",
        },
        {
          heading: "Risk lookups are untagged",
          body: "Requests to proxycheck.io carry p=0 and tag=0, so the lookup is not tagged and does not show up as a labelled query in their dashboard.",
        },
        {
          heading: "The command-line tool",
          body: "The Preflight CLI talks to third-party services directly and never routes your checks through this site. It writes nothing but your own configuration file, on your own machine.",
        },
        {
          heading: "Hosting",
          body: "The site runs on Cloudflare Workers. Cloudflare handles the request at its edge as it does for any site on its network.",
        },
      ],
    },
    terms: {
      title: "Terms · Preflight",
      heading: "Terms",
      lede: "Short version: this is a free diagnostic tool, offered as is. Use it on networks you are responsible for.",
      sections: [
        {
          heading: "Provided as is",
          body: "Preflight is provided free of charge, as is, without warranty of any kind. Checks may fail, return stale data, or be unavailable without notice.",
        },
        {
          heading: "Signals, not guarantees",
          body: "Every verdict describes signals observed at one moment from one vantage point. A clean result is not a guarantee of anonymity, security, or compliance, and a flagged result is not proof of wrongdoing. Do not rely on this tool as your only basis for a decision that matters.",
        },
        {
          heading: "Acceptable use",
          body: "Use Preflight to check networks you own or are authorised to test. Do not automate it against the endpoints at scale, and do not use it as part of an attack or to evade abuse controls. Rate limits and a daily quota apply and may be enforced without notice.",
        },
        {
          heading: "Third-party services",
          body: "Checks rely on ipify, ip-api.com, Cloudflare STUN and Turnstile, proxycheck.io, and StopForumSpam. Their data and terms are their own; we make no representation about their accuracy or availability.",
        },
        {
          heading: "Availability",
          body: "There is no uptime commitment. Endpoints, checks, and this site may change or be withdrawn at any time.",
        },
        {
          heading: "Liability",
          body: "To the maximum extent permitted by law, we are not liable for any loss or damage arising from the use of, or reliance on, this site or its results.",
        },
      ],
    },
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
