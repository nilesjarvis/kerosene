use serde::{Deserialize, Serialize};
use std::fmt;

mod outcome_labels;

/// Whether a symbol is a perpetual or spot market.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MarketType {
    Perp,
    Spot,
    Outcome,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
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

impl fmt::Debug for OutcomeSymbolInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutcomeSymbolInfo")
            .field("outcome_id", &self.outcome_id)
            .field("question_id", &self.question_id)
            .field("has_question_name", &self.question_name.is_some())
            .field(
                "has_question_description",
                &self.question_description.is_some(),
            )
            .field("question_class", &self.question_class)
            .field("question_underlying", &self.question_underlying)
            .field("question_expiry", &self.question_expiry)
            .field(
                "question_price_threshold_count",
                &self.question_price_thresholds.len(),
            )
            .field(
                "question_named_outcome_count",
                &self.question_named_outcomes.len(),
            )
            .field(
                "question_settled_named_outcome_count",
                &self.question_settled_named_outcomes.len(),
            )
            .field("question_fallback_outcome", &self.question_fallback_outcome)
            .field("bucket_index", &self.bucket_index)
            .field("is_question_fallback", &self.is_question_fallback)
            .field("side_index", &self.side_index)
            .field("has_side_name", &(!self.side_name.is_empty()))
            .field("has_outcome_name", &(!self.outcome_name.is_empty()))
            .field("has_description", &(!self.description.is_empty()))
            .field("class", &self.class)
            .field("underlying", &self.underlying)
            .field("expiry", &self.expiry)
            .field("has_target_price", &self.target_price.is_some())
            .field("period", &self.period)
            .field("quote_symbol", &self.quote_symbol)
            .field("quote_token_index", &self.quote_token_index)
            .field("encoding", &self.encoding)
            .finish()
    }
}

/// A tradeable symbol on the exchange.
/// `key` is the coin name in the format the candle/book/WS APIs expect:
///   - Main perp dex: "BTC", "ETH", "HYPE"
///   - HIP-3 dexes:   "xyz:NVDA", "flx:BTC", "km:US500"
///   - Spot pairs:     "@107" (HYPE/USDC); pairs the API names directly use
///     that name ("PURR/USDC" is the only such pair today)
///   - Outcomes:       "#0", "#1" (quote-token-denominated prediction contracts)
#[derive(Clone, Serialize, Deserialize, PartialEq)]
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

impl fmt::Debug for ExchangeSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExchangeSymbol")
            .field("key", &self.key)
            .field("ticker", &self.ticker)
            .field("category", &self.category)
            .field("has_display_name", &self.display_name.is_some())
            .field("keyword_count", &self.keywords.len())
            .field("asset_index", &self.asset_index)
            .field("collateral_token", &self.collateral_token)
            .field("sz_decimals", &self.sz_decimals)
            .field("max_leverage", &self.max_leverage)
            .field("only_isolated", &self.only_isolated)
            .field("market_type", &self.market_type)
            .field("has_outcome", &self.outcome.is_some())
            .finish()
    }
}

impl ExchangeSymbol {
    /// Whether a spot pair is quoted in a USD-pegged stable, i.e. whether its
    /// mid can be treated as a USD price. Spot symbols carry their quote in
    /// the "{base}/{quote}" display label; symbols without one (legacy caches
    /// or fixtures) default to the historical USDC assumption. Only
    /// meaningful for `MarketType::Spot` symbols.
    pub fn spot_quote_is_usd_stable(&self) -> bool {
        match self
            .display_name
            .as_deref()
            .and_then(|display| display.rsplit_once('/'))
        {
            Some((_, quote)) => matches!(quote, "USDC" | "USDE" | "USDT0" | "USDH"),
            None => true,
        }
    }

    /// Outcome questions expose fallback settlement contracts that are useful
    /// for metadata, but should not appear as user-selectable markets.
    pub fn is_user_selectable_market(&self) -> bool {
        if self.market_type == MarketType::Outcome && self.outcome.is_none() {
            return false;
        }
        !self
            .outcome
            .as_ref()
            .is_some_and(|info| info.is_question_fallback)
    }
}

/// Resolve a spot symbol from the legacy "@{index}" form of a pair the API
/// names directly (PURR/USDC). Saved layouts, watchlists, and order state may
/// still carry the indexed key those pairs were previously stored under, so
/// key lookups accept it as an alias via the pair's spot asset index.
pub fn spot_symbol_for_indexed_key<'a>(
    symbols: &'a [ExchangeSymbol],
    key: &str,
) -> Option<&'a ExchangeSymbol> {
    let index: u32 = key.strip_prefix('@')?.parse().ok()?;
    let asset_index = 10_000u32.checked_add(index)?;
    symbols
        .iter()
        .find(|symbol| symbol.market_type == MarketType::Spot && symbol.asset_index == asset_index)
}

