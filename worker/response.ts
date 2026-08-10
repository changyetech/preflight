// 统一响应信封与错误码，契约见 docs/api.md 第 1 / 4 节。

/** 错误码注册表（docs/api.md 第 4 节）。新增错误码必须同步登记到契约文档。 */
export const ERROR = {
  PARAMETER: { code: 1001, status: 400, message: "parameter error" },
  HUMAN_VERIFICATION: {
    code: 2010,
    status: 403,
    message: "human verification failed",
  },
  RATE_LIMITED: { code: 2020, status: 429, message: "too many requests" },
  UPSTREAM: { code: 5001, status: 500, message: "upstream unavailable" },
  CLIENT_IP: { code: 5002, status: 500, message: "client ip unavailable" },
} as const;

type ApiError = (typeof ERROR)[keyof typeof ERROR];

export function ok(data: unknown): Response {
  return json({ code: 0, message: "ok", data }, 200);
}

/** details 只放对前端有意义的短语，绝不回填第三方原始错误或密钥（docs/api.md 3.3）。 */
export function fail(error: ApiError, details: string): Response {
  return json(
    { code: error.code, message: error.message, details },
    error.status,
  );
}

function json(body: unknown, status: number): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}
