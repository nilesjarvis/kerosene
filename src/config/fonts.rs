use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

// ---------------------------------------------------------------------------
// Font Configuration
// ---------------------------------------------------------------------------

pub(crate) const INTER_FONT_FAMILY: &str = "Inter";
pub(crate) const ROBOTO_FONT_FAMILY: &str = "Roboto";
pub(crate) const ROBOTO_MONO_FONT_FAMILY: &str = "Roboto Mono";
pub(crate) const BUNDLED_DISPLAY_FONT_FAMILIES: &[&str] = &[
    INTER_FONT_FAMILY,
    ROBOTO_FONT_FAMILY,
    ROBOTO_MONO_FONT_FAMILY,
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomFontConfig {
    pub family: String,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum DisplayFontConfig {
    #[default]
    System,
    Custom {
        family: String,
    },
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
