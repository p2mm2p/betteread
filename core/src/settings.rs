use std::fmt;
use std::str::FromStr;

/// CSS Custom Highlight 中使用的注册名。
pub const HIGHLIGHT_NAME: &str = "betteread-active";
const DEFAULT_COLOR: &str = "#ffd54f";

/// 活动句子的高亮样式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum HighlightStyle {
    /// 使用下划线。
    Underline,
    /// 使用背景色块。
    Block,
}

impl Default for HighlightStyle {
    fn default() -> Self {
        Self::Underline
    }
}

impl HighlightStyle {
    /// 返回用于存储的稳定名称。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Underline => "underline",
            Self::Block => "block",
        }
    }
}

impl fmt::Display for HighlightStyle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for HighlightStyle {
    type Err = SettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "underline" => Ok(Self::Underline),
            "block" => Ok(Self::Block),
            _ => Err(SettingsError::InvalidStyle(value.to_string())),
        }
    }
}

/// 已校验的 `#RRGGBB` 颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightColor {
    red: u8,
    green: u8,
    blue: u8,
}

impl HighlightColor {
    /// 返回小写 `#RRGGBB` 字符串。
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }
}

impl Default for HighlightColor {
    fn default() -> Self {
        DEFAULT_COLOR.parse().expect("default color is valid")
    }
}

impl fmt::Display for HighlightColor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl FromStr for HighlightColor {
    type Err = SettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some(hex) = value.strip_prefix('#') else {
            return Err(SettingsError::InvalidColor(value.to_string()));
        };
        if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(SettingsError::InvalidColor(value.to_string()));
        }
        Ok(Self {
            red: u8::from_str_radix(&hex[0..2], 16).expect("validated hex"),
            green: u8::from_str_radix(&hex[2..4], 16).expect("validated hex"),
            blue: u8::from_str_radix(&hex[4..6], 16).expect("validated hex"),
        })
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for HighlightColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for HighlightColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// 用户可设置的阅读选项。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ReadingSettings {
    /// 活动句子的高亮样式。
    pub style: HighlightStyle,
    /// 活动句子的高亮颜色。
    pub color: HighlightColor,
}

impl Default for ReadingSettings {
    fn default() -> Self {
        Self {
            style: HighlightStyle::Underline,
            color: HighlightColor::default(),
        }
    }
}

/// 活动句子高亮的 CSS。
pub fn highlight_css(settings: &ReadingSettings) -> String {
    let color = settings.color.to_hex();
    match settings.style {
        HighlightStyle::Underline => format!(
            "::highlight({HIGHLIGHT_NAME}) {{\n  background-color: transparent;\n  color: inherit;\n  text-decoration-line: underline;\n  text-decoration-color: {color};\n  text-decoration-thickness: 3px;\n  text-underline-offset: 3px;\n}}\n"
        ),
        HighlightStyle::Block => format!(
            "::highlight({HIGHLIGHT_NAME}) {{\n  background-color: {color};\n  color: inherit;\n  text-decoration: none;\n}}\n"
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 阅读选项的校验错误。
pub enum SettingsError {
    /// 高亮样式名称无效。
    InvalidStyle(String),
    /// 高亮颜色格式无效。
    InvalidColor(String),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStyle(value) => {
                write!(
                    formatter,
                    "highlight style `{value}` must be underline or block"
                )
            }
            Self::InvalidColor(value) => {
                write!(formatter, "highlight color `{value}` must use #RRGGBB")
            }
        }
    }
}

impl std::error::Error for SettingsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_hex_colors() {
        assert_eq!(
            "#ffd54f".parse::<HighlightColor>().unwrap().to_hex(),
            "#ffd54f"
        );
        assert_eq!(
            "#A0B1C2".parse::<HighlightColor>().unwrap().to_hex(),
            "#a0b1c2"
        );
    }

    #[test]
    fn rejects_invalid_hex_colors() {
        for value in ["ffd54f", "#ffd54", "#ffd54ff", "#ggd54f", "#12345z"] {
            assert!(value.parse::<HighlightColor>().is_err(), "{value}");
        }
    }

    #[test]
    fn defaults_are_underline_with_ffd54f() {
        let settings = ReadingSettings::default();
        assert_eq!(settings.style, HighlightStyle::Underline);
        assert_eq!(settings.color.to_hex(), DEFAULT_COLOR);
        assert!(highlight_css(&settings).contains("text-decoration-line: underline"));
        assert!(highlight_css(&settings).contains("background-color: transparent"));
    }

    #[test]
    fn block_style_does_not_use_underline() {
        let settings = ReadingSettings {
            style: HighlightStyle::Block,
            color: "#112233".parse().unwrap(),
        };
        let css = highlight_css(&settings);
        assert!(css.contains("background-color: #112233"));
        assert!(css.contains("text-decoration: none"));
        assert!(!css.contains("text-decoration-line"));
    }

    #[test]
    fn style_names_are_stable_for_storage() {
        assert_eq!(
            "underline".parse::<HighlightStyle>().unwrap(),
            HighlightStyle::Underline
        );
        assert_eq!(
            "block".parse::<HighlightStyle>().unwrap(),
            HighlightStyle::Block
        );
        assert!("dotted".parse::<HighlightStyle>().is_err());
    }
}
