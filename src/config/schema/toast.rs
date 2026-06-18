use serde::{Deserialize, Deserializer, Serialize};

// ---------------------------------------------------------------------------
// Toast Notification Appearance
// ---------------------------------------------------------------------------

/// Screen corner where toast notifications stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum ToastPosition {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ToastPosition {
    pub const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];

    fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "TopLeft" => Some(Self::TopLeft),
            "TopRight" => Some(Self::TopRight),
            "BottomLeft" => Some(Self::BottomLeft),
            "BottomRight" => Some(Self::BottomRight),
            _ => None,
        }
    }

    fn config_value(self) -> &'static str {
        match self {
            Self::TopLeft => "TopLeft",
            Self::TopRight => "TopRight",
            Self::BottomLeft => "BottomLeft",
            Self::BottomRight => "BottomRight",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TopLeft => "Top Left",
            Self::TopRight => "Top Right",
            Self::BottomLeft => "Bottom Left",
            Self::BottomRight => "Bottom Right",
        }
    }

    /// Whether toasts anchor to the right edge of the window.
    pub fn is_right(self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }

    /// Whether toasts anchor to the bottom edge of the window.
    pub fn is_bottom(self) -> bool {
        matches!(self, Self::BottomLeft | Self::BottomRight)
    }
}

impl<'de> Deserialize<'de> for ToastPosition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_config_value(&value).unwrap_or_else(|| {
            let default = Self::default();
            crate::config::push_config_warning(format!(
                "Unknown toast position {value:?} in config; using {}",
                default.config_value()
            ));
            default
        }))
    }
}

impl std::fmt::Display for ToastPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}
