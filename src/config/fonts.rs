use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::fmt;

// ---------------------------------------------------------------------------
// Font Configuration
// ---------------------------------------------------------------------------

pub(crate) const INTER_FONT_FAMILY: &str = "Inter";
pub(crate) const DM_SANS_FONT_FAMILY: &str = "DM Sans";
pub(crate) const ROBOTO_FONT_FAMILY: &str = "Roboto";
pub(crate) const ROBOTO_MONO_FONT_FAMILY: &str = "Roboto Mono";
pub(crate) const QUANTICO_FONT_FAMILY: &str = "Quantico";
pub(crate) const UBUNTU_SANS_FONT_FAMILY: &str = "Ubuntu Sans";
pub(crate) const UBUNTU_SANS_MONO_FONT_FAMILY: &str = "Ubuntu Sans Mono";
pub(crate) const BUNDLED_DISPLAY_FONT_FAMILIES: &[&str] = &[
    INTER_FONT_FAMILY,
    DM_SANS_FONT_FAMILY,
    ROBOTO_FONT_FAMILY,
    ROBOTO_MONO_FONT_FAMILY,
    QUANTICO_FONT_FAMILY,
    UBUNTU_SANS_FONT_FAMILY,
    UBUNTU_SANS_MONO_FONT_FAMILY,
];

pub(crate) fn default_display_font_config() -> DisplayFontConfig {
    DisplayFontConfig::Custom {
        family: QUANTICO_FONT_FAMILY.to_string(),
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomFontConfig {
    pub family: String,
    pub file_name: String,
}

impl fmt::Debug for CustomFontConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomFontConfig")
            .field("family", &format_args!("<redacted>"))
            .field("file_name", &format_args!("<redacted>"))
            .finish()
    }
}

#[derive(Clone, Serialize, PartialEq, Eq, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum DisplayFontConfig {
    #[default]
    System,
    Custom {
        family: String,
    },
}

impl fmt::Debug for DisplayFontConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => f.write_str("System"),
            Self::Custom { .. } => f
                .debug_struct("Custom")
                .field("family", &format_args!("<redacted>"))
                .finish(),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum DisplayFontConfigWire {
    System,
    Custom { family: String },
}

impl From<DisplayFontConfigWire> for DisplayFontConfig {
    fn from(value: DisplayFontConfigWire) -> Self {
        match value {
            DisplayFontConfigWire::System => Self::System,
            DisplayFontConfigWire::Custom { family } => Self::Custom { family },
        }
    }
}

impl<'de> Deserialize<'de> for DisplayFontConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(
            match serde_json::from_value::<DisplayFontConfigWire>(value) {
                Ok(value) => value.into(),
                Err(_) => {
                    crate::config::push_config_warning(
                        "Invalid display font config; using System Default".to_string(),
                    );
                    Self::default()
                }
            },
        )
    }
}

impl fmt::Display for DisplayFontConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => f.write_str("System Default"),
            Self::Custom { family } => f.write_str(family),
        }
    }
}

impl CustomFontConfig {
    pub(crate) fn normalized(self) -> Option<Self> {
        let family = self.family.trim().to_string();
        let file_name = self.file_name.trim().to_string();

        if family.is_empty() || !custom_font_file_name_is_safe(&file_name) {
            None
        } else {
            Some(Self { family, file_name })
        }
    }
}

pub(crate) fn normalize_custom_fonts(fonts: Vec<CustomFontConfig>) -> Vec<CustomFontConfig> {
    let mut seen = HashSet::new();
    fonts
        .into_iter()
        .filter_map(CustomFontConfig::normalized)
        .filter(|font| seen.insert(font.family.to_ascii_lowercase()))
        .collect()
}

pub(crate) fn normalize_display_font(
    display_font: DisplayFontConfig,
    custom_fonts: &[CustomFontConfig],
) -> DisplayFontConfig {
    match display_font {
        DisplayFontConfig::System => DisplayFontConfig::System,
        DisplayFontConfig::Custom { family } => {
            let family = family.trim().to_string();
            if let Some(family) = bundled_display_font_family(&family) {
                return DisplayFontConfig::Custom {
                    family: family.to_string(),
                };
            }

            let matching_font = custom_fonts
                .iter()
                .find(|font| font.family.eq_ignore_ascii_case(&family));

            match matching_font {
                Some(font) if !family.is_empty() => DisplayFontConfig::Custom {
                    family: font.family.clone(),
                },
                _ => DisplayFontConfig::System,
            }
        }
    }
}

pub(crate) fn bundled_display_font_family(family: &str) -> Option<&'static str> {
    let family = family.trim();
    BUNDLED_DISPLAY_FONT_FAMILIES
        .iter()
        .copied()
        .find(|bundled| bundled.eq_ignore_ascii_case(family))
}

pub(crate) fn custom_font_file_name_is_safe(file_name: &str) -> bool {
    let file_name = file_name.trim();
    !file_name.is_empty()
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && !file_name.contains("..")
}
