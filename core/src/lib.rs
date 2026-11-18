//! BettRead 的纯阅读模型。
//!
//! 该 crate 不依赖浏览器 API。句子偏移统一使用 UTF-16 码元。

mod layout;
mod navigation;
mod sentence;
mod settings;

pub use layout::{BlockLayout, TextBlock};
pub use navigation::{Motion, Navigation};
pub use sentence::{SentenceRange, split_sentences};
pub use settings::{
    HIGHLIGHT_NAME, HighlightColor, HighlightStyle, ReadingSettings, highlight_css,
};
