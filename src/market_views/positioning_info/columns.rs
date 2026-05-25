mod change;

pub(super) use change::PositioningChangeColumns;

pub(super) const POSITIONING_CHANGE_TRADER_COMPACT_ACTIONS_MIN_WIDTH: f32 =
    change::POSITIONING_CHANGE_TRADER_COMPACT_ACTIONS_MIN_WIDTH;
#[cfg(test)]
pub(super) const POSITIONING_CHANGE_TRADER_MIN_WIDTH: f32 =
    change::POSITIONING_CHANGE_TRADER_MIN_WIDTH;

#[derive(Debug, Clone, Copy)]
pub(super) struct PositioningInfoColumns {
    pub(super) trader_width: f32,
    pub(super) side_width: f32,
    pub(super) size_width: f32,
    pub(super) notional_width: f32,
    pub(super) upnl_width: f32,
    pub(super) entry_width: f32,
    pub(super) liq_width: f32,
    pub(super) funding_width: f32,
    pub(super) account_width: f32,
    pub(super) show_entry: bool,
    pub(super) show_liq: bool,
    pub(super) show_funding: bool,
    pub(super) show_account: bool,
}

const POSITIONING_TABLE_CONTENT_PADDING: f32 = 20.0;
const POSITIONING_TABLE_SCROLLBAR_RESERVE: f32 = 14.0;
pub(super) const POSITIONING_TABLE_CELL_PADDING: f32 = 16.0;
pub(super) const POSITIONING_TABLE_COLUMN_SPACING: f32 = 6.0;
pub(super) const POSITIONING_TRADER_MIN_WIDTH: f32 = 112.0;
const POSITIONING_SIDE_WIDTH: f32 = 44.0;
pub(super) const POSITIONING_SIZE_WIDTH: f32 = 64.0;
const POSITIONING_NOTIONAL_WIDTH: f32 = 76.0;
const POSITIONING_UPNL_WIDTH: f32 = 74.0;
const POSITIONING_ENTRY_WIDTH: f32 = 70.0;
const POSITIONING_LIQ_WIDTH: f32 = 70.0;
const POSITIONING_FUNDING_WIDTH: f32 = 74.0;
const POSITIONING_ACCOUNT_WIDTH: f32 = 76.0;
const POSITIONING_TRADER_WEIGHT: f32 = 2.4;
const POSITIONING_SIDE_WEIGHT: f32 = 0.7;
const POSITIONING_SIZE_WEIGHT: f32 = 1.0;
const POSITIONING_NOTIONAL_WEIGHT: f32 = 1.15;
const POSITIONING_UPNL_WEIGHT: f32 = 1.15;
const POSITIONING_ENTRY_WEIGHT: f32 = 1.0;
const POSITIONING_LIQ_WEIGHT: f32 = 1.0;
const POSITIONING_FUNDING_WEIGHT: f32 = 1.1;
const POSITIONING_ACCOUNT_WEIGHT: f32 = 1.15;
pub(super) const POSITIONING_TRADER_COMPACT_ACTIONS_MIN_WIDTH: f32 = 168.0;
pub(super) const POSITIONING_TRADER_FULL_ACTIONS_MIN_WIDTH: f32 = 240.0;
pub(super) const POSITIONING_TRADER_COMPACT_ACTIONS_WIDTH: f32 = 42.0;
pub(super) const POSITIONING_TRADER_FULL_ACTIONS_WIDTH: f32 = 106.0;
impl PositioningInfoColumns {
    pub(super) fn for_width(width: f32) -> Self {
        let content_width = Self::available_content_width(width);
        let base_fixed_width = POSITIONING_SIDE_WIDTH
            + POSITIONING_SIZE_WIDTH
            + POSITIONING_NOTIONAL_WIDTH
            + POSITIONING_UPNL_WIDTH;
        let base_width_without_trader = POSITIONING_TABLE_CELL_PADDING
            + base_fixed_width
            + POSITIONING_TABLE_COLUMN_SPACING * 4.0;
        let available_for_trader = (content_width - base_width_without_trader).max(0.0);
        let trader_width = if available_for_trader < POSITIONING_TRADER_MIN_WIDTH {
            available_for_trader
        } else {
            POSITIONING_TRADER_MIN_WIDTH
        };
        let mut used_width = base_width_without_trader + trader_width;
        let mut include_column = |column_width: f32| {
            let next_width = used_width + POSITIONING_TABLE_COLUMN_SPACING + column_width;
            if next_width <= content_width {
                used_width = next_width;
                true
            } else {
                false
            }
        };

        let show_entry = include_column(POSITIONING_ENTRY_WIDTH);
        let show_liq = include_column(POSITIONING_LIQ_WIDTH);
        let show_funding = include_column(POSITIONING_FUNDING_WIDTH);
        let show_account = include_column(POSITIONING_ACCOUNT_WIDTH);

        let mut columns = Self {
            trader_width,
            side_width: POSITIONING_SIDE_WIDTH,
            size_width: POSITIONING_SIZE_WIDTH,
            notional_width: POSITIONING_NOTIONAL_WIDTH,
            upnl_width: POSITIONING_UPNL_WIDTH,
            entry_width: POSITIONING_ENTRY_WIDTH,
            liq_width: POSITIONING_LIQ_WIDTH,
            funding_width: POSITIONING_FUNDING_WIDTH,
            account_width: POSITIONING_ACCOUNT_WIDTH,
            show_entry,
            show_liq,
            show_funding,
            show_account,
        };
        columns.distribute_extra_width((content_width - columns.total_width()).max(0.0));
        columns
    }

