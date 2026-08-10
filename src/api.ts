// 本站两个接口的客户端，形状与错误码逐条对齐 docs/api.md（normative 契约）。

import { COPY, type Copy } from "./copy";
import type { GeoData, RiskData } from "./domain/types";

/**
 * 错误码注册表（docs/api.md 第 4 节）。
 *
 * 1001 / 4001 代表前端自己发错了请求（请求体不合法、路由或方法不对），重试一万次也一样，
 * 因此不能给「请稍后重试」这种措辞——那会让用户对着一个不可能好转的按钮反复点。
 */
function errorMessageOf(copy: Copy, code: number): string {
  const table: Record<number, string> = {
    1001: copy.errors.badRequest,
    2010: copy.errors.humanVerification,
    2020: copy.errors.rateLimited,
    4001: copy.errors.badRequest,
    5001: copy.errors.upstream,
    5002: copy.errors.clientIp,
  };
  return table[code] ?? copy.errors.unknown;
}

type Envelope<T> =
  | { code: 0; message: string; data: T }
  | { code: number; message: string; details?: string };

/** 抛出的一律是可直接展示给用户的短语（随 `copy` 语言变化）——失败卡的 reason 就是它。 */
async function request<T>(
  input: string,
  copy: Copy,
  init?: RequestInit,
): Promise<T> {
  let envelope: Envelope<T>;

  try {
    const response = await fetch(input, init);
    envelope = (await response.json()) as Envelope<T>;
  } catch {
    // 网络失败与响应体不可解析对用户是同一件事：这次没测成，可以重试。
    throw new Error(copy.errors.network);
  }

  if (envelope.code !== 0) {
    throw new Error(errorMessageOf(copy, envelope.code));
  }

  // code 为 0 却没有 data，是契约被违反了。当作失败抛出去，
  // 否则畸形响应会一路流到卡片里渲染成一张空卡——看起来像「查过了、什么都没有」。
  const { data } = envelope as { data?: T };
  if (data === undefined || data === null) {
    throw new Error(copy.errors.malformed);
  }

  return data;
}

/** `copy` 默认中文，供既有调用方（含测试）不传参也能工作；usePanel 会按当前语言显式传入。 */
export function fetchGeo(copy: Copy = COPY): Promise<GeoData> {
  return request<GeoData>("/api/geo", copy);
}

/**
 * O4 按需检测。配额耗尽在契约里是 200 + `status: "quotaExhausted"`（docs/api.md 3.2），
 * 因此它会正常返回、由调用方计入「检测失败」，而不是从这里抛出去。
 *
 * 注意：请求体只带 turnstileToken。契约明令 /api/risk 不接受客户端传入的 IP。
 */
export async function fetchRisk(
  turnstileToken: string,
  copy: Copy = COPY,
): Promise<RiskData> {
  const data = await request<RiskData>("/api/risk", copy, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ turnstileToken }),
  });

  // status 是契约里的判别式（docs/api.md 3.1 / 3.2）。取值超出这两个，
  // 后面的 RiskDetail 会渲染出一串 undefined 字段——那比直接判失败更误导。
  if (data.status !== "ok" && data.status !== "quotaExhausted") {
    throw new Error(copy.errors.malformed);
  }

  return data;
}
