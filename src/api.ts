// 本站两个接口的客户端，形状与错误码逐条对齐 docs/api.md（normative 契约）。

import { COPY } from "./copy";
import type { GeoData, RiskData } from "./domain/types";

/** 错误码注册表（docs/api.md 第 4 节）。 */
const ERROR_MESSAGE: Record<number, string> = {
  1001: COPY.errors.unknown,
  2010: COPY.errors.humanVerification,
  2020: COPY.errors.rateLimited,
  4001: COPY.errors.unknown,
  5001: COPY.errors.upstream,
  5002: COPY.errors.clientIp,
};

type Envelope<T> =
  | { code: 0; message: string; data: T }
  | { code: number; message: string; details?: string };

/** 抛出的一律是可直接展示给用户的中文短语——失败卡的 reason 就是它。 */
async function request<T>(input: string, init?: RequestInit): Promise<T> {
  let envelope: Envelope<T>;

  try {
    const response = await fetch(input, init);
    envelope = (await response.json()) as Envelope<T>;
  } catch {
    // 网络失败与响应体不可解析对用户是同一件事：这次没测成，可以重试。
    throw new Error(COPY.errors.network);
  }

  if (envelope.code !== 0) {
    throw new Error(ERROR_MESSAGE[envelope.code] ?? COPY.errors.unknown);
  }

  return (envelope as { data: T }).data;
}

export function fetchGeo(): Promise<GeoData> {
  return request<GeoData>("/api/geo");
}

/**
 * O4 按需检测。配额耗尽在契约里是 200 + `status: "quotaExhausted"`（docs/api.md 3.2），
 * 因此它会正常返回、由调用方计入「检测失败」，而不是从这里抛出去。
 *
 * 注意：请求体只带 turnstileToken。契约明令 /api/risk 不接受客户端传入的 IP。
 */
export function fetchRisk(turnstileToken: string): Promise<RiskData> {
  return request<RiskData>("/api/risk", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ turnstileToken }),
  });
}
