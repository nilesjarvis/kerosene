use crate::app_state::TradingTerminal;

impl TradingTerminal {
    pub(crate) fn outcome_read_only_status(&mut self, action: &str) {
        self.order_status = Some((format!("Outcome {action} is read-only in this build"), true));
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
    use crate::api;
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
}
