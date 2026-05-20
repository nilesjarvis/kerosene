use crate::app_state::TradingTerminal;
use crate::config::DisplayDenominationConfig;
use crate::helpers::{format_decimal_with_commas, format_price, format_usd};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Display Denomination
// ---------------------------------------------------------------------------

pub(crate) const DISPLAY_DENOMINATION_RATE_STALE_MS: u64 = 5 * 60 * 1000;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DisplayDenominationContext {
    config: DisplayDenominationConfig,
    rate: Option<f64>,
    rate_stale: bool,
}

impl Default for DisplayDenominationContext {
    fn default() -> Self {
        Self::usd()
    }
}

impl DisplayDenominationContext {
    pub(crate) fn usd() -> Self {
        Self {
            config: DisplayDenominationConfig::Usd,
            rate: None,
            rate_stale: false,
        }
    }

    pub(crate) fn from_mids(
        config: DisplayDenominationConfig,
        mids: &HashMap<String, f64>,
        updated_at_ms: &HashMap<String, u64>,
        now_ms: u64,
    ) -> Self {
        let config = config.normalized();
        if config.is_usd() {
            return Self::usd();
        }

        let rate_key = config.rate_symbol_key();
        let (rate, rate_stale) = match rate_key {
            Some(key) => {
                let rate = mids
                    .get(&key)
                    .copied()
                    .filter(|value| value.is_finite() && *value > 0.0);
                let rate_stale = updated_at_ms
                    .get(&key)
                    .copied()
                    .map(|updated_at| {
                        now_ms.saturating_sub(updated_at) > DISPLAY_DENOMINATION_RATE_STALE_MS
                    })
                    .unwrap_or(true);
                (rate, rate_stale)
            }
            None => (None, true),
        };

        Self {
            config,
            rate,
            rate_stale,
        }
    }

    pub(crate) fn is_available(&self) -> bool {
        self.config.is_usd()
            || self
                .rate
                .is_some_and(|rate| rate.is_finite() && rate > 0.0 && !self.rate_stale)
    }

    pub(crate) fn is_fallback_usd(&self) -> bool {
        !self.config.is_usd() && !self.is_available()
    }

    pub(crate) fn requested_code(&self) -> &str {
        self.config.code()
    }

    pub(crate) fn active_code(&self) -> &str {
        if self.is_fallback_usd() {
            "USD"
        } else {
            self.config.code()
        }
    }

    pub(crate) fn active_symbol(&self) -> &str {
        currency_symbol(self.active_code())
    }

    pub(crate) fn unavailable_status(&self) -> Option<String> {
        if !self.is_fallback_usd() {
            return None;
        }

        let status = if self.rate.is_some() && self.rate_stale {
            "stale"
        } else {
            "unavailable"
        };
        Some(format!(
            "{} rate {status}; showing USD",
            self.requested_code()
        ))
    }

    pub(crate) fn convert_usd_value(&self, value: f64) -> Option<f64> {
        if !value.is_finite() {
            return None;
        }
        if self.config.is_usd() || self.is_fallback_usd() {
            return Some(value);
        }

        self.rate
            .filter(|rate| rate.is_finite() && *rate > 0.0 && !self.rate_stale)
            .map(|rate| value / rate)
    }

    pub(crate) fn format_usd_str(&self, raw: &str) -> String {
        let Ok(value) = raw.parse::<f64>() else {
            return raw.to_string();
        };
        self.format_value(value, 2)
    }

    pub(crate) fn format_value(&self, usd_value: f64, decimals: usize) -> String {
        let Some(value) = self.convert_usd_value(usd_value) else {
            return "Invalid data".to_string();
        };
        let sign = if value < 0.0 { "-" } else { "" };
        let abs = value.abs();

        if self.active_code() == "USD" {
            return format_usd(&format!("{value:.decimals$}"));
        }

        if decimals == 0 {
            format!(
                "{sign}{}{}",
                self.active_symbol(),
                format_decimal_with_commas(abs, 0)
            )
        } else {
            format!(
                "{sign}{}{}",
                self.active_symbol(),
                format_decimal_with_commas(abs, decimals)
            )
        }
    }

    pub(crate) fn format_price(&self, usd_price: f64) -> String {
        let Some(value) = self.convert_usd_value(usd_price) else {
            return "Invalid data".to_string();
        };
        format_price(value)
    }

