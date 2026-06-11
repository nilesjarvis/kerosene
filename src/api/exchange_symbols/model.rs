use serde::{Deserialize, Serialize};

mod outcome_labels;

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

/// A tradeable symbol on the exchange.
/// `key` is the coin name in the format the candle/book/WS APIs expect:
///   - Main perp dex: "BTC", "ETH", "HYPE"
///   - HIP-3 dexes:   "xyz:NVDA", "flx:BTC", "km:US500"
///   - Spot pairs:     "@1" (PURR/USDC), "@107" (HYPE/USDC)
///   - Outcomes:       "#0", "#1" (quote-token-denominated prediction contracts)
#[derive(Debug, Clone, PartialEq)]
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
    /// Outcome-specific metadata for prediction market contracts.
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
