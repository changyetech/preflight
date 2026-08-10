// POST /api/risk 组装：Turnstile → 限流 → 配额 → 数据源 → 响应（docs/api.md 第 3 节）。
import { env, runInDurableObject } from "cloudflare:test";
import { SELF } from "cloudflare:test";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DAILY_LIMIT, quotaStub, utcDay } from "../worker/quota";

const PROXYCHECK_OK = {
  status: "ok",
  RESULT_IP: {
    network: { type: "Hosting" },
    detections: {
      proxy: false,
      vpn: true,
      tor: false,
      scraper: false,
      risk: 100,
    },
  },
};

interface Outcomes {
  turnstile?: boolean;
  proxycheck?: "ok" | "down";
  abuse?: boolean | "down";
}

/**
 * 按 URL 分派三个第三方的应答。返回的 mock 保留了全部调用记录，
 * 「实际查了哪个 IP」就是从这里断言出来的。
 */
function stubUpstreams({
  turnstile = true,
  proxycheck = "ok",
  abuse = false,
}: Outcomes = {}) {
  const fetchMock = vi.fn(async (url: string, _init?: RequestInit) => {
    if (url.includes("challenges.cloudflare.com")) {
      return Response.json({ success: turnstile });
    }
    if (url.includes("proxycheck.io")) {
      if (proxycheck === "down") return new Response("down", { status: 503 });
      const ip = new URL(url).pathname.replace("/v3/", "");
      return Response.json({ ...PROXYCHECK_OK, [ip]: PROXYCHECK_OK.RESULT_IP });
    }
    if (url.includes("stopforumspam.org")) {
      if (abuse === "down") return new Response("down", { status: 503 });
      return Response.json({ success: 1, ip: { appears: abuse ? 1 : 0 } });
    }
    throw new Error(`未预期的第三方调用: ${url}`);
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function postRisk(ip: string, body: unknown) {
  return SELF.fetch("https://example.com/api/risk", {
    method: "POST",
    headers: { "CF-Connecting-IP": ip, "content-type": "application/json" },
    body: typeof body === "string" ? body : JSON.stringify(body),
  });
}

/** 把配额计数摆到指定状态。 */
async function seedQuota(day: string, used: number) {
  await runInDurableObject(quotaStub(env), (_instance, state) => {
    state.storage.sql.exec(
      "INSERT OR REPLACE INTO quota (id, day, used) VALUES (1, ?, ?)",
      day,
      used,
    );
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("POST /api/risk 只用来源 IP", () => {
  // 这是本接口最要紧的一条：一旦客户端能指定查询目标，本站就退化成任意 IP 查询代理，
  // proxycheck 配额会被第三方白嫖（规格 5.1 / 验收标准 6）。
  it("请求体传入伪造 IP 时，实际查询的仍是来源 IP", async () => {
    const fetchMock = stubUpstreams();

    const response = await postRisk("203.0.113.10", {
      turnstileToken: "good",
      ip: "8.8.8.8",
    });

    expect(response.status).toBe(200);

    const calls = fetchMock.mock.calls.map(([url]) => url);
    const proxycheckCall = calls.find((url) => url.includes("proxycheck.io"))!;
    expect(new URL(proxycheckCall).pathname).toBe("/v3/203.0.113.10");
    expect(calls.some((url) => url.includes("8.8.8.8"))).toBe(false);

    const body = (await response.json()) as { data: { ip: string } };
    expect(body.data.ip).toBe("203.0.113.10");
  });

  it("查询串传入伪造 IP 同样被忽略", async () => {
    const fetchMock = stubUpstreams();

    await SELF.fetch("https://example.com/api/risk?ip=8.8.8.8", {
      method: "POST",
      headers: { "CF-Connecting-IP": "203.0.113.11" },
      body: JSON.stringify({ turnstileToken: "good" }),
    });

    const calls = fetchMock.mock.calls.map(([url]) => url);
    expect(calls.some((url) => url.includes("8.8.8.8"))).toBe(false);
    expect(calls.some((url) => url.includes("/v3/203.0.113.11"))).toBe(true);
  });

  it("StopForumSpam 也只查来源 IP", async () => {
    const fetchMock = stubUpstreams();

    await postRisk("203.0.113.12", { turnstileToken: "good", ip: "8.8.8.8" });

    const sfsCall = fetchMock.mock.calls
      .map(([url]) => url)
      .find((url) => url.includes("stopforumspam.org"))!;
    expect(new URL(sfsCall).searchParams.get("ip")).toBe("203.0.113.12");
  });
});

describe("POST /api/risk 成功响应", () => {
  it("组装 proxycheck 与 StopForumSpam 的结果", async () => {
    stubUpstreams({ abuse: true });

    const response = await postRisk("203.0.113.20", { turnstileToken: "good" });
    const body = (await response.json()) as { code: number; data: object };

    expect(response.status).toBe(200);
    expect(body.code).toBe(0);
    expect(body.data).toEqual({
      status: "ok",
      ip: "203.0.113.20",
      networkType: "Hosting",
      proxy: false,
      vpn: true,
      tor: false,
      scraper: false,
      riskScore: 100,
      riskLevel: "high",
      abuseListed: true,
    });
  });

  it("StopForumSpam 不可用时 abuseListed 为 null，仍返回 200", async () => {
    stubUpstreams({ abuse: "down" });

    const response = await postRisk("203.0.113.21", { turnstileToken: "good" });
    const body = (await response.json()) as { data: { abuseListed: null } };

    expect(response.status).toBe(200);
    expect(body.data.abuseListed).toBeNull();
  });

  it("响应中不含 proxycheck API key", async () => {
    stubUpstreams();

    const response = await postRisk("203.0.113.22", { turnstileToken: "good" });

    expect(await response.text()).not.toContain(env.PROXYCHECK_API_KEY);
  });
});

describe("POST /api/risk 拒绝与降级", () => {
  it("无 token 时 403 且不消耗任何第三方调用", async () => {
    const fetchMock = stubUpstreams();

    const response = await postRisk("203.0.113.30", {});
    const body = (await response.json()) as { code: number };

    expect(response.status).toBe(403);
    expect(body.code).toBe(2010);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("token 无效时 403，且不调用 proxycheck", async () => {
    const fetchMock = stubUpstreams({ turnstile: false });

    const response = await postRisk("203.0.113.31", { turnstileToken: "bad" });
    const body = (await response.json()) as { code: number };

    expect(response.status).toBe(403);
    expect(body.code).toBe(2010);
    expect(
      fetchMock.mock.calls.some(([url]) => url.includes("proxycheck.io")),
    ).toBe(false);
  });

  it("请求体不是合法 JSON 时 400", async () => {
    stubUpstreams();

    const response = await postRisk("203.0.113.32", "not json");
    const body = (await response.json()) as { code: number };

    expect(response.status).toBe(400);
    expect(body.code).toBe(1001);
  });

  it("拿不到来源 IP 时 500 / 5002", async () => {
    stubUpstreams();

    const response = await SELF.fetch("https://example.com/api/risk", {
      method: "POST",
      body: JSON.stringify({ turnstileToken: "good" }),
    });
    const body = (await response.json()) as { code: number };

    expect(response.status).toBe(500);
    expect(body.code).toBe(5002);
  });

  it("proxycheck 不可用时 500 / 5001", async () => {
    stubUpstreams({ proxycheck: "down" });

    const response = await postRisk("203.0.113.33", { turnstileToken: "good" });
    const body = (await response.json()) as { code: number };

    expect(response.status).toBe(500);
    expect(body.code).toBe(5001);
  });

  it("配额耗尽时返回 200 + status quotaExhausted，而不是错误", async () => {
    const fetchMock = stubUpstreams();
    await seedQuota(utcDay(new Date()), DAILY_LIMIT);

    const response = await postRisk("203.0.113.34", { turnstileToken: "good" });
    const body = (await response.json()) as {
      code: number;
      message: string;
      data: object;
    };

    expect(response.status).toBe(200);
    expect(body.code).toBe(0);
    expect(body.message).toBe("ok");
    // 没有查询发生，就没有结果可报：data 只有 status 一个键
    expect(body.data).toEqual({ status: "quotaExhausted" });
    expect(
      fetchMock.mock.calls.some(([url]) => url.includes("proxycheck.io")),
    ).toBe(false);
  });

  it("GET /api/risk 返回 404（本接口只接受 POST）", async () => {
    const response = await SELF.fetch("https://example.com/api/risk");

    expect(response.status).toBe(404);
  });
});

describe("POST /api/risk 限流", () => {
  it("单 IP 连续超限后返回 429 / 2020", async () => {
    stubUpstreams();

    const statuses: number[] = [];
    for (let i = 0; i < 15; i++) {
      const response = await postRisk("203.0.113.99", {
        turnstileToken: "good",
      });
      statuses.push(response.status);
      await response.text();
    }

    expect(statuses).toContain(429);
    // 限流必须发生在配额之前：被限流的请求不应烧掉 proxycheck 额度
    const limitedAt = statuses.indexOf(429);
    expect(statuses.slice(limitedAt).every((status) => status === 429)).toBe(
      true,
    );
  });
});
