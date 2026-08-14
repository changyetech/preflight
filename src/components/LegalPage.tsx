// /privacy/ 与 /terms/ 及其中文版的独立页面（spec docs/specs/2026-08-14-legal-pages.md）。
// 两个页面结构完全一致，只是文案不同，因此共用一个组件、按 doc 取对应文案。

import { type Lang } from "../copy";
import { Nav } from "./Nav";
import { useCopy } from "../i18n";

export type LegalDoc = "privacy" | "terms";

export function LegalPage({ lang, doc }: { lang: Lang; doc: LegalDoc }) {
  const COPY = useCopy();
  const page = COPY.legal[doc];

  return (
    <>
      <Nav lang={lang} pageSlug={`/${doc}/`} />

      <main className="legal-main">
        <h1>{page.heading}</h1>
        <p className="legal-lede">{page.lede}</p>

        {page.sections.map((section) => (
          <section key={section.heading} className="legal-section">
            <h2>{section.heading}</h2>
            <p>{section.body}</p>
          </section>
        ))}

        <p className="legal-updated">{COPY.legal.updated}</p>
      </main>
    </>
  );
}
