// O1 出口 IP 与归属：地理数据来源抽成纯函数（规格 5.2），测试注入固定 cf 值，不依赖真实 request.cf。
import { SELF } from "cloudflare:test";
import { describe, expect, it } from "vitest";

import { GEO_FIELDS, geoFrom } from "../worker/geo";

// 一份「字段齐全」的 cf 固定值，字段名与 Workers 的 IncomingRequestCfProperties 对齐。
const FULL_CF = {
  country: "CN",
  region: "Shanghai",
  city: "Shanghai",
  postalCode: "200000",
  continent: "AS",
  latitude: "31.22222",
  longitude: "121.45806",
  timezone: "Asia/Shanghai",
  asn: 4134,
  asOrganization: "Chinanet",
  colo: "SHA",
};

describe("geoFrom", () => {
  it("字段齐全时逐字段派生 O1 数据", () => {
    const geo = geoFrom(FULL_CF, "1.2.3.4");

    expect(geo).toEqual({
      ip: "1.2.3.4",
      country: "CN",
      region: "Shanghai",
      city: "Shanghai",
      postalCode: "200000",
      continent: "AS",
      latitude: "31.22222",
      longitude: "121.45806",
      timezone: "Asia/Shanghai",
      asn: 4134,
      asOrganization: "Chinanet",
      colo: "SHA",
    });
  });

  it("单个字段缺失时降级为 null，其余字段不受影响", () => {
    const { city: _city, ...withoutCity } = FULL_CF;

    const geo = geoFrom(withoutCity, "1.2.3.4");

    expect(geo.city).toBeNull();
    expect(geo.country).toBe("CN");
    expect(geo.timezone).toBe("Asia/Shanghai");
  });

  it("cf 整体缺失时所有字段为 null，但键恒存在", () => {
    const geo = geoFrom(undefined, null);

    expect(Object.keys(geo).sort()).toEqual([...GEO_FIELDS, "ip"].sort());
    expect(Object.values(geo).every((value) => value === null)).toBe(true);
  });

  it("空字符串按缺失处理，降级为 null", () => {
    const geo = geoFrom({ ...FULL_CF, city: "" }, "1.2.3.4");

    expect(geo.city).toBeNull();
  });
});

describe("GET /api/geo", () => {
  it("返回成功信封且 data 含全部 O1 键", async () => {
    const response = await SELF.fetch("https://example.com/api/geo");

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe(
      "application/json; charset=utf-8",
    );

    const body = (await response.json()) as { code: number; message: string; data: object };
    expect(body.code).toBe(0);
    expect(body.message).toBe("ok");
    expect(Object.keys(body.data).sort()).toEqual([...GEO_FIELDS, "ip"].sort());
  });

  it("使用 CF-Connecting-IP 作为出口 IP", async () => {
    const response = await SELF.fetch("https://example.com/api/geo", {
      headers: { "CF-Connecting-IP": "203.0.113.7" },
    });

    const body = (await response.json()) as { data: { ip: string } };
    expect(body.data.ip).toBe("203.0.113.7");
  });
});
