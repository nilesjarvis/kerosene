use iced::Point;

use super::{PNL_TOOLTIP_HEIGHT, PNL_TOOLTIP_WIDTH};

// ---------------------------------------------------------------------------
// PnL Tooltip
// ---------------------------------------------------------------------------

pub(in crate::portfolio_state::charts::pnl) fn pnl_tooltip_origin(
    point: Point,
    width: f32,
    height: f32,
) -> Point {
    let mut x = point.x + 10.0;
    if x + PNL_TOOLTIP_WIDTH > width {
        x = (point.x - PNL_TOOLTIP_WIDTH - 10.0).max(4.0);
    }
    let max_y = (height - PNL_TOOLTIP_HEIGHT - 4.0).max(4.0);
    let y = (point.y - PNL_TOOLTIP_HEIGHT - 8.0).clamp(4.0, max_y);
    Point::new(x, y)
}
