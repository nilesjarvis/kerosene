use crate::helpers;
use serde::{Deserialize, Serialize};

/// Whether a symbol is a perpetual or spot market.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketType {
    Perp,
    Spot,
    Outcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeSymbolInfo {
    pub outcome_id: u32,
    pub question_id: Option<u32>,
    pub question_name: Option<String>,
    pub question_description: Option<String>,
    pub question_class: Option<String>,
    pub question_underlying: Option<String>,
    pub question_expiry: Option<String>,
    pub question_price_thresholds: Vec<String>,
    pub question_period: Option<String>,
    pub question_named_outcomes: Vec<u32>,
    pub question_settled_named_outcomes: Vec<u32>,
    pub question_fallback_outcome: Option<u32>,
    pub bucket_index: Option<u32>,
    pub is_question_fallback: bool,
    pub side_index: u32,
    pub side_name: String,
    pub outcome_name: String,
    pub description: String,
    pub class: Option<String>,
    pub underlying: Option<String>,
    pub expiry: Option<String>,
    pub target_price: Option<String>,
    pub period: Option<String>,
    pub quote_symbol: String,
    pub quote_token_index: Option<u32>,
    pub encoding: u32,
}

impl OutcomeSymbolInfo {
    pub fn market_label(&self) -> String {
        self.market_label_at(None, true)
    }

    pub fn market_label_with_countdown(&self, now_ms: u64) -> String {
        self.market_label_at(Some(now_ms), true)
    }

    fn market_label_at(&self, now_ms: Option<u64>, include_expiry: bool) -> String {
        if self.question_class.as_deref() == Some("priceBucket") {
            return self.bucket_event_label(now_ms, include_expiry);
        }
        if self.class.as_deref() == Some("priceBinary")
            && let (Some(underlying), Some(target)) = (&self.underlying, &self.target_price)
        {
            let label = Self::price_threshold_label(underlying, target, true);
            if !include_expiry {
                return label;
            }
            let Some(expiry) = &self.expiry else {
                return label;
            };
            return format!("{label} at {}", Self::format_expiry_at(expiry, now_ms));
        }

        if let Some(question_name) = &self.question_name {
            return question_name.clone();
        }
        if self.outcome_name != "Recurring" {
            return self.outcome_name.clone();
        }
        match (&self.underlying, &self.target_price, &self.expiry) {
            (Some(underlying), Some(target), expiry) => {
                let label = Self::price_threshold_label(underlying, target, true);
                if include_expiry && let Some(expiry) = expiry {
                    format!("{label} at {}", Self::format_expiry_at(expiry, now_ms))
                } else {
                    label
                }
            }
            _ => self.outcome_name.clone(),
        }
    }

    pub fn display_label(&self) -> String {
        format!(
            "{}: {}",
            self.side_name.to_ascii_uppercase(),
            self.side_condition_label()
        )
    }

    pub fn side_condition_label(&self) -> String {
        self.side_condition_label_at(None, true)
    }

    pub fn side_condition_label_with_countdown(&self, now_ms: u64) -> String {
        self.side_condition_label_at(Some(now_ms), true)
    }

    pub fn side_condition_short_label(&self) -> String {
        self.side_condition_label_at(None, false)
    }

    fn side_condition_label_at(&self, now_ms: Option<u64>, include_expiry: bool) -> String {
        if self.is_no_side() {
            return self.complement_label(now_ms, include_expiry);
        }
        self.market_label_at(now_ms, include_expiry)
    }

    pub fn format_target_price(target: &str) -> String {
        let Ok(value) = target.parse::<f64>() else {
            return target.to_string();
        };
        let formatted = helpers::format_with_commas(value);
        formatted
            .strip_suffix(".00")
            .unwrap_or(formatted.as_str())
            .to_string()
    }

    fn format_expiry_at(expiry: &str, now_ms: Option<u64>) -> String {
        chrono::NaiveDateTime::parse_from_str(expiry, "%Y%m%d-%H%M")
            .map(|dt| {
                let expiry_label = dt.format("%Y-%m-%d %H:%M UTC").to_string();
                now_ms
                    .and_then(|now_ms| expiry_countdown_label(expiry, now_ms))
                    .map(|countdown| format!("{expiry_label} ({countdown})"))
                    .unwrap_or(expiry_label)
            })
            .unwrap_or_else(|_| expiry.to_string())
    }

    fn bucket_event_label(&self, now_ms: Option<u64>, include_expiry: bool) -> String {
        self.bucket_label_for_side(true, now_ms, include_expiry)
    }

    fn complement_label(&self, now_ms: Option<u64>, include_expiry: bool) -> String {
        if self.question_class.as_deref() == Some("priceBucket") {
            return self.bucket_label_for_side(false, now_ms, include_expiry);
        }
        if self.class.as_deref() == Some("priceBinary")
            && let (Some(underlying), Some(target)) = (&self.underlying, &self.target_price)
        {
            let label = Self::price_threshold_label(underlying, target, false);
            if !include_expiry {
                return label;
            }
            let Some(expiry) = &self.expiry else {
                return label;
            };
            return format!("{label} at {}", Self::format_expiry_at(expiry, now_ms));
        }

        format!("not {}", self.market_label_at(now_ms, include_expiry))
    }

