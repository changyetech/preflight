// O5 DNS 出口泄露：判定层（契约 §2.5 判定表）与探测层的取消路径。
//
// 这张表最容易写错的是三种「无从比对」：没有 ECS 段、国家名查不到表、出口国未知。
// 三者都不得回退成「两国不同」——那会把「我不知道」包装成一次泄露告警（契约 §2.5 硬约束 3）。
// 判定表第 4 行（国家名映射不出 ISO2）在 golden 向量层不可测：向量给的输入已经是 ISO2，
// 映射发生在它之前，所以这一行只能由本文件钉住。

import { afterEach, describe, expect, it, vi } from "vitest";

import {
  compareDnsEgress,
  DNS_EGRESS_PROBE_FAILED,
  ecsCountryOf,
  judgeDnsEgress,
} from "../src/domain/dnsEgress";
import { probeDnsEgress } from "../src/probes/dnsEgress";

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

  it("有 ECS 段但切不出国家名 → unmappedCountry，不是 noEcs", () => {
    // 判级上两者等价，但呈现层据 reason 选文案：报 noEcs 会让用户读到
    // 「你的 DNS 服务商不发送 ECS」——这里明明发了。
    expect(ecsCountryOf(" - Some ISP")).toEqual({
      known: false,
      reason: "unmappedCountry",
    });
  });

  it("空串与纯空白等同于没有 ECS 段", () => {
    expect(ecsCountryOf("   ")).toEqual({ known: false, reason: "noEcs" });
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

describe("probeDnsEgress · 取消路径", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("子域标签恒为 32 位十六进制，且每次重试都换新的", async () => {
    // 32 是 edns.ip-api.com 的外部契约（实测 31／33 位一律 404），此前发的是 16 位，
    // 于是三次请求全部 404 ⇒ O5 在生产环境永远落「检测失败」。两轮评审都没抓到，
    // 正是因为没有任何断言约束过标签的形状——这条测试补的就是那个缺口。
    // 换新前缀则是绕 DNS 缓存的前提：重试同一个前缀等于在打自己刚种下的缓存。
    const urls: string[] = [];
    vi.spyOn(globalThis, "fetch").mockImplementation((input) => {
      urls.push(String(input));
      return Promise.resolve(new Response("Not Found", { status: 404 }));
    });

    await expect(probeDnsEgress()).resolves.toEqual({ ok: false });

    expect(urls).toHaveLength(3);
    for (const url of urls) {
      expect(url).toMatch(/^https:\/\/[0-9a-f]{32}\.edns\.ip-api\.com\/json$/);
    }
    expect(new Set(urls).size).toBe(3);
  });

  it("HTTP 200 但形状不对 ⇒ 探测失败，绝不当成「无 ECS 段」", async () => {
    // ip-api 用 200 返回 JSON 错误体是它文档化的行为。当成「无 ECS 段」的话，卡片会
    // 打出一句**关于用户 DNS 服务商的假陈述**，把他引向一个不存在的原因，而真实原因
    // （第三方挂了、刷新可能就好）被完全遮蔽。CLI 侧是同一条判断，此前 Web 没有。
    for (const body of [
      {},
      { status: "fail", message: "quota exceeded" },
      { edns: { geo: "Japan - IT7 Networks Inc" } }, // 只有 edns、没有 dns
    ]) {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify(body), {
          headers: { "Content-Type": "application/json" },
        }),
      );

      await expect(probeDnsEgress()).resolves.toEqual({ ok: false });
      vi.restoreAllMocks();
    }
  });

  it("dns 段存在时照常返回观测，edns 缺失是正常输入而非失败", async () => {
    // 这一格必须与上一格分得开：不发 ECS 的服务商（Cloudflare 1.1.1.1）是典型用户，
    // 他们该看到「已完成 · 无从比对」，不是「检测失败」。
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ dns: { geo: "Japan - Google LLC" } }), {
        headers: { "Content-Type": "application/json" },
      }),
    );

    await expect(probeDnsEgress()).resolves.toEqual({
      ok: true,
      ecsGeo: null,
      resolverGeo: "Japan - Google LLC",
    });
  });

  it("已中止的 signal（组件已卸载）不再发请求，直接按探测失败收场", async () => {
    // 没有这条，卸载后最长 15s（3 次 × 5s 重试）还在打第三方——与 O6 的处理不对称。
    const fetchSpy = vi.spyOn(globalThis, "fetch");

    await expect(probeDnsEgress(AbortSignal.abort())).resolves.toEqual({
      ok: false,
    });
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});
