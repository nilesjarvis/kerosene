// ---------------------------------------------------------------------------
// Liquidation Feed Row Layout
// ---------------------------------------------------------------------------

pub(super) const TIME_WIDTH: f32 = 60.0;
pub(super) const COIN_WIDTH: f32 = 80.0;
pub(super) const SIDE_WIDTH: f32 = 50.0;
pub(super) const NUMBER_WIDTH: f32 = 80.0;
pub(super) const USER_WIDTH: f32 = 90.0;
pub(super) const METHOD_WIDTH: f32 = 88.0;
pub(super) const ROW_SPACING: f32 = 8.0;

const HIDE_METHOD_BELOW: f32 = 680.0;
const HIDE_USER_BELOW: f32 = 590.0;
const HIDE_PRICE_BELOW: f32 = 500.0;
const HIDE_SIZE_BELOW: f32 = 410.0;
const HIDE_SIDE_BELOW: f32 = 330.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LiquidationFeedRowLayout {
    pub(super) show_side: bool,
    pub(super) show_size: bool,
    pub(super) show_price: bool,
    pub(super) show_user: bool,
    pub(super) show_method: bool,
}

impl LiquidationFeedRowLayout {
    pub(super) fn from_width(width: f32) -> Self {
        Self {
            show_side: width >= HIDE_SIDE_BELOW,
            show_size: width >= HIDE_SIZE_BELOW,
            show_price: width >= HIDE_PRICE_BELOW,
            show_user: width >= HIDE_USER_BELOW,
            show_method: width >= HIDE_METHOD_BELOW,
        }
    }
}
