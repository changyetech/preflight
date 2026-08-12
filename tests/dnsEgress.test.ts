// O5 DNS 出口泄露判定（契约 §2.5 判定表）。
//
// 这张表最容易写错的是三种「无从比对」：没有 ECS 段、国家名查不到表、出口国未知。
// 三者都不得回退成「两国不同」——那会把「我不知道」包装成一次泄露告警（契约 §2.5 硬约束 3）。
// 判定表第 4 行（国家名映射不出 ISO2）在 golden 向量层不可测：向量给的输入已经是 ISO2，
// 映射发生在它之前，所以这一行只能由本文件钉住。

import { describe, expect, it } from "vitest";

import {
  compareDnsEgress,
  DNS_EGRESS_PROBE_FAILED,
  ecsCountryOf,
  judgeDnsEgress,
} from "../src/domain/dnsEgress";

/** ip-api 的 `edns.geo` 实测形状：`"<国家名> - <组织名>"`。 */
const ECS_JAPAN = "Japan - IT7 Networks Inc";
const RESOLVER_GEO = "Japan - Google LLC";

function ok(ecsGeo: string | null) {
  return { ok: true as const, ecsGeo, resolverGeo: RESOLVER_GEO };
}

describe("ecsCountryOf", () => {
  it("取 “ - ” 之前的国家名并查表得 ISO2", () => {
    expect(ecsCountryOf(ECS_JAPAN)).toEqual({ known: true, iso2: "JP" });
  });

  it("没有组织名后缀时整串就是国家名", () => {
    expect(ecsCountryOf("Japan")).toEqual({ known: true, iso2: "JP" });
  });

  it("响应里没有 ECS 段 → 未知，理由是「不发 ECS」", () => {
    expect(ecsCountryOf(null)).toEqual({ known: false, reason: "noEcs" });
  });

  it("国家名查不到表 → 未知，且与「不发 ECS」是两个理由（契约 §2.5 判定表第 4 行）", () => {
    expect(ecsCountryOf("Republic of Nowhere - Some ISP")).toEqual({
      known: false,
      reason: "unmappedCountry",
    });
  });
});

describe("compareDnsEgress", () => {
  const JP = { known: true as const, iso2: "JP" };

  it("ECS 国 ≠ 出口国 → 命中，且把两个国家都带出来（契约 §5.4 呈现约束）", () => {
    expect(compareDnsEgress(JP, "US")).toEqual({
      comparable: true,
      leak: true,
      ecsCountry: "JP",
      exitCountry: "US",
    });
  });

  it("ECS 国 = 出口国 → 未命中", () => {
    expect(compareDnsEgress(JP, "JP")).toMatchObject({
      comparable: true,
      leak: false,
    });
  });

  it("出口国大小写不同不算不一致（契约 §2.5 步骤 4：大写后比）", () => {
    expect(compareDnsEgress(JP, "jp")).toMatchObject({
      comparable: true,
      leak: false,
    });
  });

  it("ECS 未知 → 无从比对，绝不回退成「两国不同」", () => {
    for (const reason of ["noEcs", "unmappedCountry"] as const) {
      expect(compareDnsEgress({ known: false, reason }, "US")).toEqual({
        comparable: false,
        reason,
      });
    }
  });

  it("出口国未知（O1 未完成）→ 无从比对", () => {
    expect(compareDnsEgress(JP, null)).toEqual({
      comparable: false,
      reason: "unknownExitCountry",
    });
    expect(compareDnsEgress(JP, "")).toEqual({
      comparable: false,
      reason: "unknownExitCountry",
    });
  });
});

describe("judgeDnsEgress", () => {
  it("探测成功但无从比对仍记「已完成」，不是「检测失败」（契约 §2.5）", () => {
    // 记成失败会诱导用户反复刷新一个永远不会变的结果。
    const state = judgeDnsEgress(ok(null), "JP");

    expect(state.status).toBe("done");
    if (state.status !== "done") throw new Error("应为已完成");
    expect(state.data.comparison).toEqual({
      comparable: false,
      reason: "noEcs",
    });
  });

  it("探测本身失败 → 检测失败，不产出信号", () => {
    const state = judgeDnsEgress({ ok: false }, "JP");

    expect(state).toEqual({
      status: "failed",
      reason: DNS_EGRESS_PROBE_FAILED,
    });
  });

  it("resolver 归属只带出来展示，不参与判定（契约 §2.1／§2.5 硬约束 1）", () => {
    // 同一个 ECS 观测下换掉 resolver 归属，判定必须一字不变——
    // resolver 在哪取决于用户选了哪家 DNS 服务商，与流量走没走代理无关。
    const withGoogle = judgeDnsEgress(ok(ECS_JAPAN), "JP");
    const withAli = judgeDnsEgress(
      { ok: true, ecsGeo: ECS_JAPAN, resolverGeo: "China - Alibaba" },
      "JP",
    );

    if (withGoogle.status !== "done" || withAli.status !== "done") {
      throw new Error("应为已完成");
    }
    expect(withAli.data.comparison).toEqual(withGoogle.data.comparison);
    expect(withAli.data.resolverGeo).toBe("China - Alibaba");
  });
});
