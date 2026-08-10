// Turnstile 前端组件。仅作用于 /api/risk（规格第 5 节）。
//
// 脚本与组件都推迟到用户点击 O4 按钮时才加载：未点击就不该有任何第三方请求发生（ADR-0008）。

type TurnstileApi = {
  render: (
    container: HTMLElement,
    options: {
      sitekey: string;
      callback: (token: string) => void;
      "error-callback": () => void;
      "expired-callback": () => void;
      appearance?: "always" | "execute" | "interaction-only";
    },
  ) => string;
  remove: (widgetId: string) => void;
};

declare global {
  interface Window {
    turnstile?: TurnstileApi;
  }
}

const SCRIPT_SRC =
  "https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit";

const SITE_KEY = import.meta.env.VITE_TURNSTILE_SITE_KEY as string | undefined;

/** 未配置 site key 时 O4 不可用——与其发一个注定被 2010 拒掉的请求，不如直接说清楚。 */
export const turnstileConfigured = Boolean(SITE_KEY);

let loading: Promise<TurnstileApi> | null = null;

function loadTurnstile(): Promise<TurnstileApi> {
  if (window.turnstile) return Promise.resolve(window.turnstile);

  loading ??= new Promise<TurnstileApi>((resolve, reject) => {
    const script = document.createElement("script");
    script.src = SCRIPT_SRC;
    script.async = true;
    script.onload = () =>
      window.turnstile
        ? resolve(window.turnstile)
        : reject(new Error("turnstile unavailable"));
    script.onerror = () => reject(new Error("turnstile unavailable"));
    document.head.appendChild(script);
  });

  return loading;
}

/** 取一枚一次性 token。组件渲染在传入的容器里，token 到手即销毁。 */
export async function requestTurnstileToken(
  container: HTMLElement,
): Promise<string> {
  if (!SITE_KEY) throw new Error("turnstile not configured");

  const turnstile = await loadTurnstile();

  return new Promise<string>((resolve, reject) => {
    const widgetId = turnstile.render(container, {
      sitekey: SITE_KEY,
      appearance: "interaction-only",
      callback: (token) => {
        turnstile.remove(widgetId);
        resolve(token);
      },
      "error-callback": () => {
        turnstile.remove(widgetId);
        reject(new Error("turnstile failed"));
      },
      "expired-callback": () => {
        turnstile.remove(widgetId);
        reject(new Error("turnstile expired"));
      },
    });
  });
}
