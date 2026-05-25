use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Market Universe Config
// ---------------------------------------------------------------------------

/// Global market universe shown by the terminal.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum MarketUniverseConfig {
    #[default]
    All,
    Hip3Dex {
        dex: String,
    },
}

impl MarketUniverseConfig {
    pub fn hip3_dex(dex: impl Into<String>) -> Self {
        Self::Hip3Dex { dex: dex.into() }.normalized()
    }

    pub fn normalized(self) -> Self {
        match self {
            Self::All => Self::All,
            Self::Hip3Dex { dex } => {
                let dex = dex.trim().to_ascii_lowercase();
                if dex.is_empty() {
                    Self::All
                } else {
                    Self::Hip3Dex { dex }
                }
            }
        }
    }

    pub fn selected_hip3_dex(&self) -> Option<&str> {
        match self {
            Self::All => None,
            Self::Hip3Dex { dex } => Some(dex.as_str()),
        }
    }
}

impl fmt::Display for MarketUniverseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => f.write_str("All Markets"),
            Self::Hip3Dex { dex } => write!(f, "HIP-3: {dex}"),
        }
    }
}
