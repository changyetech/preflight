// 简体中文文案。结构由 `Copy`（源语言英文，见 en.ts）约束，漏译一个字段即编译错误。
//
// 这份是全站最早写成的文案，多条受 ADR 硬性约束（第三方披露、初步结论标注、零留存声明等），
// 改动前先读 tests/copy.test.ts 里对应的断言。

import type { Copy } from "./en";

export const ZH_HANS: Copy = {
  site: {
    title: "Preflight · 网络环境体检",
    tagline: "使用对 IP 环境敏感的工具之前，先看清自己的网络长什么样",
  },

  nav: {
    switchLanguageTo: "切换语言到",
    skipToContent: "跳到主要内容",
    brand: "Preflight",
    checks: "网页检测",
    cliChecks: "需 CLI",
    why: "为什么",
    install: "安装 CLI",
    compare: "对照表",
    dns: "DNS 清单",
    guide: "CLI 手册",
    backToTop: "回到顶部",
    theme: {
      label: "主题",
      light: "浅色",
      dark: "深色",
      system: "跟随系统",
    },
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
      fullHigh:
        "出口 IP 风险很高，此时使用对 IP 敏感的服务有较大概率触发风控。",
    },
    exitIpLabel: "出口 IP",
    exitIpUnknown: "未能取得",
    exitIpNote: "这是你流量离开代理后的公网地址，不是代理背后的本机地址。",
    summaryLabel: "综合结论",
    live: {
      checking: "检测中",
      ready: "已就绪",
    },
  },

  coverage: {
    label: "覆盖度",
    done: "已完成",
    needCli: "需 CLI",
    failed: "检测失败",
    pending: "按需未测",
    total: "共 10 项",
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
    installCommand:
      "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/changyetech/preflight/releases/latest/download/preflight-installer.sh | sh",
    installCommandWindows:
      'powershell -c "irm https://github.com/changyetech/preflight/releases/latest/download/preflight-installer.ps1 | iex"',
    meaningLabel: "这意味着什么",
  },

  checks: {
    O1: {
      title: "出口 IP 与归属",
      meaning:
        "这是你连上的所有服务看到的你。归属地与你实际所在地差得越远，越容易被判为异常流量。",
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
        "本项对应图形界面应用（浏览器时区＝系统时区）。命令行工具认 $TZ，见 C4。",
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
      anonymousNote: "该 IP 正被用作匿名化地址 —— 判高的阈值对它降到 51。",
      abuse: {
        listed: "有收录",
        clean: "无收录",
        unknown: "未知（数据源不可用）",
      },
      quotaExhausted:
        "今日额度已用尽。本项按「检测失败」计入覆盖度，明日 UTC 零点后恢复；其余各项与初步结论不受影响。",
      turnstileMissing: "人机验证组件未配置，本项暂不可用。",
    },
    O5: {
      title: "DNS 出口泄露",
      meaning:
        "如果你的 DNS 查询从跟代理流量不同的国家出网，运营那台 resolver 的人、以及监听这条链路的人，都能看到你在解析哪些域名——即使你的 HTTP/TCP 流量看起来已经走了代理。这是分流泄露最常见的两条漏之一（另一条是 UDP，见 O6）。",
      scopeNote:
        "本项测的是你的浏览器实际使用的 DNS 路径。如果浏览器开着 Secure DNS（DoH），结果会与命令行工具不同——后者走的是系统 resolver，属于 CLI 的判定范围。两者不一致时，以 CLI 的结果为准。",
      resolverLabel: "resolver 归属",
      resolverNote:
        "仅供参考，不参与判定：resolver 在哪个国家取决于你选了哪家 DNS 服务商，与流量是否走代理无关。",
      ecsLabel: "DNS 客户端子网归属国",
      exitLabel: "出口 IP 归属国",
      leak: "你的 DNS 查询似乎从与出口 IP 不同的国家出网，DNS 可能正在绕过代理。",
      noLeak:
        "你的 DNS 查询似乎与出口 IP 从同一国家出网，未检测到 DNS 出口泄露。",
      noEcs: "你的 DNS 服务商不发送 ECS，无法判定 DNS 查询是否走代理。",
      unmappedCountry:
        "resolver 返回了客户端子网归属地，但我们暂时认不出这个国家名，无法比对。",
      unknownExitCountry:
        "出口 IP 的归属国尚未取得（见上方「出口 IP 与归属」），暂时无法比对。",
      failed: "DNS 出口探测未能完成，本项无法判定。网络恢复后可重试。",
      thirdPartyNote:
        "本项由你的浏览器直接访问 ip-api.com（每次用一个新生成的随机子域名）完成，你的出口地址对该服务可见；本站不经手、不存储。",
      retryLabel: "重试（将再次由浏览器直连 ip-api.com）",
    },
    O6: {
      title: "UDP 出口一致性",
      meaning:
        "多数代理只稳定接管 TCP。如果 UDP 流量从另一条路径溜出去，暴露的地址可能与你其他流量展示的不同——包括那些直接用 UDP 的服务（WebRTC、部分游戏与语音客户端，以及一些 AI 工具）。",
      reflexiveLabel: "UDP 反射地址",
      exitLabel: "出口 IP",
      mismatch:
        "你的 UDP 流量似乎从与出口 IP 不同的地址出网，UDP 可能正在绕过代理。",
      noMismatch:
        "你的 UDP 流量似乎与出口 IP 从同一地址出网，未检测到 UDP 出口不一致。",
      familyMismatch:
        "与出口 IP 同协议族（IPv4/IPv6）的可比对反射地址不足两个，没有可以在同一基准上比较的对象。",
      unknownExitIp:
        "出口 IP 尚未取得（见上方「出口 IP 与归属」），暂时无法比对。",
      stunDisagree:
        "两个 STUN 服务器给出的地址不一致，没有一个可信的单一值可比——常见于多出口集群或对称 NAT。",
      webrtcUnavailable:
        "浏览器禁用了 WebRTC，本项无法判定 UDP 是否走代理；CLI 不受此限制——它直接使用裸 UDP socket，不依赖浏览器的 WebRTC 栈。",
      stunUnanswered:
        "两个 STUN 服务器均未在超时前应答，多半是暂时的网络问题，可重试。",
      thirdPartyNote:
        "本项通过浏览器的 WebRTC 与 stun.cloudflare.com、stun.l.google.com 两个 STUN 服务器交换请求以取得你的 UDP 反射地址；本站不经手、不存储。",
      retryLabel: "重试（将再次探测 stun.cloudflare.com 与 stun.l.google.com）",
    },
    C1: {
      title: "本机真实 IP",
      meaning:
        "经中国大陆直连回显得到的本机公网地址（规则代理对大陆 IP 走直连）。网页只能看到出口 IP，看不到代理背后的这一个——这是能力边界，不是没做。",
    },
    C2: {
      // 契约 §1 收缩：DNS 泄露判定拆到 O5，本项只剩「本地 DNS 服务器配置」（ADR-0014）。
      title: "本地 DNS 服务器配置",
      meaning:
        "DNS 查询走的是哪台服务器、配置成什么样，网页结构性读不到。查询是否真的经代理出网是另一项检测，见 O5。",
    },
    C3: {
      title: "代理检测（环境变量 / 系统代理 / TUN）",
      meaning:
        "需要读取本机环境变量与系统代理配置。CLI 能看出「你以为开了代理其实没生效」这种情况。",
    },
    C4: {
      title: "$TZ 时区一致性",
      meaning:
        "命令行工具认的是环境变量 $TZ，与系统时区可以不同。网页读不到 $TZ——上面的 O2 测的是系统时区，覆盖不了这一项。",
    },
  },

  sections: {
    online: {
      title: "网页可测到的（6 项）",
      body: "浏览器与 Cloudflare 边缘就能完成，无需在本机装任何东西。",
    },
    cli: {
      title: "需要 CLI 才能测（4 项）",
      body: "这 4 项必须读取本机环境（本地 DNS、系统代理与 TUN、$TZ），网页结构性拿不到，只能由 CLI 在你本机完成。",
    },
  },

  landing: {
    why: {
      title: "为什么需要体检",
      lede: "按 IP 判断访客的工具与服务对网络环境很敏感，最容易踩雷的集中在四类。",
      items: [
        {
          title: "出口 IP 类型与滥用记录",
          body: "机房 IP、公开代理、被大量滥用过的 IP，是触发风控最直接的原因。",
        },
        {
          title: "系统与出口 IP 时区不一致",
          body: "时区对不上是最常见的代理特征之一：IP 显示在美国、系统却是北京时间，风控一眼就能看出来。",
        },
        {
          title: "IPv6 泄露",
          body: "多数代理只接管 IPv4。一旦本机有 IPv6，部分流量会绕过代理直接出去，暴露另一个地区的地址。",
        },
        {
          title: "本地 DNS 暴露访问记录",
          body: "即使其他流量已经走了代理，本地运营商仍能看到你解析过的域名。",
        },
      ],
      checkedOnlineTag: "网页可测",
      foot: "本站可在线检测前三类；本地 DNS 服务器配置需要读取本机环境，网页结构性拿不到，属于仅 CLI 项。",
    },
    install: {
      title: "安装 CLI 补全全部 10 项",
      body: "网页版是快速摸底，能测 6 项；CLI 覆盖全部 10 项，包括本机真实 IP、本地 DNS 配置、代理与 TUN 检测、$TZ 时区一致性。",
      platforms: "macOS · Linux · Windows",
      guideLink: "查看完整使用手册 →",
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
    autoLabel: "自动发起",
    autoBody:
      "页面加载后会自动发起三项检测，均由浏览器直接发出：IPv6 检测访问 ipify，DNS 出口检测访问 ip-api.com，UDP 出口检测访问 stun.cloudflare.com 与 stun.l.google.com。",
    onDemandLabel: "需手动触发",
    onDemandBody:
      "IP 风险检测需你手动触发，届时会先加载 Cloudflare Turnstile 人机验证，随后出口 IP 会被发送至 proxycheck.io 与 StopForumSpam。",
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

  dns: {
    title: "公共 DNS 服务器 · Preflight",
    description:
      "策展的公共 DNS 解析器清单——Cloudflare、Google、Quad9、AliDNS 等，标注地区与过滤级别。",
    heading: "公共 DNS 服务器",
    tableHeaders: {
      ip: "IP",
      provider: "提供商",
      region: "地区",
      filter: "用途",
    },
    variants: {
      standard: "普通解析",
      security: "拦截恶意",
      family: "家庭保护",
      adblock: "拦截广告",
    },
    cliHint:
      "想测你这台机器上实际可用的？在 CLI 里执行 preflight dns --check。",
  },

  guide: {
    title: "CLI 使用手册 · Preflight",
    description:
      "Preflight CLI 的使用方法：安装、命令与参数、退出码、配置、JSON 输出与常见场景。",
    heading: "CLI 使用手册",
    lede: "Preflight CLI 在终端中对网络环境执行一次完整体检——10 项全测，最终给出带覆盖度的综合结论。不存储任何检测结果，探测直连第三方服务，不经过本站服务器。本页仅说明使用方法；各检测项的含义见首页。",
    install: {
      heading: "安装",
      intro:
        "每个平台一条命令——安装脚本会自动从 GitHub Releases 下载对应平台的二进制文件：",
      linuxLabel: "Linux / macOS",
      windowsLabel: "Windows（PowerShell）",
      verify: "安装完成后验证：",
    },
    sections: [
      {
        heading: "快速开始",
        paras: ["直接运行 preflight 即可开始体检，无需任何子命令："],
        code: ["preflight"],
        table: { headers: [], rows: [] },
        after: [
          "所有探测并发执行，完成后一次性输出报告：首先是综合结论（低 / 中 / 高风险，或「数据不足」）及覆盖度（n/10），随后是各检测项的详细结果。",
          "进度提示仅在交互式终端的 stderr 输出——stdout 始终是一份完整的报告，重定向到文件时不会混入进度信息。",
        ],
      },
      {
        heading: "命令与参数",
        paras: ["共四条命令，体检为默认命令："],
        code: [
          "preflight [OPTIONS]            # 体检（默认命令）",
          "preflight dns [--check]        # 公共 DNS 服务器清单",
          "preflight config <ACTION>      # 查看与修改配置",
          "preflight uninstall [--purge]  # 卸载（--purge 连配置一起删）",
        ],
        table: {
          headers: ["参数", "说明"],
          rows: [
            [
              "--lang <LANG>",
              "界面语言：en / zh-hans。全局参数，可置于任意位置。",
            ],
            ["--json", "输出机器可读的 JSON。需置于子命令之前。"],
            ["-v, --verbose", "在报告中附上各检测项的说明。"],
            ["-V / --version", "输出短版本号 / 完整版本信息。"],
          ],
        },
        after: [
          "未显式指定语言时，按以下顺序解析：--lang 参数 > 配置项 language > 系统 locale > 英文。",
        ],
      },
      {
        heading: "体检与退出码",
        paras: ["退出码是脚本集成时的关键约定："],
        code: [],
        table: {
          headers: ["退出码", "含义"],
          rows: [
            [
              "0",
              "体检完成，与风险档位无关。风险档位应从报告或 JSON 输出中读取，而非退出码——若以退出码表达风险，脚本将无法区分「高风险」与「工具运行失败」。",
            ],
            ["1", "工具自身运行失败（配置不合法、语言无效等）。"],
            [
              "2",
              "体检已执行，但未产出任何贡献信号，结论为「数据不足」。报告仍会正常输出。",
            ],
          ],
        },
        after: [
          "当 stdout 不是终端（管道、重定向）时自动关闭彩色输出；也可通过 NO_COLOR 环境变量或 config set no-color true 关闭。报告宽度在渲染前根据终端窗口测量一次；重定向输出时使用固定宽度。",
        ],
      },
      {
        heading: "preflight dns",
        paras: [
          "列出内置的公共 DNS 服务器清单（IP / 提供商 / 地区 / 过滤级别，与本站 DNS 清单页使用同一份数据）。附加 --check 时，将向每台服务器发送真实 DNS 查询，实测连通性与延迟：",
        ],
        code: [
          "preflight dns",
          "preflight dns --check",
          "preflight --json dns --check",
        ],
        table: {
          headers: ["状态", "判据"],
          rows: [
            [
              "ok",
              "收到应答，且 TXID 匹配、RCODE 为 NOERROR、至少包含一条非私网 A 记录。",
            ],
            [
              "suspicious",
              "收到应答但不满足上述全部条件——可能存在劫持或污染。",
            ],
            ["unreachable", "超时未收到有效应答。"],
          ],
        },
        after: [],
      },
      {
        heading: "preflight config",
        paras: [
          "config path 打印配置文件路径；config list 与 config get 打印生效值（合并全部来源之后）；config set 与 config unset 写入配置文件：",
        ],
        code: [
          "preflight config set language zh-hans",
          "preflight config set timeout 20",
          "preflight config set proxycheck-key   # 交互式输入，不回显",
          "preflight config unset timeout",
        ],
        table: {
          headers: ["键", "取值", "默认", "说明"],
          rows: [
            ["language", "en / zh-hans", "跟随系统 locale", "界面语言"],
            [
              "proxycheck-key",
              "交互式输入",
              "未设置",
              "proxycheck.io API key，见下文",
            ],
            ["timeout", "1–120（秒）", "10", "网络探测超时"],
            ["no-color", "true / false", "false", "关闭彩色输出"],
          ],
        },
        after: [
          "可配置的键为白名单——判级阈值与检测项开关不可配置。",
          "API key 有意不提供明文参数：明文会写入 shell 历史记录，并出现在 ps 的进程列表中。",
          "若刚写入的键被更高优先级的来源覆盖（--lang、PROXYCHECK_API_KEY、NO_COLOR），命令会在 stderr 给出提示。",
        ],
      },
      {
        heading: "配置来源与优先级",
        paras: [
          "优先级固定为：命令行参数 > 环境变量 > 配置文件 > 内置默认值。",
          "配置文件为 TOML 格式，键名使用下划线（proxycheck_key、no_color）。遇到未知键会报错退出，而非静默忽略——拼写错误的键不会表现为「已配置但未生效」。Unix 系统上文件权限设为 600。",
        ],
        code: [],
        table: {
          headers: ["平台", "配置文件路径"],
          rows: [
            [
              "Linux / macOS",
              "$XDG_CONFIG_HOME/preflight/config.toml，未设 XDG 时为 ~/.config/preflight/config.toml",
            ],
            ["Windows", "%APPDATA%\\preflight\\config.toml"],
            ["任意", "环境变量 PREFLIGHT_CONFIG 可指定任意路径，优先级最高。"],
          ],
        },
        after: [],
      },
      {
        heading: "环境变量",
        paras: [],
        code: [],
        table: {
          headers: ["变量", "作用"],
          rows: [
            [
              "PROXYCHECK_API_KEY",
              "proxycheck key，优先于配置文件；空值视为未设置。",
            ],
            [
              "NO_COLOR",
              "存在且非空即关闭彩色输出（遵循 no-color.org 约定，不校验具体值）。",
            ],
            ["PREFLIGHT_CONFIG", "覆盖配置文件路径。"],
          ],
        },
        after: [],
      },
      {
        heading: "proxycheck API key",
        paras: [
          "「IP 类型与风险」检测及归属查询通过 proxycheck.io 完成。未设置 key 时使用其匿名额度（每日 100 次查询）；设置免费 key 后提升至每日 1000 次：",
        ],
        code: [
          "preflight config set proxycheck-key    # 或 export PROXYCHECK_API_KEY=…",
          "preflight config get proxycheck-key    # 只报「已设置 / 未设置」",
        ],
        table: { headers: [], rows: [] },
        after: [
          "key 不会出现在任何输出中——包括报告、--json 与错误信息。CLI 直连 proxycheck、使用你自己的配额，不经过本站，也不占用网页版的共享配额。",
        ],
      },
      {
        heading: "--json 输出",
        paras: [
          "preflight --json 输出单个 JSON 对象：verdict（形态与档位）、coverage（done / failed / total）、signals（三态：true / false / null——null 表示未知，不等同于 false），以及 checks 下每个检测项各一个条目（完成时带各自字段，失败时带 upstream / quotaExhausted / local 之一的原因）。",
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
          'preflight --json dns 使用独立 schema：{ "servers": [ … ] }，每项包含 ip / name / region / variant；执行过 --check 时额外包含 check 块。',
        ],
      },
    ],
    scenarios: {
      heading: "典型场景",
      items: [
        {
          title: "切换代理节点后",
          body: "在执行对 IP 敏感的操作前先运行一次体检——确认 DNS、UDP、IPv6 流量均未绕过代理，且出口 IP 的风险分处于正常范围。",
          code: ["preflight"],
        },
        {
          title: "脚本与自动化",
          body: "结合 --json 与退出码使用。风险档位从 JSON 输出中读取；退出码仅表示工具是否正常运行完毕。",
          code: [
            "out=$(preflight --json) || exit 1",
            "level=$(echo \"$out\" | jq -r '.verdict.level')",
            '[ "$level" = "high" ] && echo "高风险出口，中止后续操作" >&2 && exit 1',
          ],
        },
        {
          title: "已启用代理，但仍触发风控",
          body: "查看报告中的 O5（DNS 出口泄露）、O6（UDP 出口一致性）、O3（IPv6 泄露）与 C3（TUN 未开启）；附加 -v 可查看各检测项的说明。",
          code: ["preflight -v"],
        },
        {
          title: "选择可用的公共 DNS",
          body: "在本机实测清单中每台服务器的连通性与延迟，同时识别可疑应答。",
          code: ["preflight dns --check"],
        },
        {
          title: "网络较慢导致探测超时",
          body: "调大探测超时（1–120 秒）。",
          code: ["preflight config set timeout 30"],
        },
      ],
    },
  },

  legal: {
    privacyLink: "隐私说明",
    termsLink: "使用条款",
    updated: "最后更新：2026-08-14",
    privacy: {
      title: "隐私说明 · Preflight",
      heading: "隐私说明",
      lede: "Preflight 没有账号、没有数据库、不做访问统计。你在这里检测的一切都不会被存下来。",
      sections: [
        {
          heading: "我们存了什么",
          body: "什么都没存。每一项检测要么在你的浏览器里算完，要么原样转发给第三方再把结果返给你。检测结果、IP 地址、报告，都不会写进我们持有的任何存储。",
        },
        {
          heading: "浏览器会直接访问的第三方",
          body: "页面加载后会自动发起三项检测，均由浏览器直接发出：IPv6 检测访问 ipify，DNS 出口检测访问 ip-api.com，UDP 出口检测访问 stun.cloudflare.com 与 stun.l.google.com。IP 风险检测只在你手动触发时才跑：先加载 Cloudflare Turnstile 做人机验证，随后出口 IP 会被发送至 proxycheck.io 与 StopForumSpam。这些服务都会看到你的出口 IP，并按各自的隐私政策处理。",
        },
        {
          heading: "Cookie 与本地存储",
          body: "本站不设置任何 Cookie，也不接入访问统计。留在你设备上的只有主题偏好（浅色 / 深色 / 跟随系统），存在 localStorage 的 preflight-theme 键里，不会离开你的浏览器。语言完全由 URL 路径决定，不做任何记忆。",
        },
        {
          heading: "风险查询不带标记",
          body: "发往 proxycheck.io 的请求带 p=0 与 tag=0，因此这次查询不会被打标，也不会作为一条具名查询出现在对方的后台里。",
        },
        {
          heading: "命令行工具",
          body: "Preflight CLI 直接访问第三方服务，检测流量从不经过本站。它除了你自己机器上的配置文件之外不写任何东西。",
        },
        {
          heading: "托管",
          body: "本站跑在 Cloudflare Workers 上，请求由 Cloudflare 边缘按其对网络内任何站点的通行方式处理。",
        },
      ],
    },
    terms: {
      title: "使用条款 · Preflight",
      heading: "使用条款",
      lede: "简短版：这是一个免费的诊断工具，按现状提供。请在你自己负责的网络上使用。",
      sections: [
        {
          heading: "按现状提供",
          body: "Preflight 免费提供、按现状提供，不附带任何形式的担保。检测可能失败、可能返回过时数据，也可能在不预先通知的情况下不可用。",
        },
        {
          heading: "是信号，不是保证",
          body: "每一条结论描述的都是某一时刻、从某一个观测点看到的信号。结论干净不等于你一定匿名、安全或合规；结论被标红也不等于存在违规事实。不要把本工具当作重要决策的唯一依据。",
        },
        {
          heading: "可接受的使用",
          body: "请只用 Preflight 检测你自有或已获授权测试的网络。不要大规模自动化调用这些接口，也不要把它用于攻击或规避风控。本站有限流与每日配额，且可能在不预先通知的情况下执行。",
        },
        {
          heading: "第三方服务",
          body: "检测依赖 ipify、ip-api.com、Cloudflare STUN 与 Turnstile、proxycheck.io、StopForumSpam。它们的数据与条款归其各自所有，我们不对其准确性与可用性作任何陈述。",
        },
        {
          heading: "可用性",
          body: "本站不承诺可用性。接口、检测项与本站本身都可能随时变更或下线。",
        },
        {
          heading: "责任限制",
          body: "在法律允许的最大范围内，我们不对因使用本站或依赖其结果而产生的任何损失或损害承担责任。",
        },
      ],
    },
  },
};
