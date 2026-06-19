use crate::config;
use iced::font::Family;
use iced::{Font, Settings};
use std::borrow::Cow;
use std::sync::{OnceLock, RwLock};

// ---------------------------------------------------------------------------
// Iced Font Settings
// ---------------------------------------------------------------------------

const ROBOTO_BYTES: &[u8] = include_bytes!("../assets/fonts/Roboto-Variable.ttf");
const INTER_BYTES: &[u8] = include_bytes!("../assets/fonts/Inter-Variable.ttf");
const DM_SANS_BYTES: &[u8] = include_bytes!("../assets/fonts/DMSans-Variable.ttf");
const ROBOTO_MONO_BYTES: &[u8] = include_bytes!("../assets/fonts/RobotoMono-Variable.ttf");
const QUANTICO_REGULAR_BYTES: &[u8] = include_bytes!("../assets/fonts/Quantico-Regular.ttf");
const QUANTICO_BOLD_BYTES: &[u8] = include_bytes!("../assets/fonts/Quantico-Bold.ttf");
const QUANTICO_ITALIC_BYTES: &[u8] = include_bytes!("../assets/fonts/Quantico-Italic.ttf");
const QUANTICO_BOLD_ITALIC_BYTES: &[u8] = include_bytes!("../assets/fonts/Quantico-BoldItalic.ttf");
const UBUNTU_SANS_BYTES: &[u8] = include_bytes!("../assets/fonts/UbuntuSans-Variable.ttf");
const UBUNTU_SANS_MONO_BYTES: &[u8] = include_bytes!("../assets/fonts/UbuntuSansMono-Variable.ttf");
const BUNDLED_FONT_BYTES: &[(&str, &[u8])] = &[
    (config::INTER_FONT_FAMILY, INTER_BYTES),
    (config::DM_SANS_FONT_FAMILY, DM_SANS_BYTES),
    (config::ROBOTO_FONT_FAMILY, ROBOTO_BYTES),
    (config::ROBOTO_MONO_FONT_FAMILY, ROBOTO_MONO_BYTES),
    (config::QUANTICO_FONT_FAMILY, QUANTICO_REGULAR_BYTES),
    (config::QUANTICO_FONT_FAMILY, QUANTICO_BOLD_BYTES),
    (config::QUANTICO_FONT_FAMILY, QUANTICO_ITALIC_BYTES),
    (config::QUANTICO_FONT_FAMILY, QUANTICO_BOLD_ITALIC_BYTES),
    (config::UBUNTU_SANS_FONT_FAMILY, UBUNTU_SANS_BYTES),
    (config::UBUNTU_SANS_MONO_FONT_FAMILY, UBUNTU_SANS_MONO_BYTES),
];
static MONOSPACE_FONT: OnceLock<RwLock<Font>> = OnceLock::new();

pub(crate) fn settings_from_config(config: &config::KeroseneConfig) -> Settings {
    let mut settings = Settings::default();
    let selected_family = match &config.display_font {
        config::DisplayFontConfig::System => None,
        config::DisplayFontConfig::Custom { family } => Some(family.as_str()),
    };
    let selected_monospace_family = match &config.monospace_font {
        config::DisplayFontConfig::System => None,
        config::DisplayFontConfig::Custom { family } => Some(family.as_str()),
    };
    let mut selected_family_loaded = false;
    let mut selected_monospace_family_loaded = false;

    for (family, bytes) in BUNDLED_FONT_BYTES {
        if selected_family.is_some_and(|selected| family.eq_ignore_ascii_case(selected)) {
            selected_family_loaded = true;
        }
        if selected_monospace_family.is_some_and(|selected| family.eq_ignore_ascii_case(selected)) {
            selected_monospace_family_loaded = true;
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
        if selected_monospace_family.is_some_and(|family| font.family.eq_ignore_ascii_case(family))
        {
            selected_monospace_family_loaded = true;
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
    set_monospace_font(match selected_monospace_family {
        Some(family) if selected_monospace_family_loaded => Font {
            family: Family::Name(leak_font_family_name(family)),
            ..Font::DEFAULT
        },
        _ => Font::MONOSPACE,
    });

    settings
}

pub(crate) fn monospace_font() -> Font {
    let lock = MONOSPACE_FONT.get_or_init(|| RwLock::new(Font::MONOSPACE));
    let font = match lock.read() {
        Ok(font) => font,
        Err(poisoned) => poisoned.into_inner(),
    };

    *font
}

/// Editorial serif face used for display headlines (window/pane titles, asset
/// names). Falls back to the platform serif when no bundled serif is present.
pub(crate) fn serif_font() -> Font {
    Font {
        family: Family::Serif,
        ..Font::DEFAULT
    }
}

/// UI sans face used for body copy (reflection prose, button labels).
pub(crate) fn sans_font() -> Font {
    Font {
        family: Family::Name(config::INTER_FONT_FAMILY),
        ..Font::DEFAULT
    }
}

fn set_monospace_font(font: Font) {
    let lock = MONOSPACE_FONT.get_or_init(|| RwLock::new(Font::MONOSPACE));
    let mut current_font = match lock.write() {
        Ok(font) => font,
        Err(poisoned) => poisoned.into_inner(),
    };

    *current_font = font;
}

fn leak_font_family_name(family: &str) -> &'static str {
    Box::leak(family.to_string().into_boxed_str())
}
