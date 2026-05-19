use crate::api::MarketType;
use crate::app_state::TradingTerminal;

pub(crate) const OUTCOME_MIN_PRICE: f64 = 0.001;
pub(crate) const OUTCOME_MAX_PRICE: f64 = 0.999;

impl TradingTerminal {
    pub(crate) fn outcome_read_only_status(&mut self, action: &str) {
        self.order_status = Some((
            format!(
                "Outcome {action} is not available from this control; use the main order ticket"
            ),
            true,
        ));
    }

    pub(crate) fn outcome_balance_coin_to_trade_coin(coin: &str) -> Option<String> {
        coin.strip_prefix('+')
            .map(|encoding| format!("#{encoding}"))
    }

    pub(crate) fn outcome_trade_coin_for_balance_coin(&self, coin: &str) -> Option<String> {
        let trade_coin = Self::outcome_balance_coin_to_trade_coin(coin)?;
        self.exchange_symbols
            .iter()
            .any(|symbol| {
                symbol.key == trade_coin
                    && symbol.market_type == crate::api::MarketType::Outcome
                    && symbol.is_user_selectable_market()
            })
            .then_some(trade_coin)
    }

    pub(crate) fn display_coin_for_spot_balance(&self, coin: &str) -> String {
        if let Some(symbol) = self
            .exchange_symbols
            .iter()
            .find(|symbol| symbol.key == coin && symbol.market_type == MarketType::Spot)
        {
            return Self::exchange_symbol_display_name(symbol);
        }

        self.outcome_trade_coin_for_balance_coin(coin)
            .map(|trade_coin| format!("{} ({coin})", self.display_name_for_symbol(&trade_coin)))
            .unwrap_or_else(|| coin.to_string())
    }

    pub(crate) fn validate_outcome_order_price(price: f64) -> Result<(), String> {
        if price.is_finite() && (OUTCOME_MIN_PRICE..=OUTCOME_MAX_PRICE).contains(&price) {
            Ok(())
        } else {
            Err(format!(
                "Outcome prices must be between {OUTCOME_MIN_PRICE:.3} and {OUTCOME_MAX_PRICE:.3}"
            ))
        }
    }

    pub(crate) fn clamp_outcome_market_price(price: f64) -> f64 {
        price.clamp(OUTCOME_MIN_PRICE, OUTCOME_MAX_PRICE)
    }

    pub(crate) fn validate_outcome_contract_size(&self, qty: f64) -> Result<(), String> {
        if qty.fract().abs() <= 1e-9 {
            Ok(())
        } else {
            Err("Outcome orders require whole-contract sizes".to_string())
        }
    }

