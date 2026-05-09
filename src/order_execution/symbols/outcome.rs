use crate::api;
use crate::app_state::TradingTerminal;
use crate::helpers;

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

    pub(crate) fn outcome_market_label(info: &api::OutcomeSymbolInfo) -> String {
        if let Some(question_name) = &info.question_name {
            return question_name.clone();
        }
        if info.outcome_name != "Recurring" {
            return info.outcome_name.clone();
        }
        if info.class.as_deref() == Some("priceBinary")
            && let (Some(underlying), Some(target), Some(expiry)) =
                (&info.underlying, &info.target_price, &info.expiry)
        {
            return format!(
                "Will {} be above {} at {}?",
                underlying,
                Self::format_outcome_target_price(target),
                Self::format_outcome_expiry(expiry)
            );
        }

        match (&info.underlying, &info.target_price, &info.expiry) {
            (Some(underlying), Some(target), Some(expiry)) => {
                format!(
                    "{} above {} at {}",
                    underlying,
                    Self::format_outcome_target_price(target),
                    Self::format_outcome_expiry(expiry)
                )
            }
            _ => info.outcome_name.clone(),
        }
    }

    pub(crate) fn format_outcome_target_price(target: &str) -> String {
        let Ok(value) = target.parse::<f64>() else {
            return target.to_string();
        };
        let formatted = helpers::format_with_commas(value);
        formatted
            .strip_suffix(".00")
            .unwrap_or(formatted.as_str())
            .to_string()
    }

    pub(crate) fn format_outcome_expiry(expiry: &str) -> String {
        chrono::NaiveDateTime::parse_from_str(expiry, "%Y%m%d-%H%M")
            .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_else(|_| expiry.to_string())
    }
}
