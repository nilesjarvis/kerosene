use super::PreferenceAssetImportTarget;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::redact_sensitive_response_text;
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
            Message::MonospaceFontChanged(monospace_font) => {
                self.monospace_font =
                    config::normalize_display_font(monospace_font, &self.custom_fonts);
                self.persist_config();
                self.push_toast(
                    "Monospace font saved. Restart Kerosene to apply it.".to_string(),
                    false,
                );
            }
            Message::ImportDisplayFont => {
                if self.config_clear_requested || self.config_cleared_this_session {
                    self.push_toast(
                        "Font import is unavailable until Kerosene restarts.".to_string(),
                        true,
                    );
                    return Task::none();
                }
                let request = self
                    .next_preference_asset_import_request(PreferenceAssetImportTarget::DisplayFont);
                self.display_font_import_request = Some(request);
                return Task::perform(import_font(), move |result| {
                    Message::DisplayFontImported(request, result.into())
                });
            }
            Message::ImportMonospaceFont => {
                if self.config_clear_requested || self.config_cleared_this_session {
                    self.push_toast(
                        "Font import is unavailable until Kerosene restarts.".to_string(),
                        true,
                    );
                    return Task::none();
                }
                let request = self.next_preference_asset_import_request(
                    PreferenceAssetImportTarget::MonospaceFont,
                );
                self.monospace_font_import_request = Some(request);
                return Task::perform(import_font(), move |result| {
                    Message::MonospaceFontImported(request, result.into())
                });
            }
            Message::DisplayFontImported(request, result) => {
                if !request.is_for(PreferenceAssetImportTarget::DisplayFont)
                    || self.display_font_import_request != Some(request)
                {
                    return Task::none();
                }
                self.display_font_import_request = None;

                match result.into_result() {
                    Ok(font) => {
                        if self.config_clear_requested || self.config_cleared_this_session {
                            self.push_toast(
                                "Font import was discarded because config persistence is paused until restart."
                                    .to_string(),
                                true,
                            );
                            return Task::none();
                        }
                        let family = font.family.clone();
                        upsert_custom_font(&mut self.custom_fonts, font);
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
                            self.push_toast(
                                format!(
                                    "Font import failed: {}",
                                    redact_sensitive_response_text(&e)
                                ),
                                true,
                            );
                        }
                    }
                }
            }
            Message::MonospaceFontImported(request, result) => {
                if !request.is_for(PreferenceAssetImportTarget::MonospaceFont)
                    || self.monospace_font_import_request != Some(request)
                {
                    return Task::none();
                }
                self.monospace_font_import_request = None;

                match result.into_result() {
                    Ok(font) => {
                        if self.config_clear_requested || self.config_cleared_this_session {
                            self.push_toast(
                                "Font import was discarded because config persistence is paused until restart."
                                    .to_string(),
                                true,
                            );
                            return Task::none();
                        }
                        let family = font.family.clone();
                        upsert_custom_font(&mut self.custom_fonts, font);
                        self.custom_fonts =
                            config::normalize_custom_fonts(std::mem::take(&mut self.custom_fonts));
                        self.monospace_font = config::DisplayFontConfig::Custom {
                            family: family.clone(),
                        };
                        self.persist_config();
                        self.push_toast(
                            format!(
                                "Monospace font set to {family}. Restart Kerosene to apply it."
                            ),
                            false,
                        );
                    }
                    Err(e) => {
                        if e != "Import cancelled" {
                            self.push_toast(
                                format!(
                                    "Font import failed: {}",
                                    redact_sensitive_response_text(&e)
                                ),
                                true,
                            );
                        }
                    }
                }
            }
            _ => {}
        }

        Task::none()
    }
}

fn upsert_custom_font(
    custom_fonts: &mut Vec<config::CustomFontConfig>,
    font: config::CustomFontConfig,
) {
    let family = font.family.clone();
    if let Some(existing) = custom_fonts
        .iter_mut()
        .find(|existing| existing.family.eq_ignore_ascii_case(&family))
    {
        *existing = font;
    } else {
        custom_fonts.push(font);
    }
}

