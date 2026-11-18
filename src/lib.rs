//! BettRead 浏览器扩展。
//!
//! 用户按 `Alt+R` 或点击工具栏图标开启或停止阅读模式。

mod background;
mod content_script;
mod options;
mod page;
mod reader;
mod settings;

#[allow(missing_docs)]
mod extension {
    use oxichrome::prelude::*;

    /// BettRead 扩展入口。
    #[extension(
        name = "BettRead",
        description = "Highlight one sentence at a time and navigate with Vim-style keys.",
        version = "0.1.0",
        gecko_id = "betteread@example.org",
        permissions = [ActiveTab, Scripting, Storage],
        icons = { 16 = "icons/16.png", 32 = "icons/32.png", 48 = "icons/48.png", 128 = "icons/128.png" },
        action = { default_title = "Toggle reading mode" }
    )]
    pub struct Betteread;
}

pub use extension::Betteread;
