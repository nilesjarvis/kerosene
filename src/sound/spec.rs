use super::SoundKind;

pub(super) const SAMPLE_RATE: u32 = 44_100;

#[derive(Debug, Clone, Copy)]
pub(super) struct Tone {
    pub(super) freq_hz: f32,
    pub(super) duration_ms: u64,
    pub(super) amplitude: f32,
}

#[derive(Debug, Clone)]
pub(super) struct SoundSpec {
    pub(super) tones: &'static [Tone],
    pub(super) gap_ms: u64,
    pub(super) fallback_event: &'static str,
}

const FILL_TONES: &[Tone] = &[Tone {
    freq_hz: 880.0,
    duration_ms: 80,
    amplitude: 0.30,
}];

const ERROR_TONES: &[Tone] = &[Tone {
    freq_hz: 330.0,
    duration_ms: 120,
    amplitude: 0.25,
}];

const INTEREST_TONES: &[Tone] = &[
    Tone {
        freq_hz: 740.0,
        duration_ms: 90,
        amplitude: 0.24,
    },
    Tone {
        freq_hz: 988.0,
        duration_ms: 110,
        amplitude: 0.26,
    },
];

pub(super) fn sound_spec(kind: SoundKind) -> SoundSpec {
    match kind {
        SoundKind::Fill => SoundSpec {
            tones: FILL_TONES,
            gap_ms: 0,
            fallback_event: "complete",
        },
        SoundKind::Error => SoundSpec {
            tones: ERROR_TONES,
            gap_ms: 0,
            fallback_event: "dialog-error",
        },
        SoundKind::Interest => SoundSpec {
            tones: INTEREST_TONES,
            gap_ms: 55,
            fallback_event: "message-new-instant",
        },
        SoundKind::HudOrder => SoundSpec {
            tones: FILL_TONES,
            gap_ms: 0,
            fallback_event: "complete",
        },
    }
}
