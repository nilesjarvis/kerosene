use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::journal::{self, AggregatedTrade};
use crate::journal_views::style::journal_panel_style;
use crate::message::Message;
use iced::alignment;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, button, column, container, row, rule, text};
use iced::{Color, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(super) fn filtered_journal_trades(&self) -> Vec<&AggregatedTrade> {
        let mut filtered_trades: Vec<_> = self
            .journal
            .trades
            .iter()
            .filter(|trade| !self.symbol_key_is_hidden(&trade.coin))
            .filter(|trade| self.journal.filter.matches_coin(&trade.coin))
            .collect();

        match self.journal.sort {
            journal::JournalSort::TimeDesc => {
                // Already sorted this way by aggregate_trades.
            }
            journal::JournalSort::TimeAsc => {
                filtered_trades.reverse();
            }
            journal::JournalSort::PnlDesc => {
                filtered_trades.sort_by(|a, b| {
                    b.pnl
                        .partial_cmp(&a.pnl)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            journal::JournalSort::PnlAsc => {
                filtered_trades.sort_by(|a, b| {
                    a.pnl
                        .partial_cmp(&b.pnl)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        filtered_trades
    }

    pub(super) fn view_journal_fetching_history_row<'a>(
        &self,
        theme: &Theme,
    ) -> Element<'a, Message> {
        container(
            text("Fetching historical trades...")
                .size(12)
                .color(theme.palette().success),
        )
        .width(Fill)
        .padding(12)
        .center_x(Fill)
        .into()
    }

    pub(super) fn view_journal_trade_table<'a>(
        &'a self,
        trades: &[&'a AggregatedTrade],
        current_time_ms: u64,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let mut table = column![journal_trade_table_header(self.journal.sort, &theme)]
            .spacing(0)
            .push(rule::horizontal(1));

        for trade in trades.iter().copied() {
            table = table.push(self.view_journal_trade_table_row(trade, current_time_ms, &theme));

            if self.journal.expanded_snapshot_trade_ids.contains(&trade.id) {
                table = table.push(
                    container(self.view_journal_trade_snapshot(trade))
                        .padding([4, 8])
                        .width(Fill),
                );
            }

            if self
                .journal
                .edit_modes
                .get(&trade.id)
                .copied()
                .unwrap_or(false)
            {
                table = table.push(
                    container(self.push_journal_trade_editor(Column::new().spacing(4), trade))
                        .padding([4, 8])
                        .width(Fill),
                );
            }

            table = table.push(rule::horizontal(1));
        }

        if self.journal.loading {
            table = table.push(self.view_journal_fetching_history_row(&theme));
        }

        container(table)
            .width(Fill)
            .style(journal_panel_style)
            .into()
    }

    fn view_journal_trade_table_row<'a>(
        &'a self,
        trade: &'a AggregatedTrade,
        current_time_ms: u64,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let denomination = self.display_denomination_context();
        let display_coin = self.display_coin_for_journal(&trade.coin);
        let duration_ms = trade
            .end_time
            .unwrap_or(current_time_ms)
            .saturating_sub(trade.start_time);
        let note_key = journal::note_key_for_trade(&self.journal.entries, trade);
        let snapshot_expanded = self.journal.expanded_snapshot_trade_ids.contains(&trade.id);
        let status_color = if trade.status == "OPEN" {
            theme.palette().success
        } else {
            theme.palette().text
        };
        let pnl_color = journal_pnl_color(trade.pnl, theme);
        let muted = theme.extended_palette().background.weak.text;

        let actions = row![
            table_action_button(
                if snapshot_expanded { "Hide" } else { "Chart" },
                Message::JournalSnapshotToggle(trade.id.clone()),
            ),
            table_action_button(
                if note_key.is_some() { "Note" } else { "+ Note" },
                Message::JournalEditStart(trade.id.clone(), note_key),
            ),
        ]
        .spacing(4)
        .width(Length::Fixed(ACTIONS_COL));

        container(
            row![
                table_text_cell(
                    display_coin,
                    ASSET_COL,
                    theme.palette().primary,
                    true,
                    false
                ),
                table_text_cell(
                    self.journal_max_position_label(trade),
                    POSITION_COL,
                    theme.palette().text,
                    true,
                    false
                ),
                table_text_cell(trade.status.clone(), STATUS_COL, status_color, true, false),
                table_text_cell(
                    helpers::format_timestamp_exact(trade.start_time),
                    OPENED_COL,
                    muted,
                    true,
                    false
                ),
                table_text_cell(
                    helpers::format_duration(duration_ms),
                    DURATION_COL,
                    muted,
                    true,
                    false
                ),
                table_text_cell(trade.fill_count.to_string(), FILLS_COL, muted, true, true),
                table_text_cell(
                    denomination.format_value(trade.fee, 2),
                    FEES_COL,
                    muted,
                    true,
                    true
                ),
                table_text_cell(
                    denomination.format_signed_value(trade.pnl, 2),
                    PNL_COL,
                    pnl_color,
                    true,
                    true
                ),
                actions,
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .width(Fill)
        .padding([5, 8])
        .style(|_theme: &Theme| container_style::Style {
            background: Some(Color::TRANSPARENT.into()),
            ..Default::default()
        })
        .into()
    }
}

const ASSET_COL: f32 = 86.0;
const POSITION_COL: f32 = 84.0;
const STATUS_COL: f32 = 60.0;
const OPENED_COL: f32 = 106.0;
const DURATION_COL: f32 = 70.0;
const FILLS_COL: f32 = 38.0;
const FEES_COL: f32 = 74.0;
const PNL_COL: f32 = 92.0;
const ACTIONS_COL: f32 = 96.0;

fn journal_trade_table_header(
    sort: journal::JournalSort,
    theme: &Theme,
) -> Element<'static, Message> {
    container(
        row![
            table_header_cell("Asset", ASSET_COL, theme),
            table_header_cell("Position", POSITION_COL, theme),
            table_header_cell("Status", STATUS_COL, theme),
            table_sort_header(
                time_header_label(sort),
                OPENED_COL,
                next_time_sort(sort),
                theme,
                false
            ),
            table_header_cell("Duration", DURATION_COL, theme),
            table_header_cell("Fills", FILLS_COL, theme),
            table_header_cell("Fees", FEES_COL, theme),
            table_sort_header(
                pnl_header_label(sort),
                PNL_COL,
                next_pnl_sort(sort),
                theme,
                true
            ),
            Space::new().width(Length::Fixed(ACTIONS_COL)),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .padding([6, 8])
    .into()
}

fn table_header_cell(label: &'static str, width: f32, theme: &Theme) -> Element<'static, Message> {
    text(label)
        .size(10)
        .font(crate::app_fonts::monospace_font())
        .color(theme.extended_palette().background.weak.text)
        .width(Length::Fixed(width))
        .into()
}

fn table_sort_header(
    label: String,
    width: f32,
    sort: journal::JournalSort,
    theme: &Theme,
    align_right: bool,
) -> Element<'static, Message> {
    let mut label = text(label)
        .size(10)
        .font(crate::app_fonts::monospace_font())
        .color(theme.extended_palette().background.weak.text)
        .width(Length::Fixed(width));
    if align_right {
        label = label.align_x(alignment::Horizontal::Right);
    }

    button(label)
        .on_press(Message::JournalSortChanged(sort))
        .padding(0)
        .style(button::text)
        .into()
}

fn table_text_cell(
    value: String,
    width: f32,
    color: Color,
    monospace: bool,
    align_right: bool,
) -> Element<'static, Message> {
    let mut cell = text(value)
        .size(11)
        .color(color)
        .width(Length::Fixed(width));
    if monospace {
        cell = cell.font(crate::app_fonts::monospace_font());
    }
    if align_right {
        cell = cell.align_x(alignment::Horizontal::Right);
    }
    cell.into()
}

fn table_action_button(label: &'static str, message: Message) -> Element<'static, Message> {
    button(text(label).size(10))
        .on_press(message)
        .padding([2, 4])
        .style(button::text)
        .into()
}

fn journal_pnl_color(pnl: f64, theme: &Theme) -> Color {
    if pnl > 0.0 {
        theme.palette().success
    } else if pnl < 0.0 {
        theme.palette().danger
    } else {
        theme.palette().text
    }
}

fn time_header_label(sort: journal::JournalSort) -> String {
    match sort {
        journal::JournalSort::TimeDesc => "Opened v".to_string(),
        journal::JournalSort::TimeAsc => "Opened ^".to_string(),
        _ => "Opened".to_string(),
    }
}

fn pnl_header_label(sort: journal::JournalSort) -> String {
    match sort {
        journal::JournalSort::PnlDesc => "PnL v".to_string(),
        journal::JournalSort::PnlAsc => "PnL ^".to_string(),
        _ => "PnL".to_string(),
    }
}

fn next_time_sort(sort: journal::JournalSort) -> journal::JournalSort {
    match sort {
        journal::JournalSort::TimeDesc => journal::JournalSort::TimeAsc,
        _ => journal::JournalSort::TimeDesc,
    }
}

fn next_pnl_sort(sort: journal::JournalSort) -> journal::JournalSort {
    match sort {
        journal::JournalSort::PnlDesc => journal::JournalSort::PnlAsc,
        _ => journal::JournalSort::PnlDesc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trade(coin: &str, start_time: u64) -> AggregatedTrade {
        AggregatedTrade {
            id: format!("{coin}-{start_time}"),
            legacy_note_ids: Vec::new(),
            coin: coin.to_string(),
            start_time,
            end_time: Some(start_time),
            max_position: 1.0,
            volume: 100.0,
            fee: 1.0,
            pnl: 1.0,
            status: "CLOSED".to_string(),
            fill_count: 1,
            avg_entry_price: 100.0,
            total_entry_notional: 100.0,
            total_entry_size: 1.0,
            is_long: true,
            basis_complete: true,
        }
    }

    #[test]
    fn journal_filters_partition_perp_spot_and_outcome_trades() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.journal.trades = vec![trade("BTC", 3), trade("@107", 2), trade("#950", 1)];

        terminal.journal.filter = journal::JournalFilter::All;
        assert_eq!(
            terminal
                .filtered_journal_trades()
                .iter()
                .map(|trade| trade.coin.as_str())
                .collect::<Vec<_>>(),
            vec!["BTC", "@107", "#950"]
        );

        terminal.journal.filter = journal::JournalFilter::Perp;
        assert_eq!(
            terminal
                .filtered_journal_trades()
                .iter()
                .map(|trade| trade.coin.as_str())
                .collect::<Vec<_>>(),
            vec!["BTC"]
        );

        terminal.journal.filter = journal::JournalFilter::Spot;
        assert_eq!(
            terminal
                .filtered_journal_trades()
                .iter()
                .map(|trade| trade.coin.as_str())
                .collect::<Vec<_>>(),
            vec!["@107"]
        );

        terminal.journal.filter = journal::JournalFilter::Outcome;
        assert_eq!(
            terminal
                .filtered_journal_trades()
                .iter()
                .map(|trade| trade.coin.as_str())
                .collect::<Vec<_>>(),
            vec!["#950"]
        );
    }
}
