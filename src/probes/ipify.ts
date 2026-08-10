// O3 的探测层：向 ipify 的双栈端点各发一次请求（ADR-0003）。
//
// 判定逻辑不在这里——它在 domain/ipv6.ts，输入只是两个 Probe。
// 这条缝是刻意留的：规格 5.3 的降级方案（零依赖启发式，只看浏览器连本站用的是 v4 还是 v6）
// 只需换一个产出 Probe 对的实现，judgeIpv6 与上层状态机一行不用动。

import type { Probe } from "../domain/ipv6";

const V4_ENDPOINT = "https://api.ipify.org?format=json";
const V6_ENDPOINT = "https://api6.ipify.org?format=json";

/** 探测超时。没有它，无 IPv6 的用户要等浏览器把连接耗到底才看到结果。 */
const TIMEOUT_MS = 5000;

async function probe(endpoint: string): Promise<Probe> {
  try {
    const response = await fetch(endpoint, {
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
    if (!response.ok) return { reachable: false };

    const { ip } = (await response.json()) as { ip?: unknown };
    return typeof ip === "string" && ip.length > 0
      ? { reachable: true, ip }
      : { reachable: false };
  } catch {
    // 浏览器把 CORS 失败与网络失败抛成同一个不透明 TypeError，这里无从区分，
    // 也不需要区分——区分是 v4 对照端点的职责（ADR-0003）。
    return { reachable: false };
  }
}

export async function probeIpify(): Promise<{ v4: Probe; v6: Probe }> {
  const [v4, v6] = await Promise.all([probe(V4_ENDPOINT), probe(V6_ENDPOINT)]);
  return { v4, v6 };
}
