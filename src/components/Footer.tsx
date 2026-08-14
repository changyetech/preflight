// 页脚双栏披露（原型 .footer，规格 §4 要点 5）：披露内容以现网 src/locales/*.ts 文案为准，
// 按「自动发起／需手动触发」两栏排布，原文一字未改，只是从一段长文拆成结构化两栏。
//
// 硬性红线：原型页脚里的「本页为设计稿」声明是关于稿件本身的元信息，不是产品内容，
// 绝不落地——落地就是把设计稿声明发到生产站上。

import { pageUrl, type Lang } from "../copy";
import { useCopy } from "../i18n";

// compact：只保留版权行（版权 + 法务链接）。披露两栏讲的是首页检测行为
// （自动发起/需手动触发），dns/guide 这类静态内容页不发起检测，披露放上去是错误陈述。
export function Footer({
  lang,
  compact = false,
}: {
  lang: Lang;
  compact?: boolean;
}) {
  const COPY = useCopy();
  const { footer } = COPY;

  return (
    <footer className="site-footer">
      {!compact && (
        <>
          <p className="footer-lede">{footer.privacy}</p>
          <div className="footer-cols">
            <div>
              <span className="eyebrow">{footer.autoLabel}</span>
              <p>{footer.autoBody}</p>
            </div>
            <div>
              <span className="eyebrow">{footer.onDemandLabel}</span>
              <p>{footer.onDemandBody}</p>
            </div>
          </div>
        </>
      )}
      {/* 版权行不进 locales：符号、年份、公司名均无语种差异；年份取运行时当前年。
          公司名指向官网，外站一律新标签 + rel="noopener noreferrer"。 */}
      <p className="footer-copyright">
        <span>
          © {new Date().getFullYear()}{" "}
          <a
            className="footer-company"
            href="https://changyetech.com"
            target="_blank"
            rel="noopener noreferrer"
          >
            Hangzhou Changye Network Technology Co., Ltd.
          </a>
        </span>
        {/* 法务两页的入口收在右下角。这里只放法务页：DNS 清单是内容资源不是法务页，
            入口在顶栏（Nav 的 .nav-dns），别再往这一组里加。
            路径一律走 pageUrl 拼语种前缀，中文首页的页脚不能把人送去英文子页。
            一律新标签打开：页脚是查阅入口，不该打断用户正在页面上进行的检测。
            带 rel="noopener"——即便同源也照挂，避免新页面拿到 window.opener。 */}
        <span className="footer-links">
          <a href={pageUrl(lang, "/privacy/")} target="_blank" rel="noopener">
            {COPY.legal.privacyLink}
          </a>
          <a href={pageUrl(lang, "/terms/")} target="_blank" rel="noopener">
            {COPY.legal.termsLink}
          </a>
        </span>
      </p>
    </footer>
  );
}
