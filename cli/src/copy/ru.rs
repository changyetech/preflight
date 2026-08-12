//! Русский — переведено частично. Непереведённые поля берутся из английского.

use super::{ConfigTextPatch, LangTextPatch, TextPatch};

pub const RU: TextPatch = TextPatch {
    lang: LangTextPatch {
        partial_notice: Some(
            "Этот язык переведён частично; непереведённые элементы показаны на английском.",
        ),
    },

    config: ConfigTextPatch {
        path_label: Some("Файл конфигурации"),
        ..ConfigTextPatch::DEFAULT
    },

    ..TextPatch::DEFAULT
};
