mod header;
mod table;

use self::header::position_size_is_nonzero;
use super::table_helpers::{account_table_scroll, empty_account_table};
use crate::account::{self, AccountDataSection};
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::message::Message;
use crate::order_pending_indicators::ProjectedPositionDelta;

use iced::widget::text::Wrapping;
use iced::widget::{Column, column, container, responsive, row, rule, text};
use iced::{Color, Element, Fill, Theme};

pub(super) const POSITION_ACTION_WIDTH: f32 = 152.0;

// Columns whose content is bounded (a side label, a price, a leverage string)
// use fixed widths so they stay compact and aligned. The variable-content
// columns — Symbol plus Size, Value, uPnL and Total PnL, which can hold large
// values or an in-flight "projected size" label — are left as `Fill` in the
// view code: they split the pane's leftover width *equally*, so they widen as
// the pane grows (fitting larger values, with no hard 90px cap) and narrow as
// it shrinks. The reveal thresholds below keep that shared slack at no less than
// `MIN_FILL_WIDTH` whenever full-precision numbers are shown, so ordinary values
// never clip; only unusually large values or long projected labels clip, and
// only until the pane is widened.
pub(super) const POSITION_SIDE_WIDTH: f32 = 65.0;
pub(super) const POSITION_ENTRY_WIDTH: f32 = 90.0;
pub(super) const POSITION_LIQ_WIDTH: f32 = 90.0;
pub(super) const POSITION_MARK_WIDTH: f32 = 90.0;
pub(super) const POSITION_FUNDING_WIDTH: f32 = 90.0;
pub(super) const POSITION_LEVERAGE_WIDTH: f32 = 100.0;

// The natural width of a full-precision number cell. The reveal thresholds are
// derived so each `Fill` column keeps at least this much of the shared slack in
// Full mode, so revealing an optional column never squeezes the numbers below
// their natural width.
const MIN_FILL_WIDTH: f32 = 90.0;

// Row geometry, mirroring the layout built in `header.rs` / `position_row.rs`:
// a `row!` with `.spacing(ROW_SPACING)` inside a container padded 8px on each
// side. Shared with the layout tests.
const ROW_SPACING: f32 = 4.0;
const ROW_HORIZONTAL_PADDING: f32 = 16.0;

// Optional columns are revealed only once the pane is wide enough that, with the
// column shown, every `Fill` column still gets at least `MIN_FILL_WIDTH` of the
// shared slack — so revealing a column never squeezes the numbers or clips the
// close/NUKE action cell. Each threshold is therefore the sum of the visible
// fixed-column widths, the inter-column spacing, the row padding, and one
// `MIN_FILL_WIDTH` per `Fill` column. As the pane narrows the columns drop
// widest-budget first; Entry then Mark are the last fixed columns to go before
// only the essentials remain (Symbol, Side, Size, Value, uPnL, action), which
// still fit down to ~260px (numbers are compact/abbreviated by then). See the
// `fill_columns_stay_at_least_min_width_in_full_mode` and
// `fixed_columns_never_overflow_at_realistic_pane_widths` tests.
//
// At reveal there are 4 `Fill` columns (Symbol, Size, Value, uPnL); Total PnL
// adds a 5th. The `* ROW_SPACING` factor is (child count - 1) gaps.
const HIDE_LIQUIDATION_BELOW: f32 = POSITION_SIDE_WIDTH
    + POSITION_ENTRY_WIDTH
    + POSITION_LIQ_WIDTH
    + POSITION_MARK_WIDTH
    + POSITION_ACTION_WIDTH
    + 8.0 * ROW_SPACING // 9 children
    + ROW_HORIZONTAL_PADDING
    + 4.0 * MIN_FILL_WIDTH;
const HIDE_FUNDING_BELOW: f32 = POSITION_SIDE_WIDTH
    + POSITION_ENTRY_WIDTH
    + POSITION_LIQ_WIDTH
    + POSITION_MARK_WIDTH
    + POSITION_FUNDING_WIDTH
    + POSITION_ACTION_WIDTH
    + 9.0 * ROW_SPACING // 10 children
    + ROW_HORIZONTAL_PADDING
    + 4.0 * MIN_FILL_WIDTH;
const HIDE_LEVERAGE_BELOW: f32 = POSITION_SIDE_WIDTH
    + POSITION_ENTRY_WIDTH
    + POSITION_LIQ_WIDTH
    + POSITION_MARK_WIDTH
    + POSITION_FUNDING_WIDTH
    + POSITION_LEVERAGE_WIDTH
    + POSITION_ACTION_WIDTH
    + 10.0 * ROW_SPACING // 11 children
    + ROW_HORIZONTAL_PADDING
    + 4.0 * MIN_FILL_WIDTH;
