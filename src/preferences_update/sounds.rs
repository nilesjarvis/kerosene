use super::PreferenceAssetImportTarget;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::sound;
use iced::Task;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Sound Preferences
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn update_sound_preferences(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChartHudOrderSoundChanged(sound) => {
                self.chart_hud_order_sound = sound;
                self.persist_config();
            }
            Message::ChartHudOrderSoundVolumeChanged(volume) => {
                self.chart_hud_order_sound_volume =
                    config::normalize_chart_hud_order_sound_volume(volume);
                self.persist_config();
            }
            Message::ImportChartHudOrderSound => {
                if self.config_clear_requested || self.config_cleared_this_session {
                    self.push_toast(
                        "HUD order sound import is unavailable until Kerosene restarts."
                            .to_string(),
                        true,
                    );
                    return Task::none();
                }
                let request = self.next_preference_asset_import_request(
                    PreferenceAssetImportTarget::ChartHudOrderSound,
                );
                self.chart_hud_order_sound_import_request = Some(request);
                return Task::perform(import_hud_order_sound(), move |result| {
                    Message::ChartHudOrderSoundImported(request, result.into())
                });
            }
            Message::ChartHudOrderSoundImported(request, result) => {
                if !request.is_for(PreferenceAssetImportTarget::ChartHudOrderSound)
                    || self.chart_hud_order_sound_import_request != Some(request)
                {
                    return Task::none();
                }
                self.chart_hud_order_sound_import_request = None;

                match result.into_result() {
                    Ok(Some(file_name)) => {
                        if self.config_clear_requested || self.config_cleared_this_session {
                            self.push_toast(
                                "HUD order sound import was discarded because config persistence is paused until restart."
                                    .to_string(),
                                true,
                            );
                            return Task::none();
                        }
                        self.chart_hud_order_sound_file = Some(file_name);
                        self.chart_hud_order_sound = config::ChartHudOrderSound::CustomWav;
                        self.persist_config();
                        self.push_toast("HUD order sound imported".to_string(), false);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        if e != "Import cancelled" {
                            self.push_toast(
                                format!(
                                    "HUD order sound import failed: {}",
                                    redact_sensitive_response_text(&e)
                                ),
                                true,
                            );
                        }
                    }
                }
            }
            Message::TestChartHudOrderSound => {
                sound::play_hud_order(
                    self.chart_hud_order_sound,
                    self.chart_hud_order_sound_path(),
                    self.chart_hud_order_sound_volume,
                );
            }
            Message::ToggleChartHudUiSounds(enabled) => {
                self.chart_hud_ui_sounds = enabled;
                self.persist_config();
                if enabled && self.sound_enabled {
                    sound::play_hud_ui(sound::HudUiSound::Arm, self.chart_hud_order_sound_volume);
                }
            }
            _ => {}
        }

        Task::none()
    }

    pub(crate) fn chart_hud_order_sound_path(&self) -> Option<std::path::PathBuf> {
        self.chart_hud_order_sound_file
            .as_deref()
            .and_then(config::custom_sound_path)
    }
}

async fn import_hud_order_sound() -> Result<Option<String>, String> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .add_filter("WAV audio", &["wav"])
        .pick_file()
        .await
    else {
        return Err("Import cancelled".to_string());
    };

    let source_path = file.path().to_path_buf();
    super::ensure_import_file_within_limit(
        &source_path,
        "HUD order sound",
        super::MAX_IMPORTED_HUD_SOUND_BYTES,
    )?;
    let bytes = std::fs::read(&source_path)
        .map_err(|e| super::import_io_failure("read selected HUD order sound file", &e))?;
    validate_wav(&bytes)?;

    let file_name = unique_sound_file_name(&source_path);
    let sound_dir = config::sound_storage_dir()
        .ok_or_else(|| "platform config directory is unavailable".to_string())?;
    std::fs::create_dir_all(&sound_dir)
        .map_err(|e| super::import_io_failure("create HUD order sound storage directory", &e))?;
    let destination = sound_dir.join(&file_name);
    std::fs::write(&destination, bytes)
        .map_err(|e| super::import_io_failure("write imported HUD order sound file", &e))?;

    Ok(Some(file_name))
}

fn validate_wav(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("selected file is not a WAV file".to_string());
    }
    Ok(())
}

