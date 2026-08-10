// Turnstile 服务端校验，只作用于 /api/risk（规格 5 节）。
import { afterEach, describe, expect, it, vi } from "vitest";

import { SITEVERIFY_URL, verifyTurnstile } from "../worker/turnstile";

function stubFetch(reply: Response | Error) {
  const fetchMock = vi.fn(async () => {
    if (reply instanceof Error) throw reply;
    return reply;
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("verifyTurnstile", () => {
  it("无 token 时直接拒绝，且不调用 siteverify", async () => {
    const fetchMock = stubFetch(Response.json({ success: true }));

    await expect(verifyTurnstile(null, "secret", "1.2.3.4")).resolves.toBe(
      false,
    );
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("空字符串 token 同样直接拒绝", async () => {
    const fetchMock = stubFetch(Response.json({ success: true }));

    await expect(verifyTurnstile("", "secret", "1.2.3.4")).resolves.toBe(false);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("siteverify 判定无效时拒绝", async () => {
    stubFetch(
      Response.json({
        success: false,
        "error-codes": ["invalid-input-response"],
      }),
    );

    await expect(verifyTurnstile("bad", "secret", "1.2.3.4")).resolves.toBe(
      false,
    );
  });

  it("siteverify 判定有效时放行，并按契约提交 secret / token / remoteip", async () => {
    const fetchMock = stubFetch(Response.json({ success: true }));

    await expect(verifyTurnstile("good", "secret", "1.2.3.4")).resolves.toBe(
      true,
    );

    const [url, init] = fetchMock.mock.calls[0] as unknown as [
      string,
      RequestInit,
    ];
    expect(url).toBe(SITEVERIFY_URL);
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({
      secret: "secret",
      response: "good",
      remoteip: "1.2.3.4",
    });
  });

  it("siteverify 不可用时按拒绝处理（fail closed）", async () => {
    stubFetch(new Response("boom", { status: 500 }));
    await expect(verifyTurnstile("good", "secret", "1.2.3.4")).resolves.toBe(
      false,
    );

    stubFetch(new Error("network down"));
    await expect(verifyTurnstile("good", "secret", "1.2.3.4")).resolves.toBe(
      false,
    );
  });
});
