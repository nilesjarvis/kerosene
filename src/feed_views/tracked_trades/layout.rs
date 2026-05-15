// ---------------------------------------------------------------------------
// Tracked Trade Row Layout
// ---------------------------------------------------------------------------

const HIDE_INTENT_WIDTH: f32 = 810.0;
const HIDE_FEE_WIDTH: f32 = 730.0;
const HIDE_PNL_WIDTH: f32 = 650.0;

pub(super) const TIME_WIDTH: f32 = 54.0;
pub(super) const WALLET_COLUMN_WIDTH: f32 = 150.0;
pub(super) const WALLET_LABEL_WIDTH: f32 = 96.0;
pub(super) const COIN_WIDTH: f32 = 74.0;
pub(super) const SIDE_WIDTH: f32 = 42.0;
pub(super) const NUMBER_WIDTH: f32 = 72.0;
pub(super) const ROW_SPACING: f32 = 6.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct TrackedTradeRowLayout {
    pub(super) show_intent: bool,
    pub(super) show_fee: bool,
    pub(super) show_pnl: bool,
}

impl TrackedTradeRowLayout {
    pub(super) fn from_width(width: f32) -> Self {
        Self {
            show_intent: width >= HIDE_INTENT_WIDTH,
            show_fee: width >= HIDE_FEE_WIDTH,
            show_pnl: width >= HIDE_PNL_WIDTH,
        }
    }
}
