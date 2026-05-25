use super::OutcomeSymbolInfo;
use crate::helpers;

impl OutcomeSymbolInfo {
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

    pub(super) fn price_threshold_label(
        underlying: &str,
        target: &str,
        affirmative: bool,
    ) -> String {
        let target = Self::format_target_price(target);
        if affirmative {
            format!("{underlying} is above {target}")
        } else {
            format!("{underlying} is at or below {target}")
        }
    }
}