    fn bucket_label_for_side(
        &self,
        affirmative: bool,
        now_ms: Option<u64>,
        include_expiry: bool,
    ) -> String {
        if self.is_question_fallback {
            return self.fallback_label_for_side(affirmative, now_ms, include_expiry);
        }

        let Some(index) = self.bucket_index.map(|index| index as usize) else {
            return self
                .question_name
                .clone()
                .unwrap_or_else(|| self.outcome_name.clone());
        };
        let Some(underlying) = self.question_underlying.as_deref() else {
            return format!("Bucket {}", index + 1);
        };
        let thresholds = &self.question_price_thresholds;
        if thresholds.is_empty() {
            return format!("Bucket {}", index + 1);
        }

        let expiry = self
            .question_expiry
            .as_ref()
            .filter(|_| include_expiry)
            .map(|expiry| Self::format_expiry_at(expiry, now_ms));
        let with_expiry = |label: String| match &expiry {
            Some(expiry) => format!("{label} at {expiry}"),
            None => label,
        };

        if index == 0 {
            let threshold = Self::format_target_price(&thresholds[0]);
            let label = if affirmative {
                format!("{underlying} is below {threshold}")
            } else {
                format!("{underlying} is at or above {threshold}")
            };
            return with_expiry(label);
        }

        if index < thresholds.len() {
            let lower = Self::format_target_price(&thresholds[index - 1]);
            let upper = Self::format_target_price(&thresholds[index]);
            if affirmative {
                return with_expiry(format!(
                    "{underlying} is at or above {lower} and below {upper}"
                ));
            }
            return with_expiry(format!(
                "{underlying} is below {lower} or at or above {upper}"
            ));
        }

        let threshold =
            Self::format_target_price(thresholds.last().map(String::as_str).unwrap_or(""));
        let label = if affirmative {
            format!("{underlying} is at or above {threshold}")
        } else {
            format!("{underlying} is below {threshold}")
        };
        with_expiry(label)
    }

    fn fallback_label_for_side(
        &self,
        affirmative: bool,
        now_ms: Option<u64>,
        include_expiry: bool,
    ) -> String {
        let expiry = self
            .question_expiry
            .as_ref()
            .filter(|_| include_expiry)
            .map(|expiry| Self::format_expiry_at(expiry, now_ms));
        let label = if affirmative {
            "fallback / other settlement".to_string()
        } else if let Some(underlying) = self.question_underlying.as_deref() {
            format!("a named {underlying} bucket settles")
        } else {
            "a named outcome settles".to_string()
        };

        match expiry {
            Some(expiry) => format!("{label} at {expiry}"),
            None => label,
        }
    }

    fn is_no_side(&self) -> bool {
        self.side_name.trim().eq_ignore_ascii_case("no") || self.side_index == 1
    }

    fn price_threshold_label(underlying: &str, target: &str, affirmative: bool) -> String {
        let target = Self::format_target_price(target);
        if affirmative {
            format!("{underlying} is above {target}")
        } else {
            format!("{underlying} is at or below {target}")
        }
    }
}

fn expiry_countdown_label(expiry: &str, now_ms: u64) -> Option<String> {
    let expiry_ms = chrono::NaiveDateTime::parse_from_str(expiry, "%Y%m%d-%H%M")
        .ok()?
        .and_utc()
        .timestamp_millis();
    let now_ms = i64::try_from(now_ms).ok()?;
    let diff_ms = expiry_ms.saturating_sub(now_ms);
    if diff_ms <= 0 {
        return Some("expired".to_string());
    }

    Some(format!("{} left", helpers::format_duration(diff_ms as u64)))
}

/// A tradeable symbol on the exchange.
/// `key` is the coin name in the format the candle/book/WS APIs expect:
///   - Main perp dex: "BTC", "ETH", "HYPE"
///   - HIP-3 dexes:   "xyz:NVDA", "flx:BTC", "km:US500"
///   - Spot pairs:     "@1" (PURR/USDC), "@107" (HYPE/USDC)
///   - Outcomes:       "#0", "#1" (USDH-denominated prediction contracts)
#[derive(Debug, Clone)]
pub struct ExchangeSymbol {
    /// Coin name for API calls (e.g. "HYPE", "xyz:NVDA", or "@107")
    pub key: String,
    /// Short ticker shown in the UI (e.g. "NVDA", "HYPE", "PURR")
    pub ticker: String,
    /// Category: crypto, stocks, commodities, indices, fx, preipo, spot, etc.
    pub category: String,
    /// Optional display name override (e.g. "S&P500" for "SP500", "PURR/USDC" for spot)
    pub display_name: Option<String>,
    /// Optional search keywords (e.g. ["nvidia", "ai"])
    pub keywords: Vec<String>,
    /// Asset index for the exchange API order placement.
    /// Main dex: 0-based index in the universe array.
    /// Builder dexes: 110000 + (dex_idx - 1) * 10000 + asset_idx.
    /// Spot pairs: 10000 + spot universe index.
    pub asset_index: u32,
    /// Collateral token index for the perp DEX. None for non-perp markets.
    pub collateral_token: Option<u32>,
    /// Number of decimal places for the size field.
    pub sz_decimals: u32,
    /// Maximum allowed leverage for this asset (perps only).
    pub max_leverage: u32,
    /// Whether cross margin is completely disallowed on this asset.
    pub only_isolated: bool,
    /// Whether this is a perpetual, spot, or outcome market.
    pub market_type: MarketType,
    /// Outcome-specific metadata for USDH prediction market contracts.
    pub outcome: Option<OutcomeSymbolInfo>,
}

impl ExchangeSymbol {
    /// Outcome questions expose fallback settlement contracts that are useful
    /// for metadata, but should not appear as user-selectable markets.
    pub fn is_user_selectable_market(&self) -> bool {
        !self
            .outcome
            .as_ref()
            .is_some_and(|info| info.is_question_fallback)
    }
}
