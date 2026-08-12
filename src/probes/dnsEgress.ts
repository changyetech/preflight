// O5 的探测层：解析一个每次都重新生成的唯一子域，读回 ip-api 的 ECS 观测。
//
// 判定逻辑不在这里——它在 domain/dnsEgress.ts，输入只是两个原样的 geo 字符串。
// 这里刻意不切国家名、不查表：换 provider 时只需换这个文件。

import type { DnsEgressProbe } from "../domain/dnsEgress";

/** 通配符子域。`https://ip-api.com/json/` 免费版返回 403，只有这个子域支持 HTTPS。 */
const ENDPOINT = (label: string) => `https://${label}.edns.ip-api.com/json`;

/** 与 ipify 对齐。 */
const TIMEOUT_MS = 5000;

/** 首次 + 最多重试 2 次（子计划 --web 第 1 步）。 */
const ATTEMPTS = 3;

type IpApiResponse = {
  dns?: { geo?: unknown };
  edns?: { geo?: unknown };
};

function geoOf(section: { geo?: unknown } | undefined): string | null {
  return typeof section?.geo === "string" && section.geo.length > 0
    ? section.geo
    : null;
}

/**
 * 标签长度是 `edns.ip-api.com` 的**外部契约**，不是我们能选的：实测只接受**恰好 32 位**
 * 十六进制，31 位与 33 位一律 404（2026-08-13 curl 实测）。这个数字只能靠人工实测保证，
 * 单测能钉住的只是「我们确实发了 32 位」——见 tests/dnsEgress.test.ts。
 */
const LABEL_BYTES = 16;

/**
 * 唯一子域是为了绕开各级 DNS 缓存——命中缓存拿到的是**别人**的观测值。
 * 因此每次重试都必须换新前缀，重试同一个前缀等于在打自己刚种下的缓存。
 */
function randomLabel(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(LABEL_BYTES));
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join(
    "",
  );
}

/** `signal` 由调用方在组件卸载时中止：不带它，卸载后最长 15s（3 次 × 5s）还在发请求。 */
export async function probeDnsEgress(
  signal?: AbortSignal,
): Promise<DnsEgressProbe> {
  for (let attempt = 0; attempt < ATTEMPTS; attempt += 1) {
    if (signal?.aborted) break;

    try {
      const timeout = AbortSignal.timeout(TIMEOUT_MS);
      const response = await fetch(ENDPOINT(randomLabel()), {
        signal: signal ? AbortSignal.any([signal, timeout]) : timeout,
      });
      if (!response.ok) continue;

      const body = (await response.json()) as IpApiResponse;

      // `dns` 段在真实响应里恒存在，缺了说明对方换了 schema，或用 HTTP 200 返回了
      // JSON 错误体（`{"status":"fail",...}`，ip-api 文档化的行为）。此时若当成
      // 「无 ECS 段」，卡片会打出一句**关于用户 DNS 服务商的假陈述**，把他引向一个
      // 不存在的原因，而真实原因（第三方挂了、刷新可能就好）被完全遮蔽；覆盖度还会
      // 把这次「什么都没测到」记成已完成。与 CLI 是同一条判断（cli/src/probe/dns_egress.rs）。
      if (typeof body?.dns !== "object" || body.dns === null) continue;

      // edns 段缺失即「响应中无 ECS 段」，是判定表里的一行正常输入，不是失败。
      return {
        ok: true,
        ecsGeo: geoOf(body.edns),
        resolverGeo: geoOf(body.dns),
      };
    } catch {
      // 超时、网络失败、响应不是合法 JSON——对判定是同一件事：这次没测成，可以重试。
    }
  }

  return { ok: false };
}
