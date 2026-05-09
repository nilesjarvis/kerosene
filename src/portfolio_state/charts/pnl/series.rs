use iced::Point;

mod layout;
#[cfg(test)]
mod tests;
mod tooltip;

pub(super) use self::layout::{nearest_pnl_point, prepare_pnl_chart_layout};
pub(super) use self::tooltip::pnl_tooltip_origin;

// ---------------------------------------------------------------------------
// Portfolio PnL Chart Series
// ---------------------------------------------------------------------------

pub(super) const PNL_TOOLTIP_WIDTH: f32 = 188.0;
pub(super) const PNL_TOOLTIP_HEIGHT: f32 = 34.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PnlChartPoint {
    pub(super) point: Point,
    pub(super) timestamp_ms: u64,
    pub(super) pnl: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PnlChartLayout {
    pub(super) points: Vec<PnlChartPoint>,
    pub(super) zero_y: f32,
}
