// ---------------------------------------------------------------------------
// Tracked Trade Row Layout
// ---------------------------------------------------------------------------

const HIDE_INTENT_WIDTH: f32 = 835.0;
const HIDE_FEE_WIDTH: f32 = 760.0;
const HIDE_PNL_WIDTH: f32 = 680.0;
const HIDE_PRICE_WIDTH: f32 = 600.0;
const HIDE_SIZE_WIDTH: f32 = 520.0;
const HIDE_SIDE_WIDTH: f32 = 440.0;
const HIDE_NOTIONAL_WIDTH: f32 = 390.0;
const HIDE_TIME_WIDTH: f32 = 315.0;

pub(super) const TIME_WIDTH: f32 = 54.0;
pub(super) const WALLET_COLUMN_WIDTH: f32 = 150.0;
pub(super) const COIN_WIDTH: f32 = 96.0;
pub(super) const SIDE_WIDTH: f32 = 42.0;
pub(super) const NUMBER_WIDTH: f32 = 72.0;
pub(super) const ROW_SPACING: f32 = 6.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct TrackedTradeRowLayout {
    pub(super) show_time: bool,
    pub(super) show_side: bool,
    pub(super) show_size: bool,
    pub(super) show_price: bool,
    pub(super) show_notional: bool,
    pub(super) show_intent: bool,
    pub(super) show_fee: bool,
    pub(super) show_pnl: bool,
}

impl TrackedTradeRowLayout {
    pub(super) fn from_width(width: f32) -> Self {
        Self {
            show_time: width >= HIDE_TIME_WIDTH,
            show_side: width >= HIDE_SIDE_WIDTH,
            show_size: width >= HIDE_SIZE_WIDTH,
            show_price: width >= HIDE_PRICE_WIDTH,
            show_notional: width >= HIDE_NOTIONAL_WIDTH,
            show_intent: width >= HIDE_INTENT_WIDTH,
            show_fee: width >= HIDE_FEE_WIDTH,
            show_pnl: width >= HIDE_PNL_WIDTH,
        }
    }
}
