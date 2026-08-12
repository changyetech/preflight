// العربية（阿拉伯语）文案 —— 待补译。
//
// 现状：尚未翻译，全部字段逐条回落到英文源（见 src/copy.ts 的 getCopy 合并逻辑）。
// 补译方式：从 en.ts 抄对应字段填进来即可，不必一次填完——填了的生效，没填的仍回落英文。
//
// 阿拉伯语是本站唯一的 RTL 语种：页面方向由 LOCALES 表的 dir 字段驱动（见 src/copy.ts），
// 与译文进度无关——即使这里还全是英文回落，/ar 页面也已经是 dir="rtl" 的镜像布局。
// 译文里如需混排拉丁文本（IP 地址、pip install ai-ipcheck、域名），浏览器的双向算法会自动处理，
// 不要手工插入 U+200E/U+200F 方向标记。

import type { PartialCopy } from "./en";

export const AR: PartialCopy = {};
