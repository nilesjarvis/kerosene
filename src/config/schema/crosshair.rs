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
    Target,
    Rectangle,
    /// Legacy value kept so older saved configs continue to deserialize.
    StackedRectangles,
}

impl ChartCrosshairStyle {
    pub const ALL: [Self; 6] = [
        Self::Classic,
        Self::Circle,
        Self::Scope,
        Self::Rangefinder,
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
