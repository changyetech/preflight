// proxycheck.io v3 适配（ADR-0007）。
// 单次调用即取得 network.type、Proxy/VPN/TOR/Scraper 布尔量与风险分；不用它的地理段，
// 归属数据继续走 request.cf（免费无限，见 docs/api.md 第 2 节）。
//
// PROXYCHECK_API_KEY 只存在于 Worker Secret，不进仓库、不进响应、不进日志（ADR-0008）。
//
// 第三方的行为细节（基准分表、配额、必带参数、已知坑）见 docs/proxycheck.md。

export type NetworkType =
  "Residential" | "Business" | "Wireless" | "Hosting" | null;

export type RiskLevel = "low" | "medium" | "high";

export interface ProxycheckResult {
  networkType: NetworkType;
  proxy: boolean;
  vpn: boolean;
  tor: boolean;
  scraper: boolean;
  riskScore: number;
  riskLevel: RiskLevel;
  /**
   * proxycheck 判定该 IP 当前正被用作匿名化地址。**不是「用户在用 VPN」**——
   * 实测普通商业 VPN 出口是 false，TOR 出口是 true。
   * 综合结论判「高」的阈值由它选择（判级契约 §3.1），所以必须透给前端。
   */
  anonymous: boolean;
}

/**
 * 分项分级（判级契约 §6）：`< 26` 绿 / `< 76` 黄 / `>= 76` 红。
 *
 * 三个分界直接对齐 proxycheck v3 自己的分档（0–25 / 26–50 / 51–75 / 76–100）：
 * 四档收成三色时中间两档并作黄——**绿 = 它建议放行的区间，红 = 它对任何 IP 都建议
 * 拒绝的区间**。依据见 docs/proxycheck.md。
 *
 * **这与综合结论的阈值是两个常量**：结论是二维的（51 或 76），分项只看分数。
 * `anonymous: true` 时两者不同界——结论 51 起判高，分项 76 才转红。
 */
export function riskLevelOf(score: number): RiskLevel {
  if (score < 26) return "low";
  if (score < 76) return "medium";
  return "high";
}

/** v3 响应形状，只声明我们要读的字段（官方：所有键恒存在，缺数据时值为 null）。 */
interface V3Response {
  status?: string;
  [ip: string]:
    | undefined
    | string
    | number
    | {
        network?: { type?: NetworkType };
        detections?: {
          proxy?: boolean;
          vpn?: boolean;
          tor?: boolean;
          scraper?: boolean;
          risk?: number;
          anonymous?: boolean;
        };
      };
}

/** 返回 null 表示上游不可用，调用方据此返回 5001（docs/api.md 3.3）。 */
export async function fetchProxycheck(
  ip: string,
  apiKey: string,
): Promise<ProxycheckResult | null> {
  const url = new URL(`https://proxycheck.io/v3/${encodeURIComponent(ip)}`);
  url.searchParams.set("key", apiKey);
  // p=0：机器可读的紧凑输出。tag=0：不把本次查询写进 proxycheck 的正向检出日志（ADR-0008）。
  url.searchParams.set("p", "0");
  url.searchParams.set("tag", "0");

  let body: V3Response;
  try {
    const response = await fetch(url.toString());
    if (!response.ok) {
      return null;
    }
    body = (await response.json()) as V3Response;
  } catch {
    return null;
  }

  // status 可能是 warning / denied / error，比如配额被 proxycheck 侧拒绝。
  if (body.status !== "ok") {
    return null;
  }

  const entry = body[ip];
  if (typeof entry !== "object" || entry === null) {
    return null;
  }

  const detections = entry.detections ?? {};
  const riskScore = detections.risk;
  // 风险分缺失（上游改字段名、给 null 等）一律视为数据源不可用，走 5001。
  // 绝不能默认成 0——那会把有风险的 IP 静默报成「低风险」，与 stopforumspam 那边
  // 坚持的「查不到不能谎报安全」自相矛盾。
  if (typeof riskScore !== "number") {
    return null;
  }

  // anonymous 与 riskScore 必须成对：判「高」的阈值由前者决定（判级契约 §3.1）。
  // 缺它时默认成 false 会把阈值静默抬到 76，本该判高的 IP 悄悄变成低——
  // 静默降级比响亮失败难查得多，所以这里同样走 5001。
  const { anonymous } = detections;
  if (typeof anonymous !== "boolean") {
    return null;
  }

  return {
    networkType: entry.network?.type ?? null,
    proxy: detections.proxy === true,
    vpn: detections.vpn === true,
    tor: detections.tor === true,
    scraper: detections.scraper === true,
    riskScore,
    riskLevel: riskLevelOf(riskScore),
    anonymous,
  };
}
