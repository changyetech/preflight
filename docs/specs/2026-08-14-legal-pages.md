# 隐私说明与使用条款页面

日期：2026-08-14
状态：已实现

## 1. 目标

站点缺少隐私说明与使用条款两个常规页面。补上它们，入口放在首页页脚右下角（与既有的「公共 DNS 清单」链接并列）。

## 2. 决策

1. **两个独立页面，不是弹窗、不是首页锚点**：`/privacy/`、`/terms/` 与各自的中文版 `/zh-hans/privacy/`、`/zh-hans/terms/`。与 `/dns/` 完全同构——Vite 多页入口，各自一份 HTML（`<html lang>` / `<title>` / description / canonical / hreflang 各自正确），未知路径仍真 404，不做 SPA 回落。
2. **事实型简短说明，不用法务模板**：本站无账号、无数据库、不存储检测结果，标准 GDPR/CCPA 模板里大半条款（数据主体权利、留存期限）对本站根本不适用，照抄会写出与事实不符的条款。页面只陈述真实发生的数据处理。
3. **不写联系方式与运营主体**：页面不留邮箱、不写公司/个人署名。
4. **内容必须与代码事实一致**：第三方清单取自页脚既有披露（ipify / ip-api.com / stun.cloudflare.com / stun.l.google.com / Cloudflare Turnstile / proxycheck.io / StopForumSpam）；`p=0` / `tag=0` 见 `worker/proxycheck.ts`；localStorage 只有主题键 `preflight-theme`，语言由路径决定不持久化（ADR-0008）。**页面是这些事实的镜像——行为变了，本页必须同步改。**
5. **两语种均为完整译文**，无字段级回落，与 `src/copy.ts` 既有约定一致。
6. 页面本身只有 `Nav` + 正文，不带页脚（与 `/dns/` 一致）。

## 3. 内容大纲

**隐私说明**：不存储什么 / 浏览器直连的第三方 / Cookie 与本地存储 / 发给 proxycheck.io 的请求带 `p=0`+`tag=0` / CLI 直连第三方不经本站 / 托管在 Cloudflare。

**使用条款**：按现状提供、无担保 / 结论是信号不是保证 / 可接受使用与限流 / 第三方各自条款 / 可用性无承诺 / 责任限制。

## 4. 验收

- `/privacy/`、`/terms/`、`/zh-hans/privacy/`、`/zh-hans/terms/` 各返回 200 且是对应语种的真实 HTML。
- `/privacy/xxx` 一类路径仍是真 404。
- 首页页脚右下角出现三个链接：公共 DNS 清单、隐私说明、使用条款。
- `sitemap.xml` 收录四条新 URL。
