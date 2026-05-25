use super::DisplayDenominationContext;
use crate::helpers::{
    format_decimal_with_commas, format_price, format_usd, invalid_data_placeholder,
};

// ---------------------------------------------------------------------------
// Display Denomination Formatting
// ---------------------------------------------------------------------------

impl DisplayDenominationContext {
    pub(crate) fn active_symbol(&self) -> &str {
        currency_symbol(self.active_code()).unwrap_or_else(|| self.active_code())
    }

    pub(crate) fn format_active_amount(&self, sign: &str, amount: impl AsRef<str>) -> String {
        let amount = amount.as_ref();
        if let Some(symbol) = currency_symbol(self.active_code()) {
            format!("{sign}{symbol}{amount}")
        } else {
            format!("{sign}{amount} {}", self.active_code())
        }
    }

    pub(crate) fn format_usd_str(&self, raw: &str) -> String {
        let Ok(value) = raw.parse::<f64>() else {
            return raw.to_string();
        };
        self.format_value(value, 2)
    }

    pub(crate) fn format_value(&self, usd_value: f64, decimals: usize) -> String {
        let Some(value) = self.convert_usd_value(usd_value) else {
            return invalid_data_placeholder();
        };
        let sign = if value < 0.0 { "-" } else { "" };
        let abs = value.abs();

        if self.active_code() == "USD" {
            return format_usd(&format!("{value:.decimals$}"));
        }

        self.format_active_amount(sign, format_decimal_with_commas(abs, decimals))
    }

    pub(crate) fn format_price(&self, usd_price: f64) -> String {
        let Some(value) = self.convert_usd_value(usd_price) else {
            return invalid_data_placeholder();
        };
        format_price(value)
    }

    pub(crate) fn format_chart_price(&self, usd_price: f64) -> String {
        let Some(value) = self.convert_usd_value(usd_price) else {
            return invalid_data_placeholder();
        };
        if !self.includes_usd_chart_reference() {
            return format_price(value);
        }

        format!(
            "{} ({})",
            self.format_active_price(value),
            format_symbol_price("$", usd_price)
        )
    }

    pub(crate) fn includes_usd_chart_reference(&self) -> bool {
        self.active_code() != "USD"
    }

    pub(crate) fn format_signed_value(&self, usd_value: f64, decimals: usize) -> String {
        let Some(value) = self.convert_usd_value(usd_value) else {
            return invalid_data_placeholder();
        };
        let display_value = value.abs();
        let sign = if value > 0.0 {
            "+"
        } else if value < 0.0 {
            "-"
        } else {
            ""
        };
        self.format_active_amount(sign, format_decimal_with_commas(display_value, decimals))
    }

    #[cfg(test)]
    pub(crate) fn format_signed_compact_value(&self, usd_value: f64) -> String {
        let Some(value) = self.convert_usd_value(usd_value) else {
            return invalid_data_placeholder();
        };
        let sign = if value > 0.0 {
            "+"
        } else if value < 0.0 {
            "-"
        } else {
            ""
        };
        self.format_active_amount(sign, format_compact(value.abs()))
    }

    pub(crate) fn hidden_mask(&self) -> String {
        self.format_active_amount("", "***")
    }

    fn format_active_price(&self, value: f64) -> String {
        if let Some(symbol) = currency_symbol(self.active_code()) {
            return format_symbol_price(symbol, value);
        }
        let sign = if value < 0.0 { "-" } else { "" };
        format!("{sign}{} {}", format_price(value.abs()), self.active_code())
    }
}

pub(crate) fn format_compact(value: f64) -> String {
    if !value.is_finite() {
        return invalid_data_placeholder();
    }
    if value >= 1_000_000_000.0 {
        format!("{:.2}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.2}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

pub(crate) fn format_compact_usd(value: f64) -> String {
    if !value.is_finite() {
        return invalid_data_placeholder();
    }

    let sign = if value.is_sign_negative() { "-" } else { "" };
    format!("{sign}${}", format_compact(value.abs()))
}

fn currency_symbol(code: &str) -> Option<&str> {
    match code {
        "EUR" => Some("€"),
        "USD" => Some("$"),
        _ => None,
    }
}

fn format_symbol_price(symbol: &str, value: f64) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    format!("{sign}{symbol}{}", format_price(value.abs()))
}
