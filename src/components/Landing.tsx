// 检测面板之下的落地内容（规格第 4 节第 3 项 / --content 计划步骤 1-3，按原型 why/install/compare 三段重排）：
// 「为什么需要」「安装 CLI」「Web 与 CLI 完整功能对照表」。

import { CopyButton } from "./Card";
import { COMPARE_TABLE } from "../domain/compareTable";
import { useCopy } from "../i18n";

/**
 * 「为什么需要体检」四栏枚举与检测项的对应关系（原型 why-item-ip-risk/timezone/ipv6/local-dns）。
 * 只是排列顺序，不是判级逻辑，因此放在组件里而非 src/domain/（本任务不碰判级契约的目录）。
 */
const WHY_ITEMS = [
  { checkId: "O4", cli: false },
  { checkId: "O2", cli: false },
  { checkId: "O3", cli: false },
  { checkId: "C2", cli: true },
] as const;

export function Landing() {
  const COPY = useCopy();
  const { why, install, compare } = COPY.landing;

  return (
    <section className="landing">
      <article className="landing-section" id="why">
        <h2>{why.title}</h2>
        <p className="why-lede">{why.lede}</p>

        <ol className="why-grid">
          {WHY_ITEMS.map((item, index) => (
            <li key={item.checkId}>
              <span className="why-n">
                {String(index + 1).padStart(2, "0")}
              </span>
              <h3>{why.items[index].title}</h3>
              <p>{why.items[index].body}</p>
              <span className="why-tag" data-cli={item.cli}>
                <i />
                {item.checkId} ·{" "}
                {item.cli ? COPY.coverage.needCli : why.checkedOnlineTag}
              </span>
            </li>
          ))}
        </ol>

        <p className="why-foot">{why.foot}</p>
      </article>

      <article className="landing-section" id="install">
        <div className="install-grid">
          <div className="install-copy">
            <h2>{install.title}</h2>
            <p>{install.body}</p>
          </div>

          <div className="install-panel">
            <div className="install-row">
              <code>{COPY.actions.installCommand}</code>
              <CopyButton text={COPY.actions.installCommand} />
            </div>
            <div className="install-row">
              <code>{COPY.actions.installCommandWindows}</code>
              <CopyButton text={COPY.actions.installCommandWindows} />
            </div>
            <div className="install-meta">
              <span className="plat">{install.platforms}</span>
            </div>
          </div>
        </div>
      </article>

      <article className="landing-section" id="compare">
        <h2>{compare.title}</h2>
        <div className="compare-table-wrap">
          <table className="compare-table">
            <thead>
              <tr>
                <th scope="col">{compare.columnId}</th>
                <th scope="col">{compare.columnItem}</th>
                <th scope="col">{compare.columnWeb}</th>
                <th scope="col">{compare.columnCli}</th>
              </tr>
            </thead>
            <tbody>
              {COMPARE_TABLE.map((row) => {
                const webOn = row.owner === "web";
                const webValue = webOn
                  ? row.execution === "auto"
                    ? compare.auto
                    : compare.onDemand
                  : compare.dash;

                return (
                  <tr key={row.id}>
                    <td className="c-id">{row.id}</td>
                    <td className="c-item">{COPY.checks[row.id].title}</td>
                    {/* Web 列的「自动/按需」是信息标注，不是二元能力确认，不套绿色 .mark.on
                        （原型 refs/ipcheck-web-redesign.html:1072-1081：.mark.on 只用在 CLI
                        列的「支持」上；Web 列只有裸 .mark 与「—」的 .mark.dash 两种）。 */}
                    <td className="c-mark" data-label={compare.columnWeb}>
                      <span className={webOn ? "mark" : "mark dash"}>
                        {webValue}
                      </span>
                    </td>
                    {/* CLI 覆盖全部 10 项（CLI README.md 功能表），这一列恒为「有」——
                        C1 的教训：之前把「归属＝Web 能否在线测」误当成「哪边有这功能」，
                        导致 O1-O4 在 CLI 列显示「—」，与 landing.install 里「CLI 覆盖全部 10 项」自相矛盾。 */}
                    <td className="c-mark" data-label={compare.columnCli}>
                      <span className="mark on">{compare.available}</span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </article>
    </section>
  );
}
