use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Display Denomination Config
// ---------------------------------------------------------------------------

/// Display-only denomination for USD-valued readouts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum DisplayDenominationConfig {
    #[default]
    Usd,
    Asset {
        code: String,
        dex: String,
        symbol: String,
    },
}

impl DisplayDenominationConfig {
    pub fn eur() -> Self {
        Self::Asset {
            code: "EUR".to_string(),
            dex: "xyz".to_string(),
            symbol: "EUR".to_string(),
        }
    }

    pub fn hype() -> Self {
        Self::Asset {
            code: "HYPE".to_string(),
            dex: String::new(),
            symbol: "HYPE".to_string(),
        }
    }

    pub fn btc() -> Self {
        Self::Asset {
            code: "BTC".to_string(),
            dex: String::new(),
            symbol: "BTC".to_string(),
        }
    }

    pub fn normalized(self) -> Self {
        match self {
            Self::Usd => Self::Usd,
            Self::Asset { code, dex, symbol } => {
                let code = code.trim().to_ascii_uppercase();
                let dex = dex.trim().to_ascii_lowercase();
                let symbol = symbol.trim().to_ascii_uppercase();
                if code.is_empty() || symbol.is_empty() {
                    Self::Usd
                } else {
                    Self::Asset { code, dex, symbol }
                }
            }
        }
    }

    pub fn is_usd(&self) -> bool {
        matches!(self, Self::Usd)
    }

    pub fn code(&self) -> &str {
        match self {
            Self::Usd => "USD",
            Self::Asset { code, .. } => code.as_str(),
        }
    }

    pub fn mids_dex(&self) -> Option<&str> {
        match self {
            Self::Usd => None,
            Self::Asset { dex, .. } => Some(dex.as_str()),
        }
    }

    pub fn rate_symbol_key(&self) -> Option<String> {
        match self {
            Self::Usd => None,
            Self::Asset { dex, symbol, .. } if dex.is_empty() => Some(symbol.clone()),
            Self::Asset { dex, symbol, .. } => Some(format!("{dex}:{symbol}")),
        }
    }
}

impl fmt::Display for DisplayDenominationConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
