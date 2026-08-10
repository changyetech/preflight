// Turnstile 服务端校验，只作用于 /api/risk（规格 5 节）。
// TURNSTILE_SECRET_KEY 只存在于 Worker Secret，不进仓库、不进响应。

export const SITEVERIFY_URL =
  "https://challenges.cloudflare.com/turnstile/v0/siteverify";

/**
 * 校验前端提交的 Turnstile token。
 *
 * 一律 fail closed：siteverify 不可用时按「未通过」处理。放行才是危险的默认——
 * /api/risk 背后是要花钱的 proxycheck 配额，宁可这段时间没人能查，也不能被机器人刷。
 */
export async function verifyTurnstile(
  token: string | null,
  secret: string,
  remoteip: string,
): Promise<boolean> {
  if (!token) {
    return false;
  }

  try {
    const response = await fetch(SITEVERIFY_URL, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ secret, response: token, remoteip }),
    });

    if (!response.ok) {
      return false;
    }

    const result = (await response.json()) as { success?: boolean };
    return result.success === true;
  } catch {
    return false;
  }
}
