// 检测面板之下的落地内容（规格第 4 节第 3 项 / --content 计划步骤 1-3，按原型 why/install/compare 三段重排）：
// 「为什么需要」「安装 CLI」「Web 与 CLI 完整功能对照表」。

import { CopyButton } from "./Card";
import { COMPARE_TABLE } from "../domain/compareTable";
import { pageUrl, type Lang } from "../copy";
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

export function Landing({ lang }: { lang: Lang }) {
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
              {/* 手册入口跟着安装场景走（spec docs/specs/2026-08-14-cli-guide-page.md 决策 3）：
                  刚复制完安装命令的用户最需要手册。新标签打开 + noopener，与顶栏跨页链接同一约定。 */}
              <a
                className="install-guide-link"
                href={pageUrl(lang, "/guide/")}
                target="_blank"
                rel="noopener"
              >
                {install.guideLink}
              </a>
            </div>
          </div>
        </div>

        {/* 实物截图放在安装命令之后：这一段论证的是「CLI 覆盖全部 10 项」，截图末尾的
            C1–C4 就是证据，而命令是本段的行动点，得紧跟正文不被图挤走。
            两张是同一次输出的上下半，上下排（原始宽 787/800px，缩到分栏就看不清字了）。 */}
        <figure className="cli-shot">
          <img
            src="/screenshot_cli_check1.png"
            alt={install.shotAlt1}
            width={787}
            height={1002}
            loading="lazy"
            decoding="async"
          />
          <img
            src="/screenshot_cli_check2.png"
            alt={install.shotAlt2}
            width={800}
            height={669}
            loading="lazy"
            decoding="async"
          />
          <figcaption>
            <code>preflight --verbose</code> — {install.shotCaption}
          </figcaption>
        </figure>
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
