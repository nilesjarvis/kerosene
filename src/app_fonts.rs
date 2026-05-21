use crate::config;
use iced::font::Family;
use iced::{Font, Settings};
use std::borrow::Cow;

// ---------------------------------------------------------------------------
// Iced Font Settings
// ---------------------------------------------------------------------------

const ROBOTO_BYTES: &[u8] = include_bytes!("../assets/fonts/Roboto-Variable.ttf");
const INTER_BYTES: &[u8] = include_bytes!("../assets/fonts/Inter-Variable.ttf");
const ROBOTO_MONO_BYTES: &[u8] = include_bytes!("../assets/fonts/RobotoMono-Variable.ttf");
const BUNDLED_FONT_BYTES: &[(&str, &[u8])] = &[
    (config::INTER_FONT_FAMILY, INTER_BYTES),
    (config::ROBOTO_FONT_FAMILY, ROBOTO_BYTES),
    (config::ROBOTO_MONO_FONT_FAMILY, ROBOTO_MONO_BYTES),
];

pub(crate) fn settings_from_config(config: &config::KeroseneConfig) -> Settings {
    let mut settings = Settings::default();
    let selected_family = match &config.display_font {
        config::DisplayFontConfig::System => None,
        config::DisplayFontConfig::Custom { family } => Some(family.as_str()),
    };
    let mut selected_family_loaded = false;

    for (family, bytes) in BUNDLED_FONT_BYTES {
        if selected_family.is_some_and(|selected| family.eq_ignore_ascii_case(selected)) {
            selected_family_loaded = true;
        }

        settings.fonts.push(Cow::Borrowed(*bytes));
    }

    for font in &config.custom_fonts {
        let Some(path) = config::custom_font_path(&font.file_name) else {
            continue;
        };
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };

        if selected_family.is_some_and(|family| font.family.eq_ignore_ascii_case(family)) {
            selected_family_loaded = true;
        }

        settings.fonts.push(Cow::Owned(bytes));
    }

    if let Some(family) = selected_family
        && selected_family_loaded
    {
        settings.default_font = Font {
            family: Family::Name(leak_font_family_name(family)),
            ..Font::DEFAULT
        };
    }

    settings
}

fn leak_font_family_name(family: &str) -> &'static str {
    Box::leak(family.to_string().into_boxed_str())
}
