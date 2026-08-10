// proxycheck.io v3 适配（ADR-0007）。
// 单次调用即取得 network.type、Proxy/VPN/TOR/Scraper 布尔量与风险分；不用它的地理段，
// 归属数据继续走 request.cf（免费无限，见 docs/api.md 第 2 节）。
//
// PROXYCHECK_API_KEY 只存在于 Worker Secret，不进仓库、不进响应、不进日志（ADR-0008）。

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
}

/** 分项分级沿用 CLI：< 30 绿 / < 70 黄 / >= 70 红（规格 3.2）。 */
export function riskLevelOf(score: number): RiskLevel {
  if (score < 30) return "low";
  if (score < 70) return "medium";
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
  const riskScore = detections.risk ?? 0;

  return {
    networkType: entry.network?.type ?? null,
    proxy: detections.proxy === true,
    vpn: detections.vpn === true,
    tor: detections.tor === true,
    scraper: detections.scraper === true,
    riskScore,
    riskLevel: riskLevelOf(riskScore),
  };
}