fn unique_sound_file_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("hud-order")
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
    let stem = if stem.is_empty() {
        "hud-order".to_string()
    } else {
        stem
    };
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    format!("{stem}-{millis}.wav")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChartHudOrderSound;
    use std::fs::File;

    #[test]
    fn completed_hud_sound_import_is_discarded_after_config_clear() {
        let (mut terminal, _) = TradingTerminal::boot();
        let _task = terminal.update_sound_preferences(Message::ImportChartHudOrderSound);
        let request = terminal
            .chart_hud_order_sound_import_request
            .expect("sound import owner");
        terminal.config_cleared_this_session = true;

        let _task = terminal.update_sound_preferences(Message::ChartHudOrderSoundImported(
            request,
            Ok(Some("after-clear.wav".to_string())).into(),
        ));

        assert_eq!(terminal.chart_hud_order_sound_file, None);
        assert_eq!(
            terminal.chart_hud_order_sound,
            ChartHudOrderSound::default()
        );
        assert!(terminal.config_save_due_at.is_none());
        assert!(terminal.chart_hud_order_sound_import_request.is_none());
    }

    #[test]
    fn newer_hud_sound_import_cannot_be_overwritten_by_older_completion() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.preference_asset_import_next_request_id = u64::MAX - 1;

        let _task = terminal.update_sound_preferences(Message::ImportChartHudOrderSound);
        let older_request = terminal
            .chart_hud_order_sound_import_request
            .expect("older sound import owner");
        let _task = terminal.update_sound_preferences(Message::ImportChartHudOrderSound);
        let newer_request = terminal
            .chart_hud_order_sound_import_request
            .expect("newer sound import owner");
        assert_eq!(older_request.request_id(), u64::MAX);
        assert_eq!(newer_request.request_id(), 0);

        let _task = terminal.update_sound_preferences(Message::ChartHudOrderSoundImported(
            newer_request,
            Ok(Some("newer.wav".to_string())).into(),
        ));
        let toast_count = terminal.toasts.len();
        let _task = terminal.update_sound_preferences(Message::ChartHudOrderSoundImported(
            older_request,
            Ok(Some("older.wav".to_string())).into(),
        ));

        assert_eq!(
            terminal.chart_hud_order_sound_file.as_deref(),
            Some("newer.wav")
        );
        assert_eq!(
            terminal.chart_hud_order_sound,
            ChartHudOrderSound::CustomWav
        );
        assert_eq!(terminal.toasts.len(), toast_count);
        assert!(terminal.config_save_due_at.is_some());
        let toast = terminal.toasts.last().expect("import success toast");
        assert!(!toast.is_error);
        assert_eq!(toast.message, "HUD order sound imported");
    }

    #[test]
    fn current_hud_sound_import_cancellation_clears_owner_without_feedback() {
        let (mut terminal, _) = TradingTerminal::boot();
        let sound_before = terminal.chart_hud_order_sound;
        let file_before = terminal.chart_hud_order_sound_file.clone();
        let toast_count = terminal.toasts.len();

        let _task = terminal.update_sound_preferences(Message::ImportChartHudOrderSound);
        let request = terminal
            .chart_hud_order_sound_import_request
            .expect("sound import owner");
        let _task = terminal.update_sound_preferences(Message::ChartHudOrderSoundImported(
            request,
            Err("Import cancelled".to_string()).into(),
        ));

        assert!(terminal.chart_hud_order_sound_import_request.is_none());
        assert_eq!(terminal.chart_hud_order_sound, sound_before);
        assert_eq!(terminal.chart_hud_order_sound_file, file_before);
        assert_eq!(terminal.toasts.len(), toast_count);
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn newer_cancellation_prevents_older_sound_import_from_reviving() {
        let (mut terminal, _) = TradingTerminal::boot();
        let sound_before = terminal.chart_hud_order_sound;
        let file_before = terminal.chart_hud_order_sound_file.clone();
        let toast_count = terminal.toasts.len();

        let _task = terminal.update_sound_preferences(Message::ImportChartHudOrderSound);
        let older_request = terminal
            .chart_hud_order_sound_import_request
            .expect("older sound import owner");
        let _task = terminal.update_sound_preferences(Message::ImportChartHudOrderSound);
        let newer_request = terminal
            .chart_hud_order_sound_import_request
            .expect("newer sound import owner");
        let _task = terminal.update_sound_preferences(Message::ChartHudOrderSoundImported(
            newer_request,
            Err("Import cancelled".to_string()).into(),
        ));
        let _task = terminal.update_sound_preferences(Message::ChartHudOrderSoundImported(
            older_request,
            Ok(Some("older.wav".to_string())).into(),
        ));

        assert!(terminal.chart_hud_order_sound_import_request.is_none());
        assert_eq!(terminal.chart_hud_order_sound, sound_before);
        assert_eq!(terminal.chart_hud_order_sound_file, file_before);
        assert_eq!(terminal.toasts.len(), toast_count);
        assert!(terminal.config_save_due_at.is_none());
    }

    #[test]
    fn oversized_hud_sound_import_file_is_rejected_before_read() {
        let path = unique_temp_path("oversized-hud-order.wav");
        let file = File::create(&path).expect("create sparse sound fixture");
        file.set_len(super::super::MAX_IMPORTED_HUD_SOUND_BYTES + 1)
            .expect("size sparse sound fixture");

        let err = super::super::ensure_import_file_within_limit(
            &path,
            "HUD order sound",
            super::super::MAX_IMPORTED_HUD_SOUND_BYTES,
        )
        .expect_err("oversized HUD sound should be rejected");

        let _ = std::fs::remove_file(&path);
        assert!(err.contains("too large"));
    }

    #[test]
    fn hud_order_sound_import_error_redacts_toast_detail() {
        let (mut terminal, _) = TradingTerminal::boot();
        let _task = terminal.update_sound_preferences(Message::ImportChartHudOrderSound);
        let request = terminal
            .chart_hud_order_sound_import_request
            .expect("sound import owner");

        let _task = terminal.update_sound_preferences(Message::ChartHudOrderSoundImported(
            request,
            Err("copy failed: auth_token=token-secret".to_string()).into(),
        ));

        let toast = terminal.toasts.last().expect("toast");
        assert!(toast.is_error);
        assert!(toast.message.contains("auth_token=<redacted>"));
        assert!(!toast.message.contains("token-secret"));
    }

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("kerosene-{nanos}-{name}"))
    }
}
