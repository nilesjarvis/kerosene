use serde::{Deserialize, Serialize};

use super::defaults::default_true;

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
    RacingHud,
    Target,
    Rectangle,
    /// Legacy value kept so older saved configs continue to deserialize.
    StackedRectangles,
}

impl ChartCrosshairStyle {
    pub const CROSSHAIRS: [Self; 6] = [
        Self::Classic,
        Self::Circle,
        Self::Scope,
        Self::Rangefinder,
        Self::Target,
        Self::Rectangle,
    ];

    pub const GAME_HUDS: [Self; 2] = [Self::Hud, Self::RacingHud];

    pub fn normalized(self) -> Self {
        match self {
            Self::StackedRectangles => Self::Rectangle,
            style => style,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Circle => "Circle",
            Self::Scope => "Scope",
            Self::Rangefinder => "Rangefinder",
            Self::Hud => "HUD",
            Self::RacingHud => "Racing HUD",
            Self::Target => "Target",
            Self::Rectangle => "Rectangle",
            Self::StackedRectangles => "Rectangle",
        }
    }

    pub fn is_game_hud(self) -> bool {
        matches!(self.normalized(), Self::Hud | Self::RacingHud)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartHudReadoutConfig {
    #[serde(default = "default_true")]
    pub symbol: bool,
    #[serde(default = "default_true")]
    pub price: bool,
    #[serde(default = "default_true")]
    pub coordinates: bool,
    #[serde(default = "default_true")]
    pub hover_time: bool,
    #[serde(default = "default_true")]
    pub clock: bool,
    #[serde(default = "default_true")]
    pub candle_close: bool,
}

impl Default for ChartHudReadoutConfig {
    fn default() -> Self {
        Self {
            symbol: true,
            price: true,
            coordinates: true,
            hover_time: true,
            clock: true,
            candle_close: true,
        }
    }
}

impl ChartHudReadoutConfig {
    pub fn enabled(self, element: ChartHudReadoutElement) -> bool {
        match element {
            ChartHudReadoutElement::Symbol => self.symbol,
            ChartHudReadoutElement::Price => self.price,
            ChartHudReadoutElement::Coordinates => self.coordinates,
            ChartHudReadoutElement::HoverTime => self.hover_time,
            ChartHudReadoutElement::Clock => self.clock,
            ChartHudReadoutElement::CandleClose => self.candle_close,
        }
    }

    pub fn set(&mut self, element: ChartHudReadoutElement, enabled: bool) {
        match element {
            ChartHudReadoutElement::Symbol => self.symbol = enabled,
            ChartHudReadoutElement::Price => self.price = enabled,
            ChartHudReadoutElement::Coordinates => self.coordinates = enabled,
            ChartHudReadoutElement::HoverTime => self.hover_time = enabled,
            ChartHudReadoutElement::Clock => self.clock = enabled,
            ChartHudReadoutElement::CandleClose => self.candle_close = enabled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartHudReadoutElement {
    Symbol,
    Price,
    Coordinates,
    HoverTime,
    Clock,
    CandleClose,
}

impl ChartHudReadoutElement {
    pub const ALL: [Self; 6] = [
        Self::Symbol,
        Self::Price,
        Self::Coordinates,
        Self::HoverTime,
        Self::Clock,
        Self::CandleClose,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Symbol => "Symbol + timeframe",
            Self::Price => "Hover price",
            Self::Coordinates => "Cursor coordinates",
            Self::HoverTime => "Hover time",
            Self::Clock => "Current clock",
            Self::CandleClose => "Candle close countdown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChartCrosshairStyle;

    #[test]
    fn racing_hud_is_selectable_game_hud_style() {
        assert!(ChartCrosshairStyle::GAME_HUDS.contains(&ChartCrosshairStyle::RacingHud));
        assert!(
            !ChartCrosshairStyle::CROSSHAIRS
                .iter()
                .any(|style| style.is_game_hud())
        );
        assert!(ChartCrosshairStyle::RacingHud.is_game_hud());
    }

    #[test]
    fn legacy_stacked_rectangles_labels_and_displays_as_rectangle() {
        assert_eq!(
            ChartCrosshairStyle::StackedRectangles.normalized(),
            ChartCrosshairStyle::Rectangle
        );
        assert_eq!(ChartCrosshairStyle::StackedRectangles.label(), "Rectangle");
        assert_eq!(
            ChartCrosshairStyle::StackedRectangles.to_string(),
            "Rectangle"
        );
    }
}
