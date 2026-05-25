/// Order type selected by the order entry UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderKind {
    Market,
    Limit,
    Chase,
    Twap,
    LimitIoc,
}

impl OrderKind {
    pub(crate) fn config_str(self) -> &'static str {
        match self {
            Self::Market => "Market",
            Self::Limit => "Limit",
            Self::Chase => "Chase",
            Self::Twap => "TWAP",
            Self::LimitIoc => "Limit IOC",
        }
    }

    pub(crate) fn from_config_str(value: &str) -> Self {
        match value {
            "Market" => Self::Market,
            "Chase" => Self::Chase,
            "TWAP" | "Twap" => Self::Twap,
            "Limit IOC" | "LimitIoc" | "IOC" => Self::LimitIoc,
            _ => Self::Limit,
        }
    }
}