    pub(crate) fn sanitize_outcome_quantity_input(input: &str) -> String {
        let mut out = String::new();
        for ch in input.chars() {
            if ch.is_ascii_digit() {
                out.push(ch);
            } else if ch == '.' || ch == ',' {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{OUTCOME_MAX_PRICE, OUTCOME_MIN_PRICE};
    use crate::api;
    use crate::app_state::TradingTerminal;
    use chrono::TimeZone;

    fn outcome_info() -> api::OutcomeSymbolInfo {
        api::OutcomeSymbolInfo {
            outcome_id: 65,
            question_id: None,
            question_name: None,
            question_description: None,
            question_class: None,
            question_underlying: None,
            question_expiry: None,
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: Vec::new(),
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: None,
            bucket_index: None,
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring".to_string(),
            description: "class:priceBinary|underlying:BTC".to_string(),
            class: Some("priceBinary".to_string()),
            underlying: Some("BTC".to_string()),
            expiry: Some("20260520-0600".to_string()),
            target_price: Some("76886".to_string()),
            period: Some("1d".to_string()),
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(150),
            encoding: 650,
        }
    }

    fn spot_symbol(key: &str, ticker: &str, display: &str) -> api::ExchangeSymbol {
        api::ExchangeSymbol {
            key: key.to_string(),
            ticker: ticker.to_string(),
            category: "spot".to_string(),
            display_name: Some(display.to_string()),
            keywords: Vec::new(),
            asset_index: 10107,
            collateral_token: None,
            sz_decimals: 2,
            max_leverage: 1,
            only_isolated: false,
            market_type: api::MarketType::Spot,
            outcome: None,
        }
    }

    fn utc_ms(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> u64 {
        chrono::Utc
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .expect("valid utc timestamp")
            .timestamp_millis() as u64
    }

    #[test]
    fn binary_outcome_label_describes_threshold_and_expiry() {
        assert_eq!(
            outcome_info().market_label(),
            "BTC is above 76,886 at 2026-05-20 06:00 UTC"
        );
    }

    #[test]
    fn bucket_outcome_label_describes_price_range() {
        let mut info = outcome_info();
        info.question_class = Some("priceBucket".to_string());
        info.question_underlying = Some("BTC".to_string());
        info.question_expiry = Some("20260520-0600".to_string());
        info.question_price_thresholds = vec!["75348".to_string(), "78423".to_string()];

        info.bucket_index = Some(0);
        assert_eq!(
            info.market_label(),
            "BTC is below 75,348 at 2026-05-20 06:00 UTC"
        );

        info.bucket_index = Some(1);
        assert_eq!(
            info.market_label(),
            "BTC is at or above 75,348 and below 78,423 at 2026-05-20 06:00 UTC"
        );

        info.bucket_index = Some(2);
        assert_eq!(
            info.market_label(),
            "BTC is at or above 78,423 at 2026-05-20 06:00 UTC"
        );
    }

    #[test]
    fn no_side_outcome_label_describes_payoff_condition() {
        let mut info = outcome_info();
        info.side_index = 1;
        info.side_name = "No".to_string();

        assert_eq!(
            info.display_label(),
            "NO: BTC is at or below 76,886 at 2026-05-20 06:00 UTC"
        );

        info.question_class = Some("priceBucket".to_string());
        info.question_underlying = Some("BTC".to_string());
        info.question_expiry = Some("20260520-0600".to_string());
        info.question_price_thresholds = vec!["75348".to_string(), "78423".to_string()];
        info.bucket_index = Some(1);

        assert_eq!(
            info.display_label(),
            "NO: BTC is below 75,348 or at or above 78,423 at 2026-05-20 06:00 UTC"
        );
    }

    #[test]
    fn short_side_condition_label_omits_expiry_details() {
        let mut info = outcome_info();

        assert_eq!(info.side_condition_short_label(), "BTC is above 76,886");

        info.side_index = 1;
        info.side_name = "No".to_string();
        assert_eq!(
            info.side_condition_short_label(),
            "BTC is at or below 76,886"
        );

        info.question_class = Some("priceBucket".to_string());
        info.question_underlying = Some("BTC".to_string());
        info.question_expiry = Some("20260520-0600".to_string());
        info.question_price_thresholds = vec!["75348".to_string(), "78423".to_string()];
        info.bucket_index = Some(1);

        assert_eq!(
            info.side_condition_short_label(),
            "BTC is below 75,348 or at or above 78,423"
        );
    }

    #[test]
    fn countdown_label_uses_current_user_clock_distance() {
        let info = outcome_info();
        let now_ms = utc_ms(2026, 5, 19, 13, 45);

        assert_eq!(
            info.side_condition_label_with_countdown(now_ms),
            "BTC is above 76,886 at 2026-05-20 06:00 UTC (16h 15m left)"
        );
    }

    #[test]
    fn countdown_label_marks_expired_markets() {
        let info = outcome_info();
        let now_ms = utc_ms(2026, 5, 20, 6, 1);

        assert_eq!(
            info.side_condition_label_with_countdown(now_ms),
            "BTC is above 76,886 at 2026-05-20 06:00 UTC (expired)"
        );
    }

    #[test]
    fn fallback_outcome_label_is_explicit() {
        let mut info = outcome_info();
        info.question_class = Some("priceBucket".to_string());
        info.question_underlying = Some("BTC".to_string());
        info.question_expiry = Some("20260520-0600".to_string());
        info.is_question_fallback = true;

        assert_eq!(
            info.market_label(),
            "fallback / other settlement at 2026-05-20 06:00 UTC"
        );
    }

    #[test]
    fn outcome_balance_coin_maps_to_trade_coin() {
        assert_eq!(
            TradingTerminal::outcome_balance_coin_to_trade_coin("+650"),
            Some("#650".to_string())
        );
        assert_eq!(
            TradingTerminal::outcome_balance_coin_to_trade_coin("#650"),
            None
        );
    }

    #[test]
    fn display_coin_for_spot_balance_uses_spot_pair_display() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![spot_symbol("@107", "HYPE", "HYPE/USDC")];

        assert_eq!(terminal.display_coin_for_spot_balance("@107"), "HYPE/USDC");
        assert_eq!(terminal.display_coin_for_spot_balance("HYPE"), "HYPE");
    }

    #[test]
    fn outcome_price_validation_uses_probability_bounds() {
        assert!(TradingTerminal::validate_outcome_order_price(OUTCOME_MIN_PRICE).is_ok());
        assert!(TradingTerminal::validate_outcome_order_price(OUTCOME_MAX_PRICE).is_ok());
        assert!(TradingTerminal::validate_outcome_order_price(0.0009).is_err());
        assert!(TradingTerminal::validate_outcome_order_price(0.9991).is_err());
        assert!(TradingTerminal::validate_outcome_order_price(f64::NAN).is_err());
    }

    #[test]
    fn outcome_market_price_clamps_to_probability_bounds() {
        assert_eq!(
            TradingTerminal::clamp_outcome_market_price(0.0),
            OUTCOME_MIN_PRICE
        );
        assert_eq!(
            TradingTerminal::clamp_outcome_market_price(1.0),
            OUTCOME_MAX_PRICE
        );
        assert_eq!(TradingTerminal::clamp_outcome_market_price(0.42), 0.42);
    }
}
