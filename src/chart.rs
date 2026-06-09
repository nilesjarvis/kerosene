mod annotation_overlays;
mod candle_layer;
mod countdown;
mod crosshair;
pub(crate) mod crosshair_style;
mod data;
mod drawing;
mod earnings_hover;
pub(crate) mod fisheye;
mod formatting;
mod geometry;
mod hud_order_animation;
mod hud_safety;
mod indicators;
mod interaction;
mod model;
mod moving_averages;
mod order_cancel_hover;
mod order_hit;
mod order_labels;
mod overlays;
#[cfg(test)]
pub(crate) mod perf_probe;
mod price_badges;
mod price_range;
mod program;
mod segmented_curve;
mod state;
mod tooltips;
mod viewport;
mod volume_profile;

#[cfg(test)]
pub use self::model::PRICE_AXIS_WIDTH;
pub use self::model::{
    CANDLE_GAP_RATIO, CandlestickChart, ChartStatus, ChartViewport, DEFAULT_CANDLE_WIDTH,
    DEFAULT_FUNDING_PANEL_HEIGHT, EarningsMarker, FUNDING_PANEL_RESIZE_HIT_PX, MAX_CANDLE_WIDTH,
    MAX_FUNDING_PANEL_HEIGHT, MIN_CANDLE_WIDTH, MIN_FUNDING_PANEL_HEIGHT, MIN_MAIN_CHART_HEIGHT,
    OrderOverlay, OrderOverlayPendingState, PAN_SPEED, PRICE_PADDING_PCT, PositionOverlay,
    TIME_AXIS_HEIGHT, TradeMarker, VOLUME_REGION_RATIO, ZOOM_SPEED,
};
pub use self::state::ChartState;

#[cfg(test)]
mod tests;
