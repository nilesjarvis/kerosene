use crate::app_state::TradingTerminal;
use crate::config::DisplayDenominationConfig;
use std::collections::HashMap;

mod formatting;

pub(crate) use self::formatting::format_compact_usd;

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
            DisplayDenominationConfig::hype(),
            DisplayDenominationConfig::btc(),
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
mod tests;
