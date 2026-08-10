// 检测面板之下的落地内容（规格第 4 节第 3 项 / --content 计划步骤 1-3）：
// 「为什么需要」「安装 CLI」「Web 与 CLI 完整功能对照表」。

import { CopyButton } from "./Card";
import { COMPARE_TABLE } from "../domain/compareTable";
import { useCopy } from "../i18n";

export function Landing() {
  const COPY = useCopy();
  const { why, install, compare } = COPY.landing;

  return (
    <section className="landing">
      <article className="landing-section">
        <h2>{why.title}</h2>
        <p>{why.body}</p>
      </article>

      <article className="landing-section">
        <h2>{install.title}</h2>
        <p>{install.body}</p>
        <p className="install">
          <code>{COPY.actions.installCommand}</code>
          <CopyButton text={COPY.actions.installCommand} />
        </p>
      </article>

      <article className="landing-section">
        <h2>{compare.title}</h2>
        <div className="compare-table-wrap">
          <table className="compare-table">
            <thead>
              <tr>
                <th>{compare.columnId}</th>
                <th>{compare.columnItem}</th>
                <th>{compare.columnWeb}</th>
                <th>{compare.columnCli}</th>
              </tr>
            </thead>
            <tbody>
              {COMPARE_TABLE.map((row) => (
                <tr key={row.id}>
                  <td>{row.id}</td>
                  <td>{COPY.checks[row.id].title}</td>
                  <td>
                    {row.owner === "web"
                      ? row.execution === "auto"
                        ? compare.auto
                        : compare.onDemand
                      : compare.dash}
                  </td>
                  <td>
                    {row.owner === "cli" ? compare.cliOnly : compare.dash}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </article>
    </section>
  );
}