    pub(crate) fn format_chart_price(&self, usd_price: f64) -> String {
        let Some(value) = self.convert_usd_value(usd_price) else {
            return "Invalid data".to_string();
        };
        if !self.includes_usd_chart_reference() {
            return format_price(value);
        }

        format!(
            "{} ({})",
            format_symbol_price(self.active_symbol(), value),
            format_symbol_price("$", usd_price)
        )
    }

    pub(crate) fn format_signed_chart_price(&self, usd_price: f64) -> String {
        let Some(value) = self.convert_usd_value(usd_price) else {
            return "Invalid data".to_string();
        };
        if !self.includes_usd_chart_reference() {
            return format_signed_plain_price(value);
        }

        format!(
            "{} ({})",
            format_signed_symbol_price(self.active_symbol(), value),
            format_signed_symbol_price("$", usd_price)
        )
    }

    pub(crate) fn includes_usd_chart_reference(&self) -> bool {
        self.active_code() != "USD"
    }

    pub(crate) fn format_signed_value(&self, usd_value: f64, decimals: usize) -> String {
        let Some(value) = self.convert_usd_value(usd_value) else {
            return "Invalid data".to_string();
        };
        let display_value = value.abs();
        let sign = if value > 0.0 {
            "+"
        } else if value < 0.0 {
            "-"
        } else {
            ""
        };
        format!(
            "{sign}{}{}",
            self.active_symbol(),
            format_decimal_with_commas(display_value, decimals)
        )
    }

    pub(crate) fn format_compact_value(&self, usd_value: f64) -> String {
        let Some(value) = self.convert_usd_value(usd_value) else {
            return "Invalid data".to_string();
        };
        let sign = if value < 0.0 { "-" } else { "" };
        format!(
            "{sign}{}{}",
            self.active_symbol(),
            format_compact(value.abs())
        )
    }

    #[cfg(test)]
    pub(crate) fn format_signed_compact_value(&self, usd_value: f64) -> String {
        let Some(value) = self.convert_usd_value(usd_value) else {
            return "Invalid data".to_string();
        };
        let sign = if value > 0.0 {
            "+"
        } else if value < 0.0 {
            "-"
        } else {
            ""
        };
        format!(
            "{sign}{}{}",
            self.active_symbol(),
            format_compact(value.abs())
        )
    }

    pub(crate) fn hidden_mask(&self) -> String {
        format!("{}***", self.active_symbol())
    }
}

pub(crate) fn format_compact(value: f64) -> String {
    if !value.is_finite() {
        return "Invalid data".to_string();
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

fn currency_symbol(code: &str) -> &str {
    match code {
        "EUR" => "€",
        "USD" => "$",
        _ => "",
    }
}

fn format_symbol_price(symbol: &str, value: f64) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    format!("{sign}{symbol}{}", format_price(value.abs()))
}

fn format_signed_plain_price(value: f64) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "+" };
    format!("{sign}{}", format_price(value.abs()))
}

fn format_signed_symbol_price(symbol: &str, value: f64) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "+" };
    format!("{sign}{symbol}{}", format_price(value.abs()))
}

impl TradingTerminal {
    pub(crate) fn display_denomination_context(&self) -> DisplayDenominationContext {
        DisplayDenominationContext::from_mids(
            self.display_denomination.clone(),
            &self.all_mids,
            &self.all_mids_updated_at_ms,
            Self::now_ms(),
        )
    }

    pub(crate) fn display_denomination_options(&self) -> Vec<DisplayDenominationConfig> {
        vec![
            DisplayDenominationConfig::Usd,
            DisplayDenominationConfig::eur(),
        ]
    }

    pub(crate) fn display_denomination_rate_symbol_key(&self) -> Option<String> {
        self.display_denomination
            .clone()
            .normalized()
            .rate_symbol_key()
    }

    pub(crate) fn sync_chart_display_denominations(&mut self) {
        let context = self.display_denomination_context();
        for instance in self.charts.values_mut() {
            instance.chart.set_display_denomination(context.clone());
        }
    }

    pub(crate) fn format_display_usd_value(&self, value: f64, decimals: usize) -> String {
        self.display_denomination_context()
            .format_value(value, decimals)
    }