async fn import_font() -> Result<config::CustomFontConfig, String> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .add_filter("Font", &["ttf", "otf", "ttc"])
        .pick_file()
        .await
    else {
        return Err("Import cancelled".to_string());
    };

    let source_path = file.path().to_path_buf();
    super::ensure_import_file_within_limit(&source_path, "font", super::MAX_IMPORTED_FONT_BYTES)?;
    let bytes = std::fs::read(&source_path)
        .map_err(|e| super::import_io_failure("read selected font file", &e))?;
    let family = font_family_from_bytes(&bytes)?;
    let extension = font_extension(&source_path);
    let file_name = unique_font_file_name(&family, extension);
    let font_dir = config::font_storage_dir()
        .ok_or_else(|| "platform config directory is unavailable".to_string())?;
    std::fs::create_dir_all(&font_dir)
        .map_err(|e| super::import_io_failure("create font storage directory", &e))?;
    let destination = font_dir.join(&file_name);
    std::fs::write(&destination, bytes)
        .map_err(|e| super::import_io_failure("write imported font file", &e))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CustomFontConfig, DisplayFontConfig};
    use std::fs::File;

    #[test]
    fn completed_font_import_is_discarded_after_config_clear() {
        let (mut terminal, _) = TradingTerminal::boot();
        let display_font_before = terminal.display_font.clone();
        let _task = terminal.update_font_preferences(Message::ImportDisplayFont);
        let request = terminal
            .display_font_import_request
            .expect("display font import owner");
        terminal.config_cleared_this_session = true;

        let _task = terminal.update_font_preferences(Message::DisplayFontImported(
            request,
            Ok(CustomFontConfig {
                family: "After Clear".to_string(),
                file_name: "after-clear.ttf".to_string(),
            })
            .into(),
        ));

        assert!(terminal.custom_fonts.is_empty());
        assert_eq!(terminal.display_font, display_font_before);
        assert!(terminal.config_save_due_at.is_none());
        assert!(terminal.display_font_import_request.is_none());
    }

    #[test]
    fn completed_font_import_is_discarded_while_config_clear_is_pending() {
        let (mut terminal, _) = TradingTerminal::boot();
        let monospace_font_before = terminal.monospace_font.clone();
        let _task = terminal.update_font_preferences(Message::ImportMonospaceFont);
        let request = terminal
            .monospace_font_import_request
            .expect("monospace font import owner");
        terminal.config_clear_requested = true;

        let _task = terminal.update_font_preferences(Message::MonospaceFontImported(
            request,
            Ok(CustomFontConfig {
                family: "During Clear".to_string(),
                file_name: "during-clear.ttf".to_string(),
            })
            .into(),
        ));

        assert!(terminal.custom_fonts.is_empty());
        assert_eq!(terminal.monospace_font, monospace_font_before);
        assert!(terminal.config_save_due_at.is_none());
        assert!(terminal.monospace_font_import_request.is_none());
    }

    #[test]
    fn newer_display_font_import_cannot_be_overwritten_by_older_completion() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.preference_asset_import_next_request_id = u64::MAX - 1;

        let _task = terminal.update_font_preferences(Message::ImportDisplayFont);
        let older_request = terminal
            .display_font_import_request
            .expect("older display font import owner");
        let _task = terminal.update_font_preferences(Message::ImportDisplayFont);
        let newer_request = terminal
            .display_font_import_request
            .expect("newer display font import owner");
        assert_eq!(older_request.request_id(), u64::MAX);
        assert_eq!(newer_request.request_id(), 0);

        let _task = terminal.update_font_preferences(Message::DisplayFontImported(
            newer_request,
            Ok(CustomFontConfig {
                family: "Newer Family".to_string(),
                file_name: "newer-family.ttf".to_string(),
            })
            .into(),
        ));
        let toast_count = terminal.toasts.len();
        let _task = terminal.update_font_preferences(Message::DisplayFontImported(
            older_request,
            Ok(CustomFontConfig {
                family: "Older Family".to_string(),
                file_name: "older-family.ttf".to_string(),
            })
            .into(),
        ));

        assert_eq!(
            terminal.display_font,
            DisplayFontConfig::Custom {
                family: "Newer Family".to_string(),
            }
        );
        assert_eq!(terminal.toasts.len(), toast_count);
        assert!(terminal.config_save_due_at.is_some());
        let toast = terminal.toasts.last().expect("display import toast");
        assert!(!toast.is_error);
        assert_eq!(
            toast.message,
            "Display font set to Newer Family. Restart Kerosene to apply it."
        );
    }

    #[test]
    fn display_and_monospace_imports_keep_independent_owners() {
        let (mut terminal, _) = TradingTerminal::boot();

        let _task = terminal.update_font_preferences(Message::ImportDisplayFont);
        let display_request = terminal
            .display_font_import_request
            .expect("display font import owner");
        let _task = terminal.update_font_preferences(Message::ImportMonospaceFont);
        let monospace_request = terminal
            .monospace_font_import_request
            .expect("monospace font import owner");

        let _task = terminal.update_font_preferences(Message::MonospaceFontImported(
            monospace_request,
            Ok(CustomFontConfig {
                family: "Mono Family".to_string(),
                file_name: "mono-family.ttf".to_string(),
            })
            .into(),
        ));
        let _task = terminal.update_font_preferences(Message::DisplayFontImported(
            display_request,
            Ok(CustomFontConfig {
                family: "Display Family".to_string(),
                file_name: "display-family.ttf".to_string(),
            })
            .into(),
        ));

        assert_eq!(
            terminal.display_font,
            DisplayFontConfig::Custom {
                family: "Display Family".to_string(),
            }
        );
        assert_eq!(
            terminal.monospace_font,
            DisplayFontConfig::Custom {
                family: "Mono Family".to_string(),
            }
        );
        assert!(terminal.display_font_import_request.is_none());
        assert!(terminal.monospace_font_import_request.is_none());
        assert_eq!(terminal.custom_fonts.len(), 2);
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn cross_target_result_cannot_settle_display_import_owner() {
        let (mut terminal, _) = TradingTerminal::boot();
        let display_font_before = terminal.display_font.clone();

        let _task = terminal.update_font_preferences(Message::ImportDisplayFont);
        let display_request = terminal
            .display_font_import_request
            .expect("display font import owner");
        let wrong_target_request = crate::preferences_update::PreferenceAssetImportRequest::new(
            display_request.request_id(),
            PreferenceAssetImportTarget::MonospaceFont,
        );
        let _task = terminal.update_font_preferences(Message::DisplayFontImported(
            wrong_target_request,
            Ok(CustomFontConfig {
                family: "Wrong Target".to_string(),
                file_name: "wrong-target.ttf".to_string(),
            })
            .into(),
        ));

        assert_eq!(terminal.display_font_import_request, Some(display_request));
        assert_eq!(terminal.display_font, display_font_before);
        assert!(terminal.custom_fonts.is_empty());
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn settings_window_close_does_not_cancel_app_global_import_owner() {
        let (mut terminal, _) = TradingTerminal::boot();
        let settings_window_id = iced::window::Id::unique();
        terminal.settings_window_id = Some(settings_window_id);

        let _task = terminal.update_font_preferences(Message::ImportDisplayFont);
        let request = terminal
            .display_font_import_request
            .expect("display font import owner");
        let _task = terminal.update_window(Message::WindowClosed(settings_window_id));

        assert!(terminal.settings_window_id.is_none());
        assert_eq!(terminal.display_font_import_request, Some(request));

        let _task = terminal.update_font_preferences(Message::DisplayFontImported(
            request,
            Ok(CustomFontConfig {
                family: "After Settings Close".to_string(),
                file_name: "after-settings-close.ttf".to_string(),
            })
            .into(),
        ));

        assert_eq!(
            terminal.display_font,
            DisplayFontConfig::Custom {
                family: "After Settings Close".to_string(),
            }
        );
        assert!(terminal.display_font_import_request.is_none());
        assert!(terminal.config_save_due_at.is_some());
    }

    #[test]
    fn current_font_import_cancellation_clears_only_its_owner_without_feedback() {
        let (mut terminal, _) = TradingTerminal::boot();
        let display_font_before = terminal.display_font.clone();
        let toast_count = terminal.toasts.len();

        let _task = terminal.update_font_preferences(Message::ImportDisplayFont);
        let request = terminal
            .display_font_import_request
            .expect("display font import owner");
        let _task = terminal.update_font_preferences(Message::DisplayFontImported(
            request,
            Err("Import cancelled".to_string()).into(),
        ));

        assert!(terminal.display_font_import_request.is_none());
        assert_eq!(terminal.display_font, display_font_before);
        assert!(terminal.custom_fonts.is_empty());
        assert_eq!(terminal.toasts.len(), toast_count);
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn oversized_font_import_file_is_rejected_before_read() {
        let path = unique_temp_path("oversized-font.ttf");
        let file = File::create(&path).expect("create sparse font fixture");
        file.set_len(super::super::MAX_IMPORTED_FONT_BYTES + 1)
            .expect("size sparse font fixture");

        let err = super::super::ensure_import_file_within_limit(
            &path,
            "font",
            super::super::MAX_IMPORTED_FONT_BYTES,
        )
        .expect_err("oversized font should be rejected");

        let _ = std::fs::remove_file(&path);
        assert!(err.contains("too large"));
    }

    #[test]
    fn display_font_import_error_redacts_toast_detail() {
        let (mut terminal, _) = TradingTerminal::boot();
        let _task = terminal.update_font_preferences(Message::ImportDisplayFont);
        let request = terminal
            .display_font_import_request
            .expect("display font import owner");

        let _task = terminal.update_font_preferences(Message::DisplayFontImported(
            request,
            Err("read failed: api_key=font-secret".to_string()).into(),
        ));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("api_key=<redacted>"));
        assert!(!toast.message.contains("font-secret"));
    }

    #[test]
    fn monospace_font_import_error_redacts_toast_detail() {
        let (mut terminal, _) = TradingTerminal::boot();
        let _task = terminal.update_font_preferences(Message::ImportMonospaceFont);
        let request = terminal
            .monospace_font_import_request
            .expect("monospace font import owner");

        let _task = terminal.update_font_preferences(Message::MonospaceFontImported(
            request,
            Err("write failed: signature=sig-secret".to_string()).into(),
        ));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("signature=<redacted>"));
        assert!(!toast.message.contains("sig-secret"));
    }

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("kerosene-{nanos}-{name}"))
    }
}
