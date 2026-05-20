use super::spec::{SAMPLE_RATE, sound_spec};
use super::synthesis::generate_samples;
use super::{SoundKind, report_sound_status};

use rodio::buffer::SamplesBuffer;
use std::collections::HashMap;
#[cfg(any(
    target_os = "freebsd",
    target_os = "linux",
    target_os = "macos",
    target_os = "openbsd"
))]
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC};

const MIN_EVENT_SPACING: Duration = Duration::from_millis(80);

pub(super) fn run_audio_worker(rx: mpsc::Receiver<SoundKind>) {
    let stream = rodio::OutputStream::try_default();
    let Ok((_stream, handle)) = stream else {
        report_sound_status(
            "Audio output unavailable; using system notification sound fallback",
            false,
        );
        run_external_fallback_worker(rx);
        return;
    };

    let mut last_played_by_kind: HashMap<SoundKind, Instant> = HashMap::new();
    for kind in rx {
        if is_rate_limited(kind, &mut last_played_by_kind) {
            continue;
        }

        let spec = sound_spec(kind);
        let samples = generate_samples(&spec);
        let source = SamplesBuffer::new(1, SAMPLE_RATE, samples);
        if let Err(e) = handle.play_raw(source) {
            report_sound_status(
                format!("Audio playback failed: {e}; using system notification sound fallback"),
                true,
            );
            if !try_external_sound(spec.fallback_event) {
                report_sound_status("System notification sound fallback failed", true);
            }
        }
    }
}

fn run_external_fallback_worker(rx: mpsc::Receiver<SoundKind>) {
    let mut last_played_by_kind: HashMap<SoundKind, Instant> = HashMap::new();
    for kind in rx {
        if is_rate_limited(kind, &mut last_played_by_kind) {
            continue;
        }
        if !try_external_sound(sound_spec(kind).fallback_event) {
            report_sound_status("System notification sound fallback failed", true);
        }
    }
}

fn is_rate_limited(kind: SoundKind, last_played_by_kind: &mut HashMap<SoundKind, Instant>) -> bool {
    let now = Instant::now();
    if let Some(last) = last_played_by_kind.get(&kind)
        && now.duration_since(*last) < MIN_EVENT_SPACING
    {
        return true;
    }
    last_played_by_kind.insert(kind, now);
    false
}

#[cfg(any(target_os = "freebsd", target_os = "linux", target_os = "openbsd"))]
pub(super) fn try_external_sound(event_id: &str) -> bool {
    Command::new("canberra-gtk-play")
        .arg("-i")
        .arg(event_id)
        .spawn()
        .is_ok()
}

#[cfg(target_os = "macos")]
pub(super) fn try_external_sound(event_id: &str) -> bool {
    let sound_path = match event_id {
        "dialog-error" => "/System/Library/Sounds/Basso.aiff",
        "message-new-instant" => "/System/Library/Sounds/Glass.aiff",
        _ => "/System/Library/Sounds/Pop.aiff",
    };
    Command::new("afplay").arg(sound_path).spawn().is_ok()
}

#[cfg(all(
    not(target_os = "freebsd"),
    not(target_os = "linux"),
    not(target_os = "macos"),
    not(target_os = "openbsd"),
    not(target_os = "windows")
))]
pub(super) fn try_external_sound(_event_id: &str) -> bool {
    false
}

#[cfg(target_os = "windows")]
pub(super) fn try_external_sound(event_id: &str) -> bool {
    let alias = match event_id {
        "complete" => "SystemAsterisk",
        "dialog-error" => "SystemHand",
        "message-new-instant" => "SystemNotification",
        _ => "SystemDefault",
    };
    let wide_alias: Vec<u16> = alias.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        PlaySoundW(
            wide_alias.as_ptr(),
            std::ptr::null_mut(),
            SND_ALIAS | SND_ASYNC,
        ) != 0
    }
}
