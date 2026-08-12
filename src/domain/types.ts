// 各检测项的结果数据形状。
// GeoData / RiskData 逐字段对齐 docs/api.md（normative 契约），字段名与可空性不得自行放宽。

/** O1 出口 IP 与归属，见 docs/api.md 第 2 节。字段恒存在，缺失降级为 null。 */
export type GeoData = {
  ip: string | null;
  country: string | null;
  region: string | null;
  city: string | null;
  postalCode: string | null;
  continent: string | null;
  latitude: string | null;
  longitude: string | null;
  timezone: string | null;
  asn: number | null;
  asOrganization: string | null;
  colo: string | null;
};

/** O2 系统时区一致性（规格 2.2）。 */
export type TimezoneResult = {
  /** 浏览器（即系统）时区，IANA 名 */
  browserTimezone: string;
  /** 出口 IP 时区，IANA 名；`null` = 边缘未给出，无法比对 */
  exitTimezone: string | null;
  /** 两者是否一致；`null` = 无法比对 */
  match: boolean | null;
};

/** O3 IPv6 泄露（规格 2.3）。判定表只有「泄露」与「未启用」两个成功态，第三方故障不在此处。 */
export type Ipv6Result =
  { leak: true; ipv6: string } | { leak: false; ipv6: null };

/**
 * O5 的比对结果（判级契约 §2.5 判定表）。
 *
 * 「无从比对」与「未命中」是两回事，因此**不是** `boolean | null` 加注释，而是判别式联合：
 * 没有 ECS、国家名查不到表、出口国未知，三者都不得回退成「两国不同」（§2.5 硬约束 3），
 * 而把它们表达成一个可空布尔量，迟早有人写出 `!leak` 就当成绿色。
 */
export type DnsEgressComparison =
  | {
      comparable: true;
      leak: boolean;
      /** ECS 客户端子网归属国，ISO2 */
      ecsCountry: string;
      /** 出口 IP 归属国，ISO2。两个国家都必须呈现（契约 §5.4 呈现约束） */
      exitCountry: string;
    }
  | {
      comparable: false;
      reason: "noEcs" | "unmappedCountry" | "unknownExitCountry";
    };

/** O5 DNS 出口泄露（判级契约 §2.5）。 */
export type DnsEgressResult = {
  /**
   * 出口 resolver 自身的归属（ip-api 的 `dns.geo` 原样字符串）。
   * **只展示，不判定**（契约 §2.1／§2.5 硬约束 1）：resolver 在哪个国家取决于用户选了哪家
   * DNS 服务商，与流量走没走代理无关，拿它判定是系统性误报。
   */
  resolverGeo: string | null;
  comparison: DnsEgressComparison;
};

/**
 * O6 UDP 出口一致性（判级契约 §2.6 判定表第 2–6 行；第 1 行是检测失败，不在此类型内）。
 *
 * 同样是判别式联合而非可空布尔量：两个 STUN 各报各的（多出口集群、对称 NAT）是
 * **无从比对**，与「UDP 与 TCP 同一个出口」的未命中必须在类型层面就分得开。
 */
export type UdpEgressResult =
  | {
      comparable: true;
      mismatch: boolean;
      /** 两个 STUN 一致报出的反射地址 */
      reflexiveIp: string;
      exitIp: string;
    }
  | {
      comparable: false;
      reason: "familyMismatch" | "unknownExitIp" | "stunDisagree";
    };

/** O4 IP 类型与风险，见 docs/api.md 第 3 节。 */
export type RiskData =
  | {
      status: "ok";
      ip: string;
      networkType: "Residential" | "Business" | "Wireless" | "Hosting" | null;
      proxy: boolean;
      vpn: boolean;
      tor: boolean;
      scraper: boolean;
      riskScore: number;
      /** **分项**分级，不是综合结论——后者的阈值是二维的（docs/verdict.md §3.1 / §6）。 */
      riskLevel: "low" | "medium" | "high";
      /**
       * proxycheck 判定该 IP 当前正被用作匿名化地址。**不是「用户在用 VPN」**。
       * 综合结论判「高」的阈值由它选择（`false` ⇒ ≥ 76，`true` ⇒ ≥ 51）。
       * 与 `riskScore` 必定同时存在（docs/api.md 3.1）。
       */
      anonymous: boolean;
      /** `null` = StopForumSpam 不可用，前端显示「未知」而非「无收录」（docs/api.md 3.1） */
      abuseListed: boolean | null;
    }
  | { status: "quotaExhausted" };