    pub(crate) fn format_display_usd_str(&self, raw: &str) -> String {
        self.display_denomination_context().format_usd_str(raw)
    }

    pub(crate) fn format_display_price(&self, usd_price: f64) -> String {
        self.display_denomination_context().format_price(usd_price)
    }

    pub(crate) fn format_display_signed_usd_value(&self, value: f64) -> String {
        self.display_denomination_context()
            .format_signed_value(value, 2)
    }

    pub(crate) fn display_pnl_mask(&self) -> String {
        self.display_denomination_context().hidden_mask()
    }

    pub(crate) fn display_denomination_status(&self) -> Option<String> {
        self.display_denomination_context().unavailable_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eur_context(rate: f64) -> DisplayDenominationContext {
        DisplayDenominationContext::from_mids(
            DisplayDenominationConfig::eur(),
            &HashMap::from([("xyz:EUR".to_string(), rate)]),
            &HashMap::from([("xyz:EUR".to_string(), 1_000)]),
            1_000,
        )
    }

    #[test]
    fn usd_default_formats_like_existing_usd_formatter() {
        let ctx = DisplayDenominationContext::usd();

        assert_eq!(ctx.format_value(12_345.67, 2), "$12,345.67");
        assert_eq!(ctx.format_value(-12.5, 2), "-$12.50");
        assert_eq!(ctx.format_chart_price(125.0), "125.00");
        assert_eq!(ctx.format_signed_chart_price(12.5), "+12.50");
        assert_eq!(ctx.format_compact_value(1_250_000.0), "$1.25M");
    }

    #[test]
    fn eur_context_converts_usd_by_usd_per_eur_mid() {
        let ctx = eur_context(1.25);

        assert_eq!(ctx.format_value(125.0, 2), "€100.00");
        assert_eq!(ctx.format_price(12.5), "10.00");
        assert_eq!(ctx.format_chart_price(125.0), "€100.00 ($125.00)");
        assert_eq!(ctx.format_signed_chart_price(-125.0), "-€100.00 (-$125.00)");
        assert_eq!(ctx.format_compact_value(1_250_000.0), "€1.00M");
    }

    #[test]
    fn signed_values_keep_explicit_positive_marker() {
        let ctx = eur_context(2.0);

        assert_eq!(ctx.format_signed_value(20.0, 2), "+€10.00");
        assert_eq!(ctx.format_signed_value(-20.0, 2), "-€10.00");
        assert_eq!(ctx.format_signed_compact_value(2_000_000.0), "+€1.00M");
    }

    #[test]
    fn missing_rate_falls_back_to_usd_with_status() {
        let ctx = DisplayDenominationContext::from_mids(
            DisplayDenominationConfig::eur(),
            &HashMap::new(),
            &HashMap::new(),
            1_000,
        );

        assert!(ctx.is_fallback_usd());
        assert_eq!(ctx.format_value(125.0, 2), "$125.00");
        assert_eq!(ctx.format_chart_price(125.0), "125.00");
        assert_eq!(
            ctx.unavailable_status().as_deref(),
            Some("EUR rate unavailable; showing USD")
        );
    }

    #[test]
    fn stale_rate_falls_back_to_usd_with_status() {
        let ctx = DisplayDenominationContext::from_mids(
            DisplayDenominationConfig::eur(),
            &HashMap::from([("xyz:EUR".to_string(), 1.25)]),
            &HashMap::from([("xyz:EUR".to_string(), 1_000)]),
            1_000 + DISPLAY_DENOMINATION_RATE_STALE_MS + 1,
        );

        assert!(ctx.is_fallback_usd());
        assert_eq!(ctx.format_value(125.0, 2), "$125.00");
        assert_eq!(ctx.format_chart_price(125.0), "125.00");
        assert_eq!(
            ctx.unavailable_status().as_deref(),
            Some("EUR rate stale; showing USD")
        );
    }

    #[test]
    fn invalid_inputs_are_marked_invalid() {
        let ctx = eur_context(1.25);

        assert_eq!(ctx.format_value(f64::NAN, 2), "Invalid data");
        assert_eq!(ctx.format_price(f64::INFINITY), "Invalid data");
        assert_eq!(ctx.format_chart_price(f64::INFINITY), "Invalid data");
    }
}
