use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Chart Crosshair Appearance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChartCrosshairStyle {
    #[default]
    Classic,
    Circle,
    Scope,
    Rangefinder,
    Hud,
    Target,
    Rectangle,
    /// Legacy value kept so older saved configs continue to deserialize.
    StackedRectangles,
}

impl ChartCrosshairStyle {
    pub const ALL: [Self; 7] = [
        Self::Classic,
        Self::Circle,
        Self::Scope,
        Self::Rangefinder,
        Self::Hud,
        Self::Target,
        Self::Rectangle,
    ];

    pub fn normalized(self) -> Self {
        match self {
            Self::StackedRectangles => Self::Rectangle,
            style => style,
        }
    }

    pub fn label(self) -> &'static str {
        match self.normalized() {
            Self::Classic => "Classic",
            Self::Circle => "Circle",
            Self::Scope => "Scope",
            Self::Rangefinder => "Rangefinder",
            Self::Hud => "HUD",
            Self::Target => "Target",
            Self::Rectangle => "Rectangle",
            Self::StackedRectangles => unreachable!("legacy crosshair style is normalized"),
        }
    }
}

impl std::fmt::Display for ChartCrosshairStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChartHudOrderSound {
    Off,
    FillTone,
    #[default]
    GunShot8Bit,
    CustomWav,
}

impl ChartHudOrderSound {
    pub const ALL: [Self; 4] = [
        Self::GunShot8Bit,
        Self::FillTone,
        Self::CustomWav,
        Self::Off,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::FillTone => "Fill tone",
            Self::GunShot8Bit => "8-bit shot",
            Self::CustomWav => "Custom WAV",
        }
    }
}

impl std::fmt::Display for ChartHudOrderSound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}
