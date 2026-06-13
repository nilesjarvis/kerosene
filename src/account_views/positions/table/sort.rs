use crate::account;
use crate::account_state::PositionsSortColumn;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers::parse_finite_number;

#[cfg(test)]
mod tests;

pub(in crate::account_views::positions::table) struct PositionRowData {
    pub(in crate::account_views::positions::table) ap: account::AssetPosition,
    pub(in crate::account_views::positions::table) coin: String,
    pub(in crate::account_views::positions::table) szi: Option<f64>,
    pub(in crate::account_views::positions::table) entry_px: Option<f64>,
    pub(in crate::account_views::positions::table) is_long: Option<bool>,
    pub(in crate::account_views::positions::table) mark_px: Option<f64>,
    pub(in crate::account_views::positions::table) position_value: Option<f64>,
    pub(in crate::account_views::positions::table) upnl: Option<f64>,
    pub(in crate::account_views::positions::table) liq_px: Option<f64>,
    pub(in crate::account_views::positions::table) funding_since_open: Option<f64>,
    pub(in crate::account_views::positions::table) total_pnl: Option<f64>,
    pub(in crate::account_views::positions::table) leverage: u32,
}

impl TradingTerminal {
    pub(super) fn sorted_position_rows(
        &self,
        positions: &[account::AssetPosition],
    ) -> Vec<PositionRowData> {
        let mut row_data: Vec<PositionRowData> = positions
            .iter()
            .map(|ap| self.position_row_data(ap))
            .collect();

        row_data.sort_by(|a, b| {
            let cmp = match self.positions_sort_column {
                PositionsSortColumn::Symbol => a.coin.cmp(&b.coin),
                PositionsSortColumn::Side => {
                    a.is_long.cmp(&b.is_long).then_with(|| a.coin.cmp(&b.coin))
                }
                PositionsSortColumn::Size => {
                    optional_numeric_cmp(a.szi.map(f64::abs), b.szi.map(f64::abs))
                }
                PositionsSortColumn::Entry => optional_numeric_cmp(a.entry_px, b.entry_px),
                PositionsSortColumn::Liquidation => optional_numeric_cmp(a.liq_px, b.liq_px),
                PositionsSortColumn::Mark => optional_numeric_cmp(a.mark_px, b.mark_px),
                PositionsSortColumn::Value => {
                    optional_numeric_cmp(a.position_value, b.position_value)
                }
                PositionsSortColumn::UnrealizedPnl => optional_numeric_cmp(a.upnl, b.upnl),
                PositionsSortColumn::Funding => {
                    optional_numeric_cmp(a.funding_since_open, b.funding_since_open)
                }
                PositionsSortColumn::TotalPnl => optional_numeric_cmp(a.total_pnl, b.total_pnl),
                PositionsSortColumn::Leverage => a.leverage.cmp(&b.leverage),
            };

            if self.positions_sort_direction == config::SortDirection::Descending {
                cmp.reverse().then_with(|| a.coin.cmp(&b.coin))
            } else {
                cmp.then_with(|| a.coin.cmp(&b.coin))
            }
        });

        row_data
    }

    pub(in crate::account_views::positions::table) fn position_row_data(
        &self,
        ap: &account::AssetPosition,
    ) -> PositionRowData {
        let pos = &ap.position;
        let szi = parse_position_row_number(&pos.szi);
        let entry_px = parse_position_row_number(&pos.entry_px);
        let mark_px = self.resolve_mid_for_symbol(&pos.coin);
        let position_value = account::position_notional_from_mark_or_wire(
            szi,
            parse_position_row_number(&pos.position_value),
            mark_px,
        );
        let upnl = account::position_upnl_from_mark_or_wire(
            szi,
            entry_px,
            parse_position_row_number(&pos.unrealized_pnl),
            mark_px,
        );
        let funding_since_open = Self::position_funding_pnl(pos.cum_funding.as_ref());
        let total_pnl = upnl.map(|upnl| funding_since_open.map_or(upnl, |funding| upnl + funding));

        PositionRowData {
            ap: ap.clone(),
            coin: pos.coin.clone(),
            szi,
            entry_px,
            is_long: szi.map(|szi| szi >= 0.0),
            mark_px,
            position_value,
            upnl,
            liq_px: Self::parse_liquidation_px(ap),
            funding_since_open,
            total_pnl,
            leverage: pos.leverage.value,
        }
    }
}

fn parse_position_row_number(raw: &str) -> Option<f64> {
    parse_finite_number(raw)
}

fn numeric_cmp(left: f64, right: f64) -> std::cmp::Ordering {
    left.partial_cmp(&right)
        .unwrap_or(std::cmp::Ordering::Equal)
}

fn optional_numeric_cmp(left: Option<f64>, right: Option<f64>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => numeric_cmp(left, right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
