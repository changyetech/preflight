// O5 DNS 出口泄露判定（判级契约 §2.5 判定表）。
//
// 判据是**出口 resolver 眼里客户端在哪个国家**，与出口 IP 的归属国比。
// 探测层只把 ip-api 的两个 geo 字符串原样递过来，切国家名与查表都在这里——
// 这条缝与 ipify/judgeIpv6 同一条，换 provider 时上层一行不用动。

import type { OnlineCheck } from "./checks";
import type { DnsEgressComparison, DnsEgressResult } from "./types";
import countryCodes from "../../docs/country-codes.json";

/** 单次 DNS 出口探测的原始观测。`ok: false` = 网络错误或响应不可解析（判定表最后一行）。 */
export type DnsEgressProbe =
  | {
      ok: true;
      /** ECS 客户端子网归属，形如 `"Japan - IT7 Networks Inc"`；`null` = 响应里没有 ECS 段 */
      ecsGeo: string | null;
      /** 出口 resolver 自身的归属，只展示不判定 */
      resolverGeo: string | null;
    }
  | { ok: false };

/**
 * ECS 客户端子网的归属国，已归一化为 ISO2。
 *
 * 两种「未知」必须分开：不发 ECS 的服务商（Cloudflare 1.1.1.1 是明确的一家）与
 * 认不出的国家名，前者是用户可以换 DNS 解决的，后者是我们的表不全。
 */
export type EcsCountry =
  | { known: true; iso2: string }
  | { known: false; reason: "noEcs" | "unmappedCountry" };

/** 探测失败的内部诊断标识，不面向用户展示（文案在呈现层，见 judgeIpv6 的同款说明）。 */
export const DNS_EGRESS_PROBE_FAILED = "dns-egress-probe-failed";

/**
 * 英文国家名 → ISO2。两端共吃 docs/country-codes.json，Web 侧打进 bundle。
 * 用 Map 而不是直接下标取值：对象下标会命中 `constructor` 之类的原型键。
 */
const ISO2_BY_COUNTRY_NAME = new Map<string, string>(
  Object.entries(countryCodes.countries),
);

/** ip-api 的 geo 字符串形如 `"<国家名> - <组织名>"`，判定只要前半段（契约 §2.5 步骤 2）。 */
export function ecsCountryOf(ecsGeo: string | null): EcsCountry {
  // `noEcs` 只留给**真的没有 ECS 段**的情形：呈现层据这个 reason 写「你的 DNS 服务商
  // 不发送 ECS」，而有 ECS 段却切不出国家名（如 `" - Some ISP"`）时那句话是假的。
  if (ecsGeo === null || ecsGeo.trim() === "") {
    return { known: false, reason: "noEcs" };
  }

  const name = ecsGeo.split(" - ")[0].trim();
  if (name === "") return { known: false, reason: "unmappedCountry" };

  const iso2 = ISO2_BY_COUNTRY_NAME.get(name);
  // 查不到一律视为未知（契约 §2.5 硬约束 3）：把「我不认识这个国家名」当成「两国不同」
  // 会凭空造出误报，而误报比漏报更快毁掉用户对本产品的信任。
  return iso2 === undefined
    ? { known: false, reason: "unmappedCountry" }
    : { known: true, iso2 };
}

/**
 * 判定表本体（契约 §2.5）。两侧都已是 ISO2——golden 向量正是在这一层给输入。
 *
 * 比的是国家而不是 ECS 的 IP 前缀：掩码位数由 resolver 决定且响应里不一定给出，
 * 比前缀是掩码敏感的，比国家不是（§2.5 硬约束 2）。
 */
export function compareDnsEgress(
  ecs: EcsCountry,
  exitCountry: string | null,
): DnsEgressComparison {
  if (!ecs.known) return { comparable: false, reason: ecs.reason };

  const exit = (exitCountry ?? "").trim().toUpperCase();
  if (exit === "") return { comparable: false, reason: "unknownExitCountry" };

  return {
    comparable: true,
    leak: ecs.iso2 !== exit,
    ecsCountry: ecs.iso2,
    exitCountry: exit,
  };
}

/**
 * 探测结果 + O1 的出口国 → O5 的检测项状态。
 *
 * **「无从比对」记「已完成」而不是「检测失败」**：探测确实成功了，只是回答里不含可判定的
 * 信息。记成失败会诱导用户反复刷新一个永远不会变的结果（契约 §2.5）。
 */
export function judgeDnsEgress(
  probe: DnsEgressProbe,
  exitCountry: string | null,
): OnlineCheck<DnsEgressResult> {
  if (!probe.ok) {
    return { status: "failed", reason: DNS_EGRESS_PROBE_FAILED };
  }

  return {
    status: "done",
    data: {
      resolverGeo: probe.resolverGeo,
      comparison: compareDnsEgress(ecsCountryOf(probe.ecsGeo), exitCountry),
    },
  };
}
