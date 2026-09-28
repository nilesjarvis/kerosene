use crate::chart_state::{ChartId, ChartSurfaceId};
use std::fmt;

#[cfg(test)]
mod tests;

/// State for the right-click quick order form on a chart.
#[derive(Clone, PartialEq)]
pub(crate) struct QuickOrderForm {
    /// Price at the right-click Y coordinate (pre-filled for limit orders).
    pub(crate) price: f64,
    /// User-entered quantity string.
    pub(crate) quantity: String,
    /// True when the quantity field is USD notional, false when it is coin size.
    pub(crate) quantity_is_usd: bool,
    /// Percentage of available notional represented by the current quantity.
    pub(crate) percentage: f32,
    /// Account snapshot and pricing context used to derive `quantity` from the
    /// percentage slider.
    pub(crate) quantity_provenance: Option<QuickOrderQuantityProvenance>,
    /// True = limit order at clicked price, false = market order.
    pub(crate) is_limit: bool,
    /// Canvas-local X coordinate of the right-click (for card positioning).
    pub(crate) click_x: f32,
    /// Canvas-local Y coordinate of the right-click (for card positioning).
    pub(crate) click_y: f32,
    /// Chart canvas width when clicked.
    pub(crate) chart_w: f32,
    /// Chart canvas height when clicked.
    pub(crate) chart_h: f32,
}

impl fmt::Debug for QuickOrderForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuickOrderForm")
            .field("price", &format_args!("<redacted>"))
            .field("quantity", &format_args!("<redacted>"))
            .field("quantity_is_usd", &self.quantity_is_usd)
            .field("percentage", &format_args!("<redacted>"))
            .field(
                "quantity_provenance",
                &self.quantity_provenance.as_ref().map(|_| "<redacted>"),
            )
            .field("is_limit", &self.is_limit)
            .field("click_x", &self.click_x)
            .field("click_y", &self.click_y)
            .field("chart_w", &self.chart_w)
            .field("chart_h", &self.chart_h)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct QuickOrderRecovery {
    pub(crate) chart_id: ChartId,
    pub(crate) form: QuickOrderForm,
    pub(crate) surface_id: Option<ChartSurfaceId>,
}

impl fmt::Debug for QuickOrderRecovery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuickOrderRecovery")
            .field("chart_id", &self.chart_id)
            .field("form", &self.form)
            .field("surface_id", &self.surface_id)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct QuickOrderQuantityProvenance {
    pub(crate) account_address: String,
    pub(crate) account_data_revision: u64,
    pub(crate) spot_balances_revision: u64,
    pub(crate) symbol_key: String,
    pub(crate) quantity_is_usd: bool,
    pub(crate) percentage: f32,
    pub(crate) is_limit: bool,
    pub(crate) reference_price: Option<f64>,
    pub(crate) reduce_only: bool,
    pub(crate) market_universe: crate::config::MarketUniverseConfig,
}

impl fmt::Debug for QuickOrderQuantityProvenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuickOrderQuantityProvenance")
            .field("account_address", &"<redacted>")
            .field("account_data_revision", &self.account_data_revision)
            .field("spot_balances_revision", &self.spot_balances_revision)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("quantity_is_usd", &self.quantity_is_usd)
            .field("percentage", &format_args!("<redacted>"))
            .field("is_limit", &self.is_limit)
            .field(
                "reference_price",
                &self.reference_price.as_ref().map(|_| "<redacted>"),
            )
            .field("reduce_only", &self.reduce_only)
            .field("market_universe", &self.market_universe)
            .finish()
    }
}
