// O1 出口 IP 与归属，契约见 docs/api.md 第 2 节。
//
// 地理数据来源抽成 geoFrom 这个纯函数（规格 5.2）：生产传入 request.cf，测试传入固定值。
// 这样单测不依赖 request.cf 的真值——`wrangler dev` 本地模式给的本就是占位值（规格第 9 节未决事实 #2）。

import { ok } from "./response";

/** 取自 request.cf 的字段，全部套餐可用（规格 2.1）。 */
export const GEO_FIELDS = [
  "country",
  "region",
  "city",
  "postalCode",
  "continent",
  "latitude",
  "longitude",
  "timezone",
  "asn",
  "asOrganization",
  "colo",
] as const;

type GeoField = (typeof GEO_FIELDS)[number];

export type Geo = { ip: string | null } & {
  [K in GeoField]: string | number | null;
};

/** cf 的字段类型由 Workers 决定（asn 是数字，经纬度是字符串），这里只关心「有值 / 没值」。 */
type CfLike = Partial<Record<GeoField, unknown>>;

export function geoFrom(cf: CfLike | undefined, ip: string | null): Geo {
  const geo = { ip: ip || null } as Geo;

  for (const field of GEO_FIELDS) {
    const value = cf?.[field];
    // 空字符串按缺失处理：cf 偶尔会给空串，前端把它当有效值会渲染出空白行。
    geo[field] = value === undefined || value === null || value === "" ? null : (value as string | number);
  }

  return geo;
}

export function handleGeo(request: Request): Response {
  return ok(
    geoFrom(
      request.cf as CfLike | undefined,
      request.headers.get("CF-Connecting-IP"),
    ),
  );
}
