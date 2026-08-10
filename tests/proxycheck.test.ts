// proxycheck v3 适配（ADR-0007：单次调用取网络类型、代理检出与风险分，不再前置 ip-api）。
import { afterEach, describe, expect, it, vi } from "vitest";

import { fetchProxycheck, riskLevelOf } from "../worker/proxycheck";

/** 取自 proxycheck 官方 v3 文档的响应样例（185.59.221.75，Datacamp 的 VPN 出口）。 */
const FIXTURE = {
  status: "ok",
  "185.59.221.75": {
    network: {
      asn: "AS60068",
      range: "185.59.221.0/24",
      hostname: "185.59.221.75.adsl.inet-telecom.org",
      provider: "Datacamp Limited",
      organisation: "CDN77 - London POP",
      type: "Hosting",
    },
    location: { country_name: "United Kingdom", country_code: "GB" },
    device_estimate: { address: 50, subnet: 1890 },
    detections: {
      proxy: false,
      vpn: true,
      compromised: true,
      scraper: false,
      tor: false,
      hosting: true,
      anonymous: true,
      risk: 100,
    },
    operator: { name: "IVPN", url: "https://www.ivpn.net/" },
    last_updated: "2025-10-12T00:43:53Z",
  },
  query_time: 5,
};

function stubFetch(reply: unknown | Error) {
  const fetchMock = vi.fn(async () => {
    if (reply instanceof Error) throw reply;
    if (reply instanceof Response) return reply;
    return Response.json(reply);
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

/** 造一份只改风险分的响应，用于分级边界断言。 */
function fixtureWithRisk(ip: string, risk: number) {
  return {
    status: "ok",
    [ip]: {
      network: { type: "Residential" },
      detections: { proxy: false, vpn: false, tor: false, scraper: false, risk },
    },
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("riskLevelOf", () => {
  // 分项分级沿用 CLI：< 30 绿 / < 70 黄 / >= 70 红（规格 3.2）。
  it.each([
    [0, "low"],
    [29, "low"],
    [30, "medium"],
    [69, "medium"],
    [70, "high"],
    [100, "high"],
  ])("风险分 %i 分级为 %s", (score, level) => {
    expect(riskLevelOf(score)).toBe(level);
  });
});

describe("fetchProxycheck", () => {
  it("从 fixture 解析出网络类型、代理布尔量与风险分", async () => {
    stubFetch(FIXTURE);

    await expect(fetchProxycheck("185.59.221.75", "k")).resolves.toEqual({
      networkType: "Hosting",
      proxy: false,
      vpn: true,
      tor: false,
      scraper: false,
      riskScore: 100,
      riskLevel: "high",
    });
  });

  it("按被查 IP 组装 v3 路径并携带 key，同时关闭查询留存", async () => {
    const fetchMock = stubFetch(FIXTURE);

    await fetchProxycheck("185.59.221.75", "my-secret-key");

    const url = new URL(fetchMock.mock.calls[0][0] as unknown as string);
    expect(url.origin).toBe("https://proxycheck.io");
    expect(url.pathname).toBe("/v3/185.59.221.75");
    expect(url.searchParams.get("key")).toBe("my-secret-key");
    // tag=0：不把本次查询写进 proxycheck 的正向检出日志（ADR-0008 零留存）
    expect(url.searchParams.get("tag")).toBe("0");
  });

  it.each([
    [10, "low"],
    [50, "medium"],
    [80, "high"],
  ])("风险分 %i 的解析结果分级为 %s", async (risk, level) => {
    stubFetch(fixtureWithRisk("1.2.3.4", risk));

    const result = await fetchProxycheck("1.2.3.4", "k");

    expect(result?.riskScore).toBe(risk);
    expect(result?.riskLevel).toBe(level);
  });

  it("网络类型未知时为 null", async () => {
    stubFetch({
      status: "ok",
      "1.2.3.4": {
        network: { type: null },
        detections: { proxy: true, vpn: false, tor: false, scraper: false, risk: 100 },
      },
    });

    await expect(fetchProxycheck("1.2.3.4", "k")).resolves.toMatchObject({
      networkType: null,
      proxy: true,
    });
  });

  it("status 非 ok 时视为上游失败", async () => {
    stubFetch({ status: "denied", message: "Daily queries exhausted" });

    await expect(fetchProxycheck("1.2.3.4", "k")).resolves.toBeNull();
  });

  it("缺少被查 IP 的结果段时视为上游失败", async () => {
    stubFetch({ status: "ok", query_time: 3 });

    await expect(fetchProxycheck("1.2.3.4", "k")).resolves.toBeNull();
  });

  it("HTTP 非 200 或网络异常时视为上游失败", async () => {
    stubFetch(new Response("nope", { status: 429 }));
    await expect(fetchProxycheck("1.2.3.4", "k")).resolves.toBeNull();

    stubFetch(new Error("network down"));
    await expect(fetchProxycheck("1.2.3.4", "k")).resolves.toBeNull();
  });
});
