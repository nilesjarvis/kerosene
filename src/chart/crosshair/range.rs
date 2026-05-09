// ---------------------------------------------------------------------------
// Range Measurement Layout
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

const ZERO_PRICE_EPSILON: f64 = 1e-12;
const LABEL_HEIGHT: f32 = 16.0;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RangeMeasurement {
    pub(super) anchor_y: f32,
    pub(super) hover_y: f32,
    pub(super) top: f32,
    pub(super) bottom: f32,
    pub(super) is_up: bool,
    pub(super) label: String,
    pub(super) label_x: f32,
    pub(super) label_y: f32,
    pub(super) label_width: f32,
    pub(super) label_height: f32,
}

pub(super) fn calculate_range_measurement(
    anchor_price: f64,
    hover_price: f64,
    anchor_y: f32,
    cursor_x: f32,
    cursor_y: f32,
    chart_w: f32,
    price_h: f32,
) -> RangeMeasurement {
    let anchor_y = anchor_y.clamp(0.0, price_h);
    let hover_y = cursor_y.clamp(0.0, price_h);
    let top = anchor_y.min(hover_y);
    let bottom = anchor_y.max(hover_y);
    let delta = hover_price - anchor_price;
    let pct = if anchor_price.abs() > ZERO_PRICE_EPSILON {
        delta / anchor_price * 100.0
    } else {
        0.0
    };
    let label = format!("{:+.2}% ({:+.2})", pct, delta);
    let label_width = label.len() as f32 * 6.3 + 8.0;
    let label_x = (cursor_x + 10.0).min(chart_w - label_width - 4.0).max(4.0);
    let label_y = (cursor_y - 20.0).max(4.0).min(price_h - LABEL_HEIGHT - 2.0);

    RangeMeasurement {
        anchor_y,
        hover_y,
        top,
        bottom,
        is_up: delta >= 0.0,
        label,
        label_x,
        label_y,
        label_width,
        label_height: LABEL_HEIGHT,
    }
}