const HIDE_TOTAL_PNL_BELOW: f32 = POSITION_SIDE_WIDTH
    + POSITION_ENTRY_WIDTH
    + POSITION_LIQ_WIDTH
    + POSITION_MARK_WIDTH
    + POSITION_FUNDING_WIDTH
    + POSITION_LEVERAGE_WIDTH
    + POSITION_ACTION_WIDTH
    + 11.0 * ROW_SPACING // 12 children
    + ROW_HORIZONTAL_PADDING
    + 5.0 * MIN_FILL_WIDTH; // Total PnL is itself a Fill column
const HIDE_ENTRY_BELOW: f32 = 560.0;
const HIDE_MARK_BELOW: f32 = 470.0;
const COMPACT_NUMBERS_BELOW: f32 = HIDE_LIQUIDATION_BELOW;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PositionColumnVisibility {
    pub(super) entry: bool,
    pub(super) liquidation: bool,
    pub(super) mark: bool,
    pub(super) funding: bool,
    pub(super) total_pnl: bool,
    pub(super) leverage: bool,
}

impl PositionColumnVisibility {
    fn for_width(width: f32) -> Self {
        Self {
            entry: width >= HIDE_ENTRY_BELOW,
            liquidation: width >= HIDE_LIQUIDATION_BELOW,
            mark: width >= HIDE_MARK_BELOW,
            funding: width >= HIDE_FUNDING_BELOW,
            total_pnl: width >= HIDE_TOTAL_PNL_BELOW,
            leverage: width >= HIDE_LEVERAGE_BELOW,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PositionNumberMode {
    Full,
    Compact,
}

impl PositionNumberMode {
    fn for_width(width: f32) -> Self {
        if width < COMPACT_NUMBERS_BELOW {
            Self::Compact
        } else {
            Self::Full
        }
    }

    pub(super) fn is_compact(self) -> bool {
        matches!(self, Self::Compact)
    }
}

impl TradingTerminal {
    pub(crate) fn view_positions(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_positions_for_width(size.width))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_positions_for_width(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let columns = PositionColumnVisibility::for_width(available_width);
        let number_mode = PositionNumberMode::for_width(available_width);
        let can_close = self.connected_address.is_some() && self.has_active_committed_agent_key();

        let account_positions = self.account_positions_with_outcomes();
        let all_position_coins: Vec<String> = account_positions
            .iter()
            .map(|ap| ap.position.coin.clone())
            .collect();
        let has_nuke_positions = can_close
            && account_positions.iter().any(|ap| {
                position_size_is_nonzero(&ap.position.szi) && self.is_perp_coin(&ap.position.coin)
            });
        let symbol_hidden_count = account_positions
            .iter()
            .filter(|ap| self.symbol_key_is_hidden(&ap.position.coin))
            .count();
        let visible_symbol_positions: Vec<account::AssetPosition> = account_positions
            .into_iter()
            .filter(|ap| !self.symbol_key_is_hidden(&ap.position.coin))
            .collect();
        let account_hidden_count = visible_symbol_positions
            .iter()
            .filter(|ap| self.position_is_hidden(&ap.position.coin))
            .count();
        let hidden_count = symbol_hidden_count + account_hidden_count;
        let positions: Vec<account::AssetPosition> = visible_symbol_positions
            .into_iter()
            .filter(|ap| self.show_hidden_positions || !self.position_is_hidden(&ap.position.coin))
            .collect();
        let warning = self
            .connected_order_account_snapshot()
            .and_then(|(_, data)| {
                data.completeness
                    .section_warning(AccountDataSection::Positions)
            });
        let opening_deltas = self.optimistic_opening_position_deltas(&all_position_coins);

        let header = self.view_positions_header(
            can_close,
            &positions,
            account_hidden_count,
            has_nuke_positions,
            &theme,
            columns,
        );

        if positions.is_empty() && opening_deltas.is_empty() {
            let msg = if let Some(warning) = warning {
                warning
            } else if hidden_count > 0 {
                "All open positions are hidden".to_string()
            } else if self.connected_address.is_some() {
                "No open positions".to_string()
            } else {
                "Connect wallet to view positions".to_string()
            };
            return empty_account_table(header, msg, &theme);
        }

        let rows = self.view_position_sections(&positions, can_close, &theme, columns, number_mode);
        let mut content = column![header].spacing(4);
        if let Some(warning) = warning {
            content = content.push(text(warning).size(11).color(theme.palette().warning));
        }
        let mut content = content.push(rule::horizontal(1)).push(rows);
        for delta in &opening_deltas {
            let symbol_label = self.display_name_for_symbol(&delta.symbol);
            let size_label = self.display_size_for_symbol(&delta.symbol, delta.signed_size.abs());
            content = content.push(opening_position_row(
                delta,
                symbol_label,
                size_label,
                &theme,
            ));
        }
        column![
            account_table_scroll(content),
            self.view_position_summary_bar(&positions, &theme, number_mode),
        ]
        .spacing(0)
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// In-flight market orders for symbols with no position at all
    /// (optimistic account updates): rendered as provisional "opening" lines.
    /// Filtered against every account position — visible or user-hidden — so
    /// an order on a hidden position never masquerades as a brand-new one.
    fn optimistic_opening_position_deltas(
        &self,
        all_position_coins: &[String],
    ) -> Vec<ProjectedPositionDelta> {
        self.optimistic_position_deltas()
            .into_iter()
            .filter(|delta| delta.signed_size.abs() > f64::EPSILON)
            .filter(|delta| !all_position_coins.contains(&delta.symbol))
            .filter(|delta| !self.symbol_key_is_hidden(&delta.symbol))
            .collect()
    }

    pub(crate) fn position_is_hidden(&self, coin: &str) -> bool {
        self.accounts
            .get(self.active_account_index)
            .and_then(|profile| self.hidden_positions_by_account.get(&profile.secret_id))
            .is_some_and(|hidden| hidden.contains(coin))
    }

    fn view_position_sections<'a>(
        &'a self,
        positions: &[account::AssetPosition],
        can_close: bool,
        theme: &Theme,
        columns: PositionColumnVisibility,
        number_mode: PositionNumberMode,
    ) -> Column<'a, Message> {
        let mut perp_positions = Vec::new();
        let mut spot_positions = Vec::new();
        let mut outcome_positions = Vec::new();
        for position in positions {
            if self.is_outcome_coin(&position.position.coin) {
                outcome_positions.push(position.clone());
            } else if self.is_spot_coin(&position.position.coin) {
                spot_positions.push(position.clone());
            } else {
                perp_positions.push(position.clone());
            }
        }

        let mut content = Column::new().spacing(4);
        if !perp_positions.is_empty() {
            content = content.push(self.view_position_rows(
                &perp_positions,
                can_close,
                theme,
                columns,
                number_mode,
            ));
        }

        if !spot_positions.is_empty() {
            if !perp_positions.is_empty() {
                content = content.push(rule::horizontal(1));
            }
            content = content
                .push(position_section_header("Spot", spot_positions.len(), theme))
                .push(self.view_position_rows(
                    &spot_positions,
                    can_close,
                    theme,
                    columns,
                    number_mode,
                ));
        }

        if !outcome_positions.is_empty() {
            if !perp_positions.is_empty() || !spot_positions.is_empty() {
                content = content.push(rule::horizontal(1));
            }
            content = content
                .push(position_section_header(
                    "Outcomes",
                    outcome_positions.len(),
                    theme,
                ))
                .push(self.view_position_rows(
                    &outcome_positions,
                    can_close,
                    theme,
                    columns,
                    number_mode,
                ));
        }

        content
    }
}

fn opening_position_row<'a>(
    delta: &ProjectedPositionDelta,
    symbol_label: String,
    size_label: String,
    theme: &Theme,
) -> Element<'a, Message> {
    container(
        text(opening_position_label(delta, &symbol_label, &size_label))
            .size(11)
            .color(theme.palette().primary)
            .wrapping(Wrapping::None),
    )
    .padding([4, 8])
    .into()
}

