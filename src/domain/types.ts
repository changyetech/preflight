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
      riskLevel: "low" | "medium" | "high";
      /** `null` = StopForumSpam 不可用，前端显示「未知」而非「无收录」（docs/api.md 3.1） */
      abuseListed: boolean | null;
    }
  | { status: "quotaExhausted" };