#[cfg(test)]
mod tests {
    use super::{ExchangeSymbol, MarketType, OutcomeSymbolInfo, spot_symbol_for_indexed_key};

    fn outcome_info() -> OutcomeSymbolInfo {
        OutcomeSymbolInfo {
            outcome_id: 66,
            question_id: Some(12),
            question_name: Some("Will BTC close above the secret threshold?".to_string()),
            question_description: Some("Long question text with raw market details".to_string()),
            question_class: Some("priceBucket".to_string()),
            question_underlying: Some("BTC".to_string()),
            question_expiry: Some("20260520-0600".to_string()),
            question_price_thresholds: vec!["75348.12".to_string(), "78423.45".to_string()],
            question_period: Some("1d".to_string()),
            question_named_outcomes: vec![67, 68, 69],
            question_settled_named_outcomes: vec![67],
            question_fallback_outcome: Some(66),
            bucket_index: Some(2),
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "BTC above threshold".to_string(),
            description: "Outcome contract description".to_string(),
            class: Some("binary".to_string()),
            underlying: Some("BTC".to_string()),
            expiry: Some("20260520-0600".to_string()),
            target_price: Some("75348.12".to_string()),
            period: Some("1d".to_string()),
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 660,
        }
    }

    #[test]
    fn outcome_symbol_debug_summarizes_metadata_payload() {
        let rendered = format!("{:?}", outcome_info());

        assert!(rendered.contains("OutcomeSymbolInfo"));
        assert!(rendered.contains("question_price_threshold_count: 2"));
        assert!(rendered.contains("question_named_outcome_count: 3"));
        assert!(rendered.contains("has_question_name: true"));
        assert!(!rendered.contains("secret threshold"));
        assert!(!rendered.contains("Long question text"));
        assert!(!rendered.contains("75348.12"));
        assert!(!rendered.contains("BTC above threshold"));
        assert!(!rendered.contains("Outcome contract description"));
    }

    #[test]
    fn exchange_symbol_debug_summarizes_outcome_payload() {
        let symbol = ExchangeSymbol {
            key: "#660".to_string(),
            ticker: "#660".to_string(),
            category: "outcome".to_string(),
            display_name: Some("BTC above threshold".to_string()),
            keywords: vec!["btc".to_string(), "threshold".to_string()],
            asset_index: 100_000_000,
            collateral_token: None,
            sz_decimals: 0,
            max_leverage: 1,
            only_isolated: true,
            market_type: MarketType::Outcome,
            outcome: Some(outcome_info()),
        };

        let rendered = format!("{symbol:?}");

        assert!(rendered.contains("ExchangeSymbol"));
        assert!(rendered.contains("key: \"#660\""));
        assert!(rendered.contains("keyword_count: 2"));
        assert!(rendered.contains("has_outcome: true"));
        assert!(!rendered.contains("BTC above threshold"));
        assert!(!rendered.contains("secret threshold"));
        assert!(!rendered.contains("75348.12"));
    }

    fn spot_symbol(key: &str, display_name: Option<&str>, asset_index: u32) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "spot".to_string(),
            display_name: display_name.map(str::to_string),
            keywords: Vec::new(),
            asset_index,
            collateral_token: None,
            sz_decimals: 2,
            max_leverage: 1,
            only_isolated: false,
            market_type: MarketType::Spot,
            outcome: None,
        }
    }

    #[test]
    fn spot_quote_stability_is_derived_from_the_pair_label() {
        for usd_display in ["HYPE/USDC", "FOO/USDT0", "BAR/USDH", "BAZ/USDE"] {
            assert!(
                spot_symbol("@1", Some(usd_display), 10_001).spot_quote_is_usd_stable(),
                "{usd_display} should count as USD-quoted"
            );
        }
        assert!(!spot_symbol("@1", Some("UETH/UBTC"), 10_001).spot_quote_is_usd_stable());
        // Legacy caches and fixtures without a label keep the historical
        // USDC assumption.
        assert!(spot_symbol("@1", None, 10_001).spot_quote_is_usd_stable());
    }

    #[test]
    fn legacy_indexed_key_resolves_api_named_spot_pair_by_asset_index() {
        let symbols = vec![
            spot_symbol("PURR/USDC", Some("PURR/USDC"), 10_000),
            spot_symbol("@107", Some("HYPE/USDC"), 10_107),
        ];

        let resolved = spot_symbol_for_indexed_key(&symbols, "@0")
            .expect("legacy '@0' should resolve the API-named pair");
        assert_eq!(resolved.key, "PURR/USDC");

        assert!(spot_symbol_for_indexed_key(&symbols, "@1").is_none());
        assert!(spot_symbol_for_indexed_key(&symbols, "PURR/USDC").is_none());
        assert!(spot_symbol_for_indexed_key(&symbols, "@bad").is_none());
    }
}
