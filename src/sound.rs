use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Mutex, OnceLock};

mod spec;
mod synthesis;
mod worker;

use spec::sound_spec;
use worker::{run_audio_worker, try_external_sound};

// ---------------------------------------------------------------------------
// Sound notifications
// ---------------------------------------------------------------------------

const EVENT_QUEUE_CAPACITY: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SoundStatus {
    pub(crate) message: String,
    pub(crate) is_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundKind {
    Fill,
    Error,
    Interest,
}

/// Play a fill notification sound.
pub fn play_fill() {
    play(SoundKind::Fill);
}

/// Play an error notification sound.
pub fn play_error() {
    play(SoundKind::Error);
}

/// Play an interest notification sound.
pub fn play_interest() {
    play(SoundKind::Interest);
}

pub fn play(kind: SoundKind) {
    let sender = sound_sender();
    match sender.try_send(kind) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            report_sound_status("Audio notification queue is full; dropped sound", true);
        }
        Err(TrySendError::Disconnected(_)) => {
            if !try_external_sound(sound_spec(kind).fallback_event) {
                report_sound_status(
                    "Audio worker stopped and system notification sound fallback failed",
                    true,
                );
            }
        }
    }
}

pub(crate) fn take_status_messages() -> Vec<SoundStatus> {
    sound_statuses()
        .lock()
        .map(|mut statuses| std::mem::take(&mut *statuses))
        .unwrap_or_default()
}

fn sound_statuses() -> &'static Mutex<Vec<SoundStatus>> {
    static STATUSES: OnceLock<Mutex<Vec<SoundStatus>>> = OnceLock::new();
    STATUSES.get_or_init(|| Mutex::new(Vec::new()))
}

pub(super) fn report_sound_status(message: impl Into<String>, is_error: bool) {
    let message = message.into();
    if is_error {
        eprintln!("{message}");
    }
    if let Ok(mut statuses) = sound_statuses().lock() {
        if statuses.iter().any(|status| status.message == message) {
            return;
        }
        statuses.push(SoundStatus { message, is_error });
    }
}

fn sound_sender() -> &'static SyncSender<SoundKind> {
    static SENDER: OnceLock<SyncSender<SoundKind>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        if let Err(e) = std::thread::Builder::new()
            .name("kerosene-audio".to_string())
            .spawn(move || run_audio_worker(rx))
        {
            report_sound_status(format!("Audio worker spawn failed: {e}"), true);
        }
        tx
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_status_messages_are_deduplicated_until_drained() {
        report_sound_status("same audio warning", true);
        report_sound_status("same audio warning", true);

        let statuses = take_status_messages();

        assert_eq!(
            statuses,
            vec![SoundStatus {
                message: "same audio warning".to_string(),
                is_error: true,
            }]
        );
        assert!(take_status_messages().is_empty());
    }
}
