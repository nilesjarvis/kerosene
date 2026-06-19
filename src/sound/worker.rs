use super::spec::{SAMPLE_RATE, sound_spec};
use super::synthesis::generate_samples;
use super::{SoundKind, SoundRequest, SoundSource, report_sound_status};
use crate::helpers::path_neutral_io_error_detail;

use rodio::Decoder;
use rodio::Source;
use rodio::buffer::SamplesBuffer;
use std::collections::HashMap;
use std::io::Cursor;
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

pub(super) fn run_audio_worker(rx: mpsc::Receiver<SoundRequest>) {
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
    for request in rx {
        if is_rate_limited(request.kind, &mut last_played_by_kind) {
            continue;
        }

        let fallback_event = sound_spec(request.kind).fallback_event;
        if let Err(e) = play_request(&handle, request) {
            report_sound_status(
                format!("Audio playback failed: {e}; using system notification sound fallback"),
                true,
            );
            if let Some(event) = fallback_event
                && !try_external_sound(event)
            {
                report_sound_status("System notification sound fallback failed", true);
            }
        }
    }
}

fn run_external_fallback_worker(rx: mpsc::Receiver<SoundRequest>) {
    let mut last_played_by_kind: HashMap<SoundKind, Instant> = HashMap::new();
    for request in rx {
        if is_rate_limited(request.kind, &mut last_played_by_kind) {
            continue;
        }
        if let Some(event) = sound_spec(request.kind).fallback_event
            && !try_external_sound(event)
        {
            report_sound_status("System notification sound fallback failed", true);
        }
    }
}

fn play_request(handle: &rodio::OutputStreamHandle, request: SoundRequest) -> Result<(), String> {
    let volume = normalized_volume(request.volume);
    match request.source {
        SoundSource::Synth => {
            let spec = sound_spec(request.kind);
            let samples = generate_samples(&spec);
            let source = SamplesBuffer::new(1, SAMPLE_RATE, samples).amplify(volume);
            handle.play_raw(source).map_err(|e| e.to_string())
        }
        SoundSource::EmbeddedWav(bytes) => play_wav_bytes(handle, bytes, volume),
        SoundSource::FileWav(path) => {
            let bytes = std::fs::read(&path).map_err(|e| custom_wav_read_failure(&e))?;
            play_wav_bytes(handle, &bytes, volume)
        }
    }
}

pub(super) fn custom_wav_read_failure(error: &std::io::Error) -> String {
    format!(
        "read custom WAV file failed: {}",
        path_neutral_io_error_detail(error)
    )
}

fn play_wav_bytes(
    handle: &rodio::OutputStreamHandle,
    bytes: &[u8],
    volume: f32,
) -> Result<(), String> {
    let cursor = Cursor::new(bytes.to_vec());
    let source = Decoder::new_wav(cursor)
        .map_err(|e| e.to_string())?
        .convert_samples()
        .amplify(volume);
    handle.play_raw(source).map_err(|e| e.to_string())
}

fn normalized_volume(volume: f32) -> f32 {
    if volume.is_finite() {
        volume.clamp(0.0, 1.0)
    } else {
        1.0
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
