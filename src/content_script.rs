use oxichrome::prelude::*;
use serde::{Deserialize, Serialize};

/// 阅读模式操作结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToggleState {
    /// 阅读模式是否处于活动状态。
    pub active: bool,
}

/// 开启或停止当前页面的阅读模式。
///
/// 返回操作后的活动状态。
#[content_script]
pub async fn toggle_reading() -> ToggleState {
    ToggleState {
        active: crate::reader::toggle().await,
    }
}
