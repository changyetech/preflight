// 繁體中文文案 —— 待补译。
//
// 现状：尚未翻译，全部字段逐条回落到英文源（见 src/copy.ts 的 getCopy 合并逻辑）。
// 补译方式：从 en.ts / zh-hans.ts 抄对应字段填进来即可，不必一次填完——填了的生效，没填的仍回落英文。
// 注意繁體不是简体的字形转换：术语要按台港习惯（如「網路」而非「网络」、「軟體」而非「软件」），
// 直接跑简繁转换会产出「网路」这类混种词，比留英文更糟。

import type { PartialCopy } from "./en";

export const ZH_HANT: PartialCopy = {};
