// StopForumSpam 滥用收录（规格 2.4）。三态：有收录 / 无收录 / 服务不可用。
import { afterEach, describe, expect, it, vi } from "vitest";

import { fetchAbuseListed } from "../worker/stopforumspam";

function stubFetch(reply: unknown | Error) {
  const fetchMock = vi.fn(async (_url: string) => {
    if (reply instanceof Error) throw reply;
    if (reply instanceof Response) return reply;
    return Response.json(reply);
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("fetchAbuseListed", () => {
  it("有滥用收录时为 true", async () => {
    stubFetch({
      success: 1,
      ip: {
        value: "1.2.3.4",
        appears: 1,
        frequency: 27,
        lastseen: "2026-07-01 10:00:00",
        confidence: 92.1,
      },
    });

    await expect(fetchAbuseListed("1.2.3.4")).resolves.toBe(true);
  });

  it("无滥用收录时为 false", async () => {
    stubFetch({
      success: 1,
      ip: { value: "1.2.3.4", appears: 0, frequency: 0 },
    });

    await expect(fetchAbuseListed("1.2.3.4")).resolves.toBe(false);
  });

  it("按契约以 json 形式查询被查 IP", async () => {
    const fetchMock = stubFetch({ success: 1, ip: { appears: 0 } });

    await fetchAbuseListed("203.0.113.7");

    const url = new URL(fetchMock.mock.calls[0][0]);
    expect(url.origin).toBe("https://api.stopforumspam.org");
    expect(url.searchParams.get("ip")).toBe("203.0.113.7");
    expect(url.searchParams.has("json")).toBe(true);
  });

  it("服务不可用时为 null，而不是谎报「无收录」", async () => {
    stubFetch(new Response("down", { status: 503 }));
    await expect(fetchAbuseListed("1.2.3.4")).resolves.toBeNull();

    stubFetch(new Error("network down"));
    await expect(fetchAbuseListed("1.2.3.4")).resolves.toBeNull();

    // success != 1：接口自己声明这次查询没成功
    stubFetch({ success: 0, error: "invalid ip" });
    await expect(fetchAbuseListed("1.2.3.4")).resolves.toBeNull();
  });
});
