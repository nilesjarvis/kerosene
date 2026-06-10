use crate::config::ChartHudOrderSound;
use std::path::PathBuf;
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
    HudOrder,
    HudModeLimit,
    HudModeMarket,
    HudSideLong,
    HudSideShort,
    HudArm,
    HudDisarm,
    HudAutoDisarm,
    HudIdleWarning,
    HudSizeUp,
    HudSizeDown,
}

/// Interface click played when a HUD game-mode control changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudUiSound {
    ModeLimit,
    ModeMarket,
    SideLong,
    SideShort,
    Arm,
    Disarm,
    AutoDisarm,
    IdleWarning,
    SizeUp,
    SizeDown,
}

impl HudUiSound {
    fn kind(self) -> SoundKind {
        match self {
            Self::ModeLimit => SoundKind::HudModeLimit,
            Self::ModeMarket => SoundKind::HudModeMarket,
            Self::SideLong => SoundKind::HudSideLong,
            Self::SideShort => SoundKind::HudSideShort,
            Self::Arm => SoundKind::HudArm,
            Self::Disarm => SoundKind::HudDisarm,
            Self::AutoDisarm => SoundKind::HudAutoDisarm,
            Self::IdleWarning => SoundKind::HudIdleWarning,
            Self::SizeUp => SoundKind::HudSizeUp,
            Self::SizeDown => SoundKind::HudSizeDown,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum SoundSource {
    Synth,
    EmbeddedWav(&'static [u8]),
    FileWav(PathBuf),
}

#[derive(Debug, Clone)]
pub(super) struct SoundRequest {
    pub(super) kind: SoundKind,
    pub(super) source: SoundSource,
    pub(super) volume: f32,
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

pub fn play_hud_order(sound: ChartHudOrderSound, custom_path: Option<PathBuf>, volume: f32) {
    let volume = normalized_volume(volume);
    match sound {
        ChartHudOrderSound::Off => {}
        ChartHudOrderSound::FillTone => play_request(SoundRequest {
            kind: SoundKind::HudOrder,
            source: SoundSource::Synth,
            volume,
        }),
        ChartHudOrderSound::GunShot8Bit => play_request(SoundRequest {
            kind: SoundKind::HudOrder,
            source: SoundSource::EmbeddedWav(include_bytes!(
                "../assets/sounds/hud-order-gun-shot-8-bit.wav"
            )),
            volume,
        }),
        ChartHudOrderSound::CustomWav => {
            if let Some(path) = custom_path {
                play_request(SoundRequest {
                    kind: SoundKind::HudOrder,
                    source: SoundSource::FileWav(path),
                    volume,
                });
            } else {
                play_request(SoundRequest {
                    kind: SoundKind::HudOrder,
                    source: SoundSource::EmbeddedWav(include_bytes!(
                        "../assets/sounds/hud-order-gun-shot-8-bit.wav"
                    )),
                    volume,
                });
            }
        }
    }
}

/// Play a synthesized HUD interface click at the HUD sound volume.
pub fn play_hud_ui(sound: HudUiSound, volume: f32) {
    play_request(SoundRequest {
        kind: sound.kind(),
        source: SoundSource::Synth,
        volume: normalized_volume(volume),
    });
}

pub fn play(kind: SoundKind) {
    play_request(SoundRequest {
        kind,
        source: SoundSource::Synth,
        volume: 1.0,
    });
}

fn normalized_volume(volume: f32) -> f32 {
    if volume.is_finite() {
        volume.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn play_request(request: SoundRequest) {
    let sender = sound_sender();
    let fallback_event = sound_spec(request.kind).fallback_event;
    match sender.try_send(request) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            report_sound_status("Audio notification queue is full; dropped sound", true);
        }
        Err(TrySendError::Disconnected(_)) => {
            if let Some(event) = fallback_event
                && !try_external_sound(event)
            {
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

fn sound_sender() -> &'static SyncSender<SoundRequest> {
    static SENDER: OnceLock<SyncSender<SoundRequest>> = OnceLock::new();
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
mod tests;
