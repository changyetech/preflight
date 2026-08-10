// O3 IPv6 泄露判定（规格 2.3 判定表，ADR-0003）。

import type { OnlineCheck } from "./checks";
import type { Ipv6Result } from "./types";

/** 单个 ipify 端点的探测结果。不可达一律不带 ip——CORS 失败与网络失败在浏览器里不可区分。 */
export type Probe = { reachable: true; ip: string } | { reachable: false };

/**
 * 判定表（规格 2.3）：
 *
 * | v4 | v6 | 判定 |
 * |----|----|------|
 * | 通  | 通  | IPv6 泄露（中风险） |
 * | 通  | 不通 | IPv6 未启用（无风险） |
 * | 不通 | 不通 | 第三方故障 → 检测失败 |
 * | 不通 | 通  | 异常，按第三方故障处理 → 检测失败 |
 *
 * v4 对照是必需的：没有它就无法区分「用户没有 IPv6」与「ipify 挂了」，
 * 而把后者判成前者，等于把没测出来的结果说成测过了且安全。
 */
export function judgeIpv6(v4: Probe, v6: Probe): OnlineCheck<Ipv6Result> {
  if (!v4.reachable) {
    // 这个 reason 不面向用户展示——O3Card 渲染失败态时用的是 copy.checks.O3.failed
    // （cards.tsx），不读这个字段。这里只需要一个非空、非语言相关的内部诊断标识，
    // 不走 i18n（m1：之前这里硬编码了中文，属于绕过 i18n 的死文案）。
    return { status: "failed", reason: "ipv6-probe-v4-unreachable" };
  }

  return v6.reachable
    ? { status: "done", data: { leak: true, ipv6: v6.ip } }
    : { status: "done", data: { leak: false, ipv6: null } };
}
