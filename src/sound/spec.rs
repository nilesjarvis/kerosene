use super::SoundKind;

pub(super) const SAMPLE_RATE: u32 = 44_100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Waveform {
    Sine,
    /// Square wave for the chip-tune "8-bit" HUD interface clicks.
    Square,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Tone {
    pub(super) freq_hz: f32,
    pub(super) duration_ms: u64,
    pub(super) amplitude: f32,
    pub(super) waveform: Waveform,
}

#[derive(Debug, Clone)]
pub(super) struct SoundSpec {
    pub(super) tones: &'static [Tone],
    pub(super) gap_ms: u64,
    /// System notification event used when local audio output is unavailable.
    /// `None` keeps cosmetic UI clicks silent instead of spamming OS sounds.
    pub(super) fallback_event: Option<&'static str>,
}

const FILL_TONES: &[Tone] = &[Tone {
    freq_hz: 880.0,
    duration_ms: 80,
    amplitude: 0.30,
    waveform: Waveform::Sine,
}];

const ERROR_TONES: &[Tone] = &[Tone {
    freq_hz: 330.0,
    duration_ms: 120,
    amplitude: 0.25,
    waveform: Waveform::Sine,
}];

const INTEREST_TONES: &[Tone] = &[
    Tone {
        freq_hz: 740.0,
        duration_ms: 90,
        amplitude: 0.24,
        waveform: Waveform::Sine,
    },
    Tone {
        freq_hz: 988.0,
        duration_ms: 110,
        amplitude: 0.26,
        waveform: Waveform::Sine,
    },
];

// HUD interface clicks: square-wave "fire selector" vocabulary. Mode pairs
// mirror each other (rising = market, falling = limit), side pairs share a
// 659 Hz anchor note with direction encoding the side, and the arm sequence
// is deliberately the longest so going hot never blends into key noise.
const HUD_MODE_LIMIT_TONES: &[Tone] = &[
    Tone {
        freq_hz: 987.77,
        duration_ms: 28,
        amplitude: 0.22,
        waveform: Waveform::Square,
    },
    Tone {
        freq_hz: 783.99,
        duration_ms: 42,
        amplitude: 0.20,
        waveform: Waveform::Square,
    },
];

const HUD_MODE_MARKET_TONES: &[Tone] = &[
    Tone {
        freq_hz: 783.99,
        duration_ms: 28,
        amplitude: 0.22,
        waveform: Waveform::Square,
    },
    Tone {
        freq_hz: 987.77,
        duration_ms: 42,
        amplitude: 0.24,
        waveform: Waveform::Square,
    },
];

const HUD_SIDE_LONG_TONES: &[Tone] = &[
    Tone {
        freq_hz: 659.25,
        duration_ms: 30,
        amplitude: 0.20,
        waveform: Waveform::Square,
    },
    Tone {
        freq_hz: 987.77,
        duration_ms: 30,
        amplitude: 0.20,
        waveform: Waveform::Square,
    },
];

const HUD_SIDE_SHORT_TONES: &[Tone] = &[
    Tone {
        freq_hz: 659.25,
        duration_ms: 30,
        amplitude: 0.20,
        waveform: Waveform::Square,
    },
    Tone {
        freq_hz: 440.0,
        duration_ms: 30,
        amplitude: 0.20,
        waveform: Waveform::Square,
    },
];

const HUD_ARM_TONES: &[Tone] = &[
    Tone {
        freq_hz: 523.25,
        duration_ms: 30,
        amplitude: 0.26,
        waveform: Waveform::Square,
    },
    Tone {
        freq_hz: 659.25,
        duration_ms: 30,
        amplitude: 0.27,
        waveform: Waveform::Square,
    },
    Tone {
        freq_hz: 880.0,
        duration_ms: 30,
        amplitude: 0.28,
        waveform: Waveform::Square,
    },
    Tone {
        freq_hz: 880.0,
        duration_ms: 60,
        amplitude: 0.18,
        waveform: Waveform::Sine,
    },
];

const HUD_DISARM_TONES: &[Tone] = &[
    Tone {
        freq_hz: 880.0,
        duration_ms: 35,
        amplitude: 0.20,
        waveform: Waveform::Square,
    },
    Tone {
        freq_hz: 523.25,
        duration_ms: 70,
        amplitude: 0.17,
        waveform: Waveform::Square,
    },
];

// Sine (not square) so the system holstering the weapon sounds advisory and
// is never confused with a manual disarm at the keyboard.
const HUD_AUTO_DISARM_TONES: &[Tone] = &[
    Tone {
        freq_hz: 659.25,
        duration_ms: 45,
        amplitude: 0.20,
        waveform: Waveform::Sine,
    },
    Tone {
        freq_hz: 523.25,
        duration_ms: 45,
        amplitude: 0.19,
        waveform: Waveform::Sine,
    },
    Tone {
        freq_hz: 392.0,
        duration_ms: 60,
        amplitude: 0.18,
        waveform: Waveform::Sine,
    },
];

const HUD_IDLE_WARNING_TONES: &[Tone] = &[
    Tone {
        freq_hz: 1174.66,
        duration_ms: 40,
        amplitude: 0.18,
        waveform: Waveform::Sine,
    },
    Tone {
        freq_hz: 1174.66,
        duration_ms: 40,
        amplitude: 0.18,
        waveform: Waveform::Sine,
    },
];

const HUD_SIZE_UP_TONES: &[Tone] = &[Tone {
    freq_hz: 1318.51,
    duration_ms: 12,
    amplitude: 0.10,
    waveform: Waveform::Square,
}];

const HUD_SIZE_DOWN_TONES: &[Tone] = &[Tone {
    freq_hz: 1079.0,
    duration_ms: 12,
    amplitude: 0.10,
    waveform: Waveform::Square,
}];

pub(super) fn sound_spec(kind: SoundKind) -> SoundSpec {
    match kind {
        SoundKind::Fill => SoundSpec {
            tones: FILL_TONES,
            gap_ms: 0,
            fallback_event: Some("complete"),
        },
        SoundKind::Error => SoundSpec {
            tones: ERROR_TONES,
            gap_ms: 0,
            fallback_event: Some("dialog-error"),
        },
        SoundKind::Interest => SoundSpec {
            tones: INTEREST_TONES,
            gap_ms: 55,
            fallback_event: Some("message-new-instant"),
        },
        SoundKind::HudOrder => SoundSpec {
            tones: FILL_TONES,
            gap_ms: 0,
            fallback_event: Some("complete"),
        },
        SoundKind::HudModeLimit => SoundSpec {
            tones: HUD_MODE_LIMIT_TONES,
            gap_ms: 6,
            fallback_event: None,
        },
        SoundKind::HudModeMarket => SoundSpec {
            tones: HUD_MODE_MARKET_TONES,
            gap_ms: 6,
            fallback_event: None,
        },
        SoundKind::HudSideLong => SoundSpec {
            tones: HUD_SIDE_LONG_TONES,
            gap_ms: 5,
            fallback_event: None,
        },
        SoundKind::HudSideShort => SoundSpec {
            tones: HUD_SIDE_SHORT_TONES,
            gap_ms: 5,
            fallback_event: None,
        },
        SoundKind::HudArm => SoundSpec {
            tones: HUD_ARM_TONES,
            gap_ms: 8,
            fallback_event: None,
        },
        SoundKind::HudDisarm => SoundSpec {
            tones: HUD_DISARM_TONES,
            gap_ms: 8,
            fallback_event: None,
        },
        SoundKind::HudAutoDisarm => SoundSpec {
            tones: HUD_AUTO_DISARM_TONES,
            gap_ms: 25,
            fallback_event: None,
        },
        SoundKind::HudIdleWarning => SoundSpec {
            tones: HUD_IDLE_WARNING_TONES,
            gap_ms: 120,
            fallback_event: None,
        },
        SoundKind::HudSizeUp => SoundSpec {
            tones: HUD_SIZE_UP_TONES,
            gap_ms: 0,
            fallback_event: None,
        },
        SoundKind::HudSizeDown => SoundSpec {
            tones: HUD_SIZE_DOWN_TONES,
            gap_ms: 0,
            fallback_event: None,
        },
    }
}
