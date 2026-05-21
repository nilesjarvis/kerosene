use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use iced::Task;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Font Preferences
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn update_font_preferences(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DisplayFontChanged(display_font) => {
                self.display_font =
                    config::normalize_display_font(display_font, &self.custom_fonts);
                self.persist_config();
                self.push_toast(
                    "Display font saved. Restart Kerosene to apply it.".to_string(),
                    false,
                );
            }
            Message::ImportDisplayFont => {
                return Task::perform(import_display_font(), Message::DisplayFontImported);
            }
            Message::DisplayFontImported(result) => match result {
                Ok(font) => {
                    let family = font.family.clone();
                    if let Some(existing) = self
                        .custom_fonts
                        .iter_mut()
                        .find(|existing| existing.family.eq_ignore_ascii_case(&family))
                    {
                        *existing = font;
                    } else {
                        self.custom_fonts.push(font);
                    }

                    self.custom_fonts =
                        config::normalize_custom_fonts(std::mem::take(&mut self.custom_fonts));
                    self.display_font = config::DisplayFontConfig::Custom {
                        family: family.clone(),
                    };
                    self.persist_config();
                    self.push_toast(
                        format!("Display font set to {family}. Restart Kerosene to apply it."),
                        false,
                    );
                }
                Err(e) => {
                    if e != "Import cancelled" {
                        self.push_toast(format!("Font import failed: {e}"), true);
                    }
                }
            },
            _ => {}
        }

        Task::none()
    }
}

async fn import_display_font() -> Result<config::CustomFontConfig, String> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .add_filter("Font", &["ttf", "otf", "ttc"])
        .pick_file()
        .await
    else {
        return Err("Import cancelled".to_string());
    };

    let source_path = file.path().to_path_buf();
    let bytes = std::fs::read(&source_path)
        .map_err(|e| format!("read {} failed: {e}", source_path.display()))?;
    let family = font_family_from_bytes(&bytes)?;
    let extension = font_extension(&source_path);
    let file_name = unique_font_file_name(&family, extension);
    let font_dir = config::font_storage_dir()
        .ok_or_else(|| "platform config directory is unavailable".to_string())?;
    std::fs::create_dir_all(&font_dir)
        .map_err(|e| format!("create font directory {} failed: {e}", font_dir.display()))?;
    let destination = font_dir.join(&file_name);
    std::fs::write(&destination, bytes)
        .map_err(|e| format!("write {} failed: {e}", destination.display()))?;

    Ok(config::CustomFontConfig { family, file_name })
}

fn font_family_from_bytes(bytes: &[u8]) -> Result<String, String> {
    let mut database = fontdb::Database::new();
    database.load_font_data(bytes.to_vec());

    database
        .faces()
        .filter_map(|face| face.families.first())
        .map(|(family, _language)| family.trim())
        .find(|family| !family.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| "font file did not contain a readable family name".to_string())
}

fn font_extension(path: &Path) -> &str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("otf") => "otf",
        Some("ttc") => "ttc",
        _ => "ttf",
    }
}

fn unique_font_file_name(family: &str, extension: &str) -> String {
    let base = family
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let base = if base.is_empty() {
        "font".to_string()
    } else {
        base
    };
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    format!("{base}-{millis}.{extension}")
}