fn opening_position_label(
    delta: &ProjectedPositionDelta,
    symbol_label: &str,
    size_label: &str,
) -> String {
    let side = if delta.signed_size >= 0.0 {
        "buy"
    } else {
        "sell"
    };
    let price = delta
        .estimated_price
        .map(|px| format!(" @ ~{}", format_price(px)))
        .unwrap_or_default();
    format!("\u{27f3} {symbol_label} market {side} {size_label}{price} in flight\u{2026}")
}

fn position_section_header<'a>(
    label: &'static str,
    count: usize,
    theme: &Theme,
) -> Element<'a, Message> {
    let text_color = theme.extended_palette().background.weak.text;
    let badge_color = theme.palette().primary;
    container(
        row![
            text(label).size(11).color(text_color),
            container(text(count.to_string()).size(10).color(badge_color))
                .padding([1, 5])
                .style(move |_theme: &Theme| iced::widget::container::Style {
                    background: Some(
                        Color {
                            a: 0.12,
                            ..badge_color
                        }
                        .into(),
                    ),
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: 1.0,
                        color: Color {
                            a: 0.28,
                            ..badge_color
                        },
                    },
                    ..Default::default()
                }),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .padding(iced::Padding {
        top: 4.0,
        right: 8.0,
        bottom: 0.0,
        left: 8.0,
    })
    .into()
}

#[cfg(test)]
mod tests;
