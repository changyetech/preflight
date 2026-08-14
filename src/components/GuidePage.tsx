// /guide/ 与 /zh-hans/guide/ 的独立页面（spec docs/specs/2026-08-14-cli-guide-page.md）。
// 底稿是 docs/cli-guide.md：只讲「怎么用」，判级规则不复述（契约红线）。

import { type Lang } from "../copy";
import { Nav } from "./Nav";
import { useCopy } from "../i18n";

export function GuidePage({ lang }: { lang: Lang }) {
  const COPY = useCopy();
  const guide = COPY.guide;

  return (
    <>
      <Nav lang={lang} pageSlug="/guide/" />

      <main className="guide-main">
        <h1>{guide.heading}</h1>
        <p className="guide-lede">{guide.lede}</p>

        {/* 安装段特殊化：两条命令引用 COPY.actions 的同一份字面量，
            不在手册文案里存第二份（改安装命令只改一处，spec 验收项）。 */}
        <section className="guide-section">
          <h2>{guide.install.heading}</h2>
          <p>{guide.install.intro}</p>
          <pre className="guide-code">
            <code>{`# ${guide.install.linuxLabel}\n${COPY.actions.installCommand}\n\n# ${guide.install.windowsLabel}\n${COPY.actions.installCommandWindows}`}</code>
          </pre>
          <p>{guide.install.verify}</p>
          <pre className="guide-code">
            <code>preflight --version</code>
          </pre>
        </section>

        {guide.sections.map((section) => (
          <section key={section.heading} className="guide-section">
            <h2>{section.heading}</h2>
            {section.paras.map((para) => (
              <p key={para}>{para}</p>
            ))}
            {section.code.length > 0 && (
              <pre className="guide-code">
                <code>{section.code.join("\n")}</code>
              </pre>
            )}
            {section.table.headers.length > 0 && (
              <div className="guide-table-wrap">
                <table className="guide-table">
                  <thead>
                    <tr>
                      {section.table.headers.map((header) => (
                        <th key={header}>{header}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {section.table.rows.map((row) => (
                      <tr key={row[0]}>
                        {row.map((cell) => (
                          <td key={cell}>{cell}</td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            {section.after.map((para) => (
              <p key={para}>{para}</p>
            ))}
          </section>
        ))}

        <section className="guide-section">
          <h2>{guide.scenarios.heading}</h2>
          {guide.scenarios.items.map((item) => (
            <div key={item.title} className="guide-scenario">
              <h3>{item.title}</h3>
              <p>{item.body}</p>
              {item.code.length > 0 && (
                <pre className="guide-code">
                  <code>{item.code.join("\n")}</code>
                </pre>
              )}
            </div>
          ))}
        </section>
      </main>
    </>
  );
}
