// Русский（俄语）文案 —— 待补译。
//
// 现状：尚未翻译，全部字段逐条回落到英文源（见 src/copy.ts 的 getCopy 合并逻辑）。
// 补译方式：从 en.ts 抄对应字段填进来即可，不必一次填完——填了的生效，没填的仍回落英文。
// 术语建议对齐 CLI 的 README_EN.md：Exit IP / Overall Verdict / Coverage 三个词全站要一致。

import type { PartialCopy } from "./en";

export const RU: PartialCopy = {};
