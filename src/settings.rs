use brcore::ReadingSettings;
use oxichrome::storage;

/// 扩展本地存储中保存阅读设置的键。
pub const SETTINGS_KEY: &str = "settings";

/// 读取已保存的阅读设置。
///
/// 读取失败或没有记录时返回默认值。
pub async fn load() -> ReadingSettings {
    storage::get(SETTINGS_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// 保存阅读设置。
///
/// # Errors
///
/// 存储 API 失败时返回错误文本。
pub async fn save(settings: &ReadingSettings) -> Result<(), String> {
    storage::set(SETTINGS_KEY, settings)
        .await
        .map_err(|error| error.to_string())
}
