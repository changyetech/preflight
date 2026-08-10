// 本站两个接口的客户端，形状与错误码逐条对齐 docs/api.md（normative 契约）。

import { COPY } from "./copy";
import type { GeoData, RiskData } from "./domain/types";

/**
 * 错误码注册表（docs/api.md 第 4 节）。
 *
 * 1001 / 4001 代表前端自己发错了请求（请求体不合法、路由或方法不对），重试一万次也一样，
 * 因此不能给「请稍后重试」这种措辞——那会让用户对着一个不可能好转的按钮反复点。
 */
const ERROR_MESSAGE: Record<number, string> = {
  1001: COPY.errors.badRequest,
  2010: COPY.errors.humanVerification,
  2020: COPY.errors.rateLimited,
  4001: COPY.errors.badRequest,
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

  // code 为 0 却没有 data，是契约被违反了。当作失败抛出去，
  // 否则畸形响应会一路流到卡片里渲染成一张空卡——看起来像「查过了、什么都没有」。
  const { data } = envelope as { data?: T };
  if (data === undefined || data === null) {
    throw new Error(COPY.errors.malformed);
  }

  return data;
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
export async function fetchRisk(turnstileToken: string): Promise<RiskData> {
  const data = await request<RiskData>("/api/risk", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ turnstileToken }),
  });

  // status 是契约里的判别式（docs/api.md 3.1 / 3.2）。取值超出这两个，
  // 后面的 RiskDetail 会渲染出一串 undefined 字段——那比直接判失败更误导。
  if (data.status !== "ok" && data.status !== "quotaExhausted") {
    throw new Error(COPY.errors.malformed);
  }

  return data;
}
