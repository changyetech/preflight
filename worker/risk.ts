// POST /api/risk：O4 IP 类型与风险，契约见 docs/api.md 第 3 节。
//
// 处理顺序 Turnstile → 限流 → 配额 → 数据源，是按「成本从低到高」排的：
// 前一道拦住的请求，绝不会消耗后一道的资源。

import type { Env } from "./env";
import { fetchProxycheck } from "./proxycheck";
import { ERROR, fail, ok } from "./response";
import { quotaStub, utcDay } from "./quota";
import { fetchAbuseListed } from "./stopforumspam";
import { verifyTurnstile } from "./turnstile";

export async function handleRisk(
  request: Request,
  env: Env,
): Promise<Response> {
  // 查询目标恒为来源 IP。请求体与查询串里的任何 IP 字段都不读、不用——
  // 一旦客户端能指定目标，本站就退化成任意 IP 查询代理，proxycheck 配额会被白嫖（规格 5.1）。
  const ip = request.headers.get("CF-Connecting-IP");
  if (!ip) {
    return fail(ERROR.CLIENT_IP, "missing source ip");
  }

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return fail(ERROR.PARAMETER, "body must be valid json");
  }

  const token = (body as { turnstileToken?: unknown } | null)?.turnstileToken;
  const verified = await verifyTurnstile(
    typeof token === "string" ? token : null,
    env.TURNSTILE_SECRET_KEY,
    ip,
  );
  if (!verified) {
    return fail(ERROR.HUMAN_VERIFICATION, "turnstile token missing or invalid");
  }

  const { success } = await env.RISK_RATE_LIMITER.limit({ key: ip });
  if (!success) {
    return fail(ERROR.RATE_LIMITED, "too many risk lookups from this ip");
  }

  // 配额耗尽是容量状态而非故障：返回 200 让前端优雅降级为「今日额度已用尽」，
  // 用错误码表达会诱导前端当故障重试，反而放大问题（规格 5.3 / docs/api.md 3.2）。
  const allowed = await quotaStub(env).consume(utcDay(new Date()));
  if (!allowed) {
    return ok({ status: "quotaExhausted" });
  }

  const [proxycheck, abuseListed] = await Promise.all([
    fetchProxycheck(ip, env.PROXYCHECK_API_KEY),
    fetchAbuseListed(ip),
  ]);

  // StopForumSpam 挂了不连累 proxycheck 的结果，abuseListed 保持 null 即可（docs/api.md 3.1）。
  if (!proxycheck) {
    return fail(ERROR.UPSTREAM, "proxycheck unavailable");
  }

  return ok({ status: "ok", ip, ...proxycheck, abuseListed });
}
