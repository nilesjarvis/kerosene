use super::spec::{SAMPLE_RATE, Waveform, sound_spec};
use super::synthesis::generate_samples;
use super::*;

#[test]
fn hud_ui_sounds_map_to_distinct_silent_fallback_kinds() {
    let sounds = [
        HudUiSound::ModeLimit,
        HudUiSound::ModeMarket,
        HudUiSound::SideLong,
        HudUiSound::SideShort,
        HudUiSound::Arm,
        HudUiSound::Disarm,
        HudUiSound::AutoDisarm,
        HudUiSound::IdleWarning,
        HudUiSound::SizeUp,
        HudUiSound::SizeDown,
    ];

    let mut kinds = Vec::new();
    for sound in sounds {
        let kind = sound.kind();
        assert!(!kinds.contains(&kind), "duplicate sound kind for {sound:?}");
        kinds.push(kind);

        let spec = sound_spec(kind);
        assert!(!spec.tones.is_empty());
        assert!(
            spec.fallback_event.is_none(),
            "cosmetic HUD click {sound:?} must not trigger OS notification sounds"
        );
    }
}

#[test]
fn hud_action_clicks_use_the_square_waveform() {
    for kind in [
        SoundKind::HudModeLimit,
        SoundKind::HudModeMarket,
        SoundKind::HudSideLong,
        SoundKind::HudSideShort,
        SoundKind::HudDisarm,
        SoundKind::HudSizeUp,
        SoundKind::HudSizeDown,
    ] {
        assert!(
            sound_spec(kind)
                .tones
                .iter()
                .all(|tone| tone.waveform == Waveform::Square),
            "{kind:?} should use the 8-bit square waveform"
        );
    }
}

#[test]
fn hud_advisory_sounds_use_sine_outside_the_action_vocabulary() {
    for kind in [SoundKind::HudAutoDisarm, SoundKind::HudIdleWarning] {
        assert!(
            sound_spec(kind)
                .tones
                .iter()
                .all(|tone| tone.waveform == Waveform::Sine),
            "{kind:?} should sound advisory (sine), not like a manual action"
        );
    }
}

#[test]
fn hud_arm_and_disarm_clicks_move_in_opposite_directions() {
    let arm = sound_spec(SoundKind::HudArm).tones;
    let disarm = sound_spec(SoundKind::HudDisarm).tones;

    assert!(
        arm.windows(2)
            .all(|pair| pair[0].freq_hz <= pair[1].freq_hz)
    );
    assert!(
        disarm
            .windows(2)
            .all(|pair| pair[0].freq_hz > pair[1].freq_hz)
    );
}

#[test]
fn hud_side_clicks_share_an_anchor_note_with_opposite_directions() {
    let long = sound_spec(SoundKind::HudSideLong).tones;
    let short = sound_spec(SoundKind::HudSideShort).tones;

    assert_eq!(long[0].freq_hz, short[0].freq_hz);
    assert!(
        long.last()
            .is_some_and(|tone| tone.freq_hz > long[0].freq_hz)
    );
    assert!(
        short
            .last()
            .is_some_and(|tone| tone.freq_hz < short[0].freq_hz)
    );
}

#[test]
fn square_tones_synthesize_clamped_samples_of_expected_length() {
    let spec = sound_spec(SoundKind::HudSizeUp);
    let samples = generate_samples(&spec);

    let expected_len: usize = spec
        .tones
        .iter()
        .map(|tone| (SAMPLE_RATE as u64 * tone.duration_ms / 1000) as usize)
        .sum();
    assert_eq!(samples.len(), expected_len);
    assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
    assert!(samples.iter().any(|sample| sample.abs() > 0.0));
}

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

#[test]
fn custom_wav_read_failure_omits_path_and_custom_error_payload() {
    let error = std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied /home/alice/hud-secret.wav api_key=sound-secret",
    );

    let rendered = worker::custom_wav_read_failure(&error);

    assert_eq!(rendered, "read custom WAV file failed: permission denied");
    assert!(!rendered.contains("/home/alice"));
    assert!(!rendered.contains("sound-secret"));
}
