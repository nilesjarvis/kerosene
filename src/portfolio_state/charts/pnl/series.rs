use iced::Point;
use std::fmt;

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

#[derive(Clone, Copy, PartialEq)]
pub(super) struct PnlChartPoint {
    pub(super) point: Point,
    pub(super) timestamp_ms: u64,
    pub(super) pnl: f64,
}

impl fmt::Debug for PnlChartPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PnlChartPoint")
            .field("point", &"<redacted>")
            .field("timestamp_ms", &"<redacted>")
            .field("pnl", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct PnlChartLayout {
    pub(super) points: Vec<PnlChartPoint>,
    pub(super) zero_y: f32,
}

impl fmt::Debug for PnlChartLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PnlChartLayout")
            .field("points_count", &self.points.len())
            .field("zero_y", &"<redacted>")
            .finish()
    }
}
