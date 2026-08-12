//! 繁體中文——尚未譯全。只寫已譯好的欄位，其餘逐欄位回落英文。
//!
//! 補譯的寫法：把要譯的欄位填上 `Some(...)`，剩下的交給 `..XxxPatch::DEFAULT`。

use super::{ConfigTextPatch, LangTextPatch, TextPatch};

pub const ZH_HANT: TextPatch = TextPatch {
    lang: LangTextPatch {
        partial_notice: Some("該語系尚未譯全，未翻譯的項目顯示英文。"),
    },

    config: ConfigTextPatch {
        path_label: Some("設定檔"),
        ..ConfigTextPatch::DEFAULT
    },

    ..TextPatch::DEFAULT
};
