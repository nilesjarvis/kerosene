use crate::chart_state::{ChartId, ChartSurfaceId};

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

#[derive(Debug, Clone)]
pub(crate) struct HudOrderRequest {
    pub(crate) chart_id: ChartId,
    pub(crate) surface_id: ChartSurfaceId,
    pub(crate) price: f64,
    pub(crate) quantity: String,
    pub(crate) order_type: HudOrderType,
    pub(crate) market_side: HudOrderSide,
    pub(crate) click_x: f32,
    pub(crate) click_y: f32,
    pub(crate) chart_w: f32,
    pub(crate) chart_h: f32,
}
