use iced::Point;

use super::{IncomeBarLayout, IncomeTooltipLayout, TOOLTIP_HEIGHT};

// ---------------------------------------------------------------------------
// Tooltip Layout
// ---------------------------------------------------------------------------

pub(in crate::portfolio_state::charts::income) fn income_tooltip_layout(
    bar: &IncomeBarLayout,
    value_text: &str,
    width: f32,
    height: f32,
) -> IncomeTooltipLayout {
    let tip_y = if bar.scaled >= 0.0 {
        (bar.y).max(6.0)
    } else {
        (bar.y - 4.0).max(6.0)
    };

    let text_chars = bar.label.len().max(value_text.len()) as f32;
    let tooltip_width = (text_chars * 6.2 + 18.0).clamp(132.0, 198.0);
    let max_x = (width - tooltip_width - 4.0).max(4.0);
    let max_y = (height - TOOLTIP_HEIGHT - 4.0).max(4.0);
    let x = (bar.center_x - tooltip_width * 0.5).clamp(4.0, max_x);
    let y = (tip_y - TOOLTIP_HEIGHT - 8.0).clamp(4.0, max_y);

    IncomeTooltipLayout {
        origin: Point::new(x, y),
        width: tooltip_width,
        height: TOOLTIP_HEIGHT,
    }
}
