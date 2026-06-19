use crate::chart_state::{ChartId, ChartSurfaceId};
use std::fmt;

// ---------------------------------------------------------------------------
// HUD Chart Order Requests
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HudOrderType {
    Limit,
    Market,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HudOrderSide {
    Long,
    Short,
}

impl HudOrderSide {
    pub(crate) fn is_buy(self) -> bool {
        matches!(self, Self::Long)
    }
}

#[derive(Clone)]
pub(crate) struct HudOrderRequest {
    pub(crate) chart_id: ChartId,
    pub(crate) surface_id: ChartSurfaceId,
    pub(crate) symbol_key: String,
    pub(crate) price: f64,
    pub(crate) quantity: String,
    pub(crate) order_type: HudOrderType,
    pub(crate) market_side: HudOrderSide,
    pub(crate) limit_side: Option<HudOrderSide>,
    pub(crate) click_x: f32,
    pub(crate) click_y: f32,
    pub(crate) chart_w: f32,
    pub(crate) chart_h: f32,
}

impl fmt::Debug for HudOrderRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HudOrderRequest")
            .field("chart_id", &self.chart_id)
            .field("surface_id", &self.surface_id)
            .field("symbol_key", &format_args!("<redacted>"))
            .field("price", &format_args!("<redacted>"))
            .field("quantity", &format_args!("<redacted>"))
            .field("order_type", &self.order_type)
            .field("market_side", &self.market_side)
            .field("limit_side", &self.limit_side)
            .field("click_x", &self.click_x)
            .field("click_y", &self.click_y)
            .field("chart_w", &self.chart_w)
            .field("chart_h", &self.chart_h)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{HudOrderRequest, HudOrderSide, HudOrderType};
    use crate::chart_state::ChartSurfaceId;

    #[test]
    fn hud_order_request_debug_redacts_order_details() {
        let request = HudOrderRequest {
            chart_id: 1,
            surface_id: ChartSurfaceId::Docked(1),
            symbol_key: "SECRETCOIN".to_string(),
            price: 98765.4321,
            quantity: "quantity-secret".to_string(),
            order_type: HudOrderType::Limit,
            market_side: HudOrderSide::Long,
            limit_side: Some(HudOrderSide::Short),
            click_x: 120.0,
            click_y: 80.0,
            chart_w: 400.0,
            chart_h: 240.0,
        };

        let rendered = format!("{request:?}");

        assert!(rendered.contains("symbol_key: <redacted>"));
        assert!(rendered.contains("price: <redacted>"));
        assert!(rendered.contains("quantity: <redacted>"));
        assert!(rendered.contains("order_type: Limit"));
        for secret in ["SECRETCOIN", "98765.4321", "quantity-secret"] {
            assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
        }
    }
}
