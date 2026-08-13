// 页脚双栏披露（原型 .footer，规格 §4 要点 5）：披露内容以现网 src/locales/*.ts 文案为准，
// 按「自动发起／需手动触发」两栏排布，原文一字未改，只是从一段长文拆成结构化两栏。
//
// 硬性红线：原型页脚里的「本页为设计稿」声明是关于稿件本身的元信息，不是产品内容，
// 绝不落地——落地就是把设计稿声明发到生产站上。

import { useCopy } from "../i18n";

export function Footer() {
  const COPY = useCopy();
  const { footer } = COPY;

  return (
    <footer className="site-footer">
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
    </footer>
  );
}
