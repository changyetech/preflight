// StopForumSpam 滥用收录（规格 2.4）。无需 API key，无配额守卫。

const API_URL = "https://api.stopforumspam.org/api";

/**
 * 返回三态：true 有收录 / false 无收录 / null 服务不可用。
 *
 * 不可用必须是 null 而不是 false：滥用收录会把综合结论拉到「中」（规格 3.1），
 * 把「查不到」谎报成「无收录」等于把有风险的人判成安全。
 */
export async function fetchAbuseListed(ip: string): Promise<boolean | null> {
  const url = new URL(API_URL);
  url.searchParams.set("ip", ip);
  url.searchParams.set("json", "");

  try {
    const response = await fetch(url.toString());
    if (!response.ok) {
      return null;
    }

    const body = (await response.json()) as {
      success?: number;
      ip?: { appears?: number };
    };
    if (body.success !== 1) {
      return null;
    }

    return (body.ip?.appears ?? 0) > 0;
  } catch {
    return null;
  }
}