    pub(super) fn available_content_width(width: f32) -> f32 {
        if width.is_finite() {
            (width - POSITIONING_TABLE_CONTENT_PADDING - POSITIONING_TABLE_SCROLLBAR_RESERVE)
                .max(0.0)
        } else {
            0.0
        }
    }

    fn visible_column_count(self) -> usize {
        5 + usize::from(self.show_entry)
            + usize::from(self.show_liq)
            + usize::from(self.show_funding)
            + usize::from(self.show_account)
    }

    pub(super) fn total_width(self) -> f32 {
        let mut optional_width = 0.0;
        if self.show_entry {
            optional_width += self.entry_width;
        }
        if self.show_liq {
            optional_width += self.liq_width;
        }
        if self.show_funding {
            optional_width += self.funding_width;
        }
        if self.show_account {
            optional_width += self.account_width;
        }
        let gap_count = self.visible_column_count().saturating_sub(1) as f32;
        POSITIONING_TABLE_CELL_PADDING
            + self.trader_width
            + self.side_width
            + self.size_width
            + self.notional_width
            + self.upnl_width
            + optional_width
            + POSITIONING_TABLE_COLUMN_SPACING * gap_count
    }

    fn distribute_extra_width(&mut self, extra: f32) {
        if extra <= 0.0 {
            return;
        }

        let total_weight = POSITIONING_TRADER_WEIGHT
            + POSITIONING_SIDE_WEIGHT
            + POSITIONING_SIZE_WEIGHT
            + POSITIONING_NOTIONAL_WEIGHT
            + POSITIONING_UPNL_WEIGHT
            + if self.show_entry {
                POSITIONING_ENTRY_WEIGHT
            } else {
                0.0
            }
            + if self.show_liq {
                POSITIONING_LIQ_WEIGHT
            } else {
                0.0
            }
            + if self.show_funding {
                POSITIONING_FUNDING_WEIGHT
            } else {
                0.0
            }
            + if self.show_account {
                POSITIONING_ACCOUNT_WEIGHT
            } else {
                0.0
            };

        self.trader_width += extra * POSITIONING_TRADER_WEIGHT / total_weight;
        self.side_width += extra * POSITIONING_SIDE_WEIGHT / total_weight;
        self.size_width += extra * POSITIONING_SIZE_WEIGHT / total_weight;
        self.notional_width += extra * POSITIONING_NOTIONAL_WEIGHT / total_weight;
        self.upnl_width += extra * POSITIONING_UPNL_WEIGHT / total_weight;
        if self.show_entry {
            self.entry_width += extra * POSITIONING_ENTRY_WEIGHT / total_weight;
        }
        if self.show_liq {
            self.liq_width += extra * POSITIONING_LIQ_WEIGHT / total_weight;
        }
        if self.show_funding {
            self.funding_width += extra * POSITIONING_FUNDING_WEIGHT / total_weight;
        }
        if self.show_account {
            self.account_width += extra * POSITIONING_ACCOUNT_WEIGHT / total_weight;
        }
    }
}
