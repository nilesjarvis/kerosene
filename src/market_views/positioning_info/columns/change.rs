use super::{
    POSITIONING_TABLE_CELL_PADDING, POSITIONING_TABLE_COLUMN_SPACING, PositioningInfoColumns,
};

// ---------------------------------------------------------------------------
// Positioning Change Columns
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(in crate::market_views::positioning_info) struct PositioningChangeColumns {
    pub(in crate::market_views::positioning_info) trader_width: f32,
    pub(in crate::market_views::positioning_info) previous_width: f32,
    pub(in crate::market_views::positioning_info) current_width: f32,
    pub(in crate::market_views::positioning_info) delta_width: f32,
    pub(in crate::market_views::positioning_info) current_usd_width: f32,
    pub(in crate::market_views::positioning_info) delta_usd_width: f32,
}

pub(super) const POSITIONING_CHANGE_TRADER_MIN_WIDTH: f32 = 132.0;
pub(super) const POSITIONING_CHANGE_TRADER_COMPACT_ACTIONS_MIN_WIDTH: f32 =
    POSITIONING_CHANGE_TRADER_MIN_WIDTH;
const POSITIONING_CHANGE_PREVIOUS_WIDTH: f32 = 76.0;
const POSITIONING_CHANGE_CURRENT_WIDTH: f32 = 76.0;
const POSITIONING_CHANGE_DELTA_WIDTH: f32 = 76.0;
const POSITIONING_CHANGE_CURRENT_USD_WIDTH: f32 = 84.0;
const POSITIONING_CHANGE_DELTA_USD_WIDTH: f32 = 84.0;
const POSITIONING_CHANGE_TRADER_WEIGHT: f32 = 2.6;
const POSITIONING_CHANGE_NUMERIC_WEIGHT: f32 = 1.0;

impl PositioningChangeColumns {
    pub(in crate::market_views::positioning_info) fn for_width(width: f32) -> Self {
        let content_width = PositioningInfoColumns::available_content_width(width);
        let fixed_width = POSITIONING_CHANGE_PREVIOUS_WIDTH
            + POSITIONING_CHANGE_CURRENT_WIDTH
            + POSITIONING_CHANGE_DELTA_WIDTH
            + POSITIONING_CHANGE_CURRENT_USD_WIDTH
            + POSITIONING_CHANGE_DELTA_USD_WIDTH;
        let base_width_without_trader =
            POSITIONING_TABLE_CELL_PADDING + fixed_width + POSITIONING_TABLE_COLUMN_SPACING * 5.0;
        let available_for_trader = (content_width - base_width_without_trader).max(0.0);
        let trader_width = if available_for_trader < POSITIONING_CHANGE_TRADER_MIN_WIDTH {
            available_for_trader
        } else {
            POSITIONING_CHANGE_TRADER_MIN_WIDTH
        };

        let mut columns = Self {
            trader_width,
            previous_width: POSITIONING_CHANGE_PREVIOUS_WIDTH,
            current_width: POSITIONING_CHANGE_CURRENT_WIDTH,
            delta_width: POSITIONING_CHANGE_DELTA_WIDTH,
            current_usd_width: POSITIONING_CHANGE_CURRENT_USD_WIDTH,
            delta_usd_width: POSITIONING_CHANGE_DELTA_USD_WIDTH,
        };
        columns.distribute_extra_width((content_width - columns.total_width()).max(0.0));
        columns
    }

    pub(in crate::market_views::positioning_info) fn total_width(self) -> f32 {
        POSITIONING_TABLE_CELL_PADDING
            + self.trader_width
            + self.previous_width
            + self.current_width
            + self.delta_width
            + self.current_usd_width
            + self.delta_usd_width
            + POSITIONING_TABLE_COLUMN_SPACING * 5.0
    }

    fn distribute_extra_width(&mut self, extra: f32) {
        if extra <= 0.0 {
            return;
        }

        let total_weight =
            POSITIONING_CHANGE_TRADER_WEIGHT + POSITIONING_CHANGE_NUMERIC_WEIGHT * 5.0;
        self.trader_width += extra * POSITIONING_CHANGE_TRADER_WEIGHT / total_weight;
        let numeric_extra = extra * POSITIONING_CHANGE_NUMERIC_WEIGHT / total_weight;
        self.previous_width += numeric_extra;
        self.current_width += numeric_extra;
        self.delta_width += numeric_extra;
        self.current_usd_width += numeric_extra;
        self.delta_usd_width += numeric_extra;
    }
}
