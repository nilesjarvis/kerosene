use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::journal::{self, AggregatedTrade};
use crate::journal_views::style::journal_panel_style;
use crate::message::Message;
use iced::alignment;
use iced::widget::container as container_style;
use iced::widget::{Column, button, column, container, row, rule, text};
use iced::{Color, Element, Fill, Length, Theme};
use std::cmp::Ordering;

impl TradingTerminal {
    pub(super) fn filtered_journal_trades(&self) -> Vec<&AggregatedTrade> {
        let mut filtered_trades: Vec<_> = self
            .journal
            .trades
            .iter()
            .filter(|trade| !self.symbol_key_is_hidden(&trade.coin))
            .filter(|trade| self.journal.filter.matches_coin(&trade.coin))
            .collect();

        let current_time_ms = self.status_bar_now_ms;

        match self.journal.sort {
            journal::JournalSort::AssetAsc => {
                filtered_trades.sort_by(|a, b| {
                    self.display_coin_for_journal(&a.coin)
                        .cmp(&self.display_coin_for_journal(&b.coin))
                });
            }
            journal::JournalSort::AssetDesc => {
                filtered_trades.sort_by(|a, b| {
                    self.display_coin_for_journal(&b.coin)
                        .cmp(&self.display_coin_for_journal(&a.coin))
                });
            }
            journal::JournalSort::PositionDesc => {
                filtered_trades
                    .sort_by(|a, b| compare_f64_desc(a.max_position.abs(), b.max_position.abs()));
            }
            journal::JournalSort::PositionAsc => {
                filtered_trades
                    .sort_by(|a, b| compare_f64_asc(a.max_position.abs(), b.max_position.abs()));
            }
            journal::JournalSort::StatusAsc => {
                filtered_trades.sort_by(|a, b| a.status.cmp(&b.status));
            }
            journal::JournalSort::StatusDesc => {
                filtered_trades.sort_by(|a, b| b.status.cmp(&a.status));
            }
            journal::JournalSort::TimeDesc => {
                // Already sorted this way by aggregate_trades.
            }
            journal::JournalSort::TimeAsc => {
                filtered_trades.reverse();
            }
            journal::JournalSort::DurationDesc => {
                filtered_trades.sort_by(|a, b| {
                    trade_duration_ms(b, current_time_ms)
                        .cmp(&trade_duration_ms(a, current_time_ms))
                });
            }
            journal::JournalSort::DurationAsc => {
                filtered_trades.sort_by(|a, b| {
                    trade_duration_ms(a, current_time_ms)
                        .cmp(&trade_duration_ms(b, current_time_ms))
                });
            }
            journal::JournalSort::FillsDesc => {
                filtered_trades.sort_by(|a, b| b.fill_count.cmp(&a.fill_count));
            }
            journal::JournalSort::FillsAsc => {
                filtered_trades.sort_by(|a, b| a.fill_count.cmp(&b.fill_count));
            }
            journal::JournalSort::FeesDesc => {
                filtered_trades.sort_by(|a, b| compare_f64_desc(a.fee, b.fee));
            }
            journal::JournalSort::FeesAsc => {
                filtered_trades.sort_by(|a, b| compare_f64_asc(a.fee, b.fee));
            }
            journal::JournalSort::PnlDesc => {
                filtered_trades.sort_by(|a, b| compare_f64_desc(a.pnl, b.pnl));
            }
            journal::JournalSort::PnlAsc => {
                filtered_trades.sort_by(|a, b| compare_f64_asc(a.pnl, b.pnl));
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
        let duration_ms = trade_duration_ms(trade, current_time_ms);
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
        .width(Fill);

        container(table_divided_row(vec![
            table_text_cell(
                display_coin,
                ASSET_COL,
                theme.palette().primary,
                true,
                false,
            ),
            table_text_cell(
                self.journal_max_position_label(trade),
                POSITION_COL,
                theme.palette().text,
                true,
                false,
            ),
            table_text_cell(trade.status.clone(), STATUS_COL, status_color, true, false),
            table_text_cell(
                helpers::format_timestamp_exact(trade.start_time),
                OPENED_COL,
                muted,
                true,
                false,
            ),
            table_text_cell(
                helpers::format_duration(duration_ms),
                DURATION_COL,
                muted,
                true,
                false,
            ),
            table_text_cell(trade.fill_count.to_string(), FILLS_COL, muted, true, true),
            table_text_cell(
                denomination.format_value(trade.fee, 2),
                FEES_COL,
                muted,
                true,
                true,
            ),
            table_text_cell(
                denomination.format_signed_value(trade.pnl, 2),
                PNL_COL,
                pnl_color,
                true,
                true,
            ),
            container(actions)
                .width(Length::Fixed(ACTIONS_COL))
                .padding([5, 6])
                .into(),
        ]))
        .width(Fill)
        .height(Length::Fixed(TABLE_ROW_HEIGHT))
        .style(|_theme: &Theme| container_style::Style {
            background: Some(Color::TRANSPARENT.into()),
            ..Default::default()
        })
        .into()
    }
}

const ASSET_COL: f32 = 86.0;
const POSITION_COL: f32 = 92.0;
const STATUS_COL: f32 = 62.0;
const OPENED_COL: f32 = 116.0;
const DURATION_COL: f32 = 78.0;
const FILLS_COL: f32 = 48.0;
const FEES_COL: f32 = 82.0;
const PNL_COL: f32 = 92.0;
const ACTIONS_COL: f32 = 96.0;
const TABLE_ROW_HEIGHT: f32 = 30.0;
const TABLE_HEADER_HEIGHT: f32 = 28.0;

fn journal_trade_table_header(
    sort: journal::JournalSort,
    theme: &Theme,
) -> Element<'static, Message> {
    container(table_divided_row(vec![
        table_sort_header(JournalTradeTableColumn::Asset, theme, sort),
        table_sort_header(JournalTradeTableColumn::Position, theme, sort),
        table_sort_header(JournalTradeTableColumn::Status, theme, sort),
        table_sort_header(JournalTradeTableColumn::Opened, theme, sort),
        table_sort_header(JournalTradeTableColumn::Duration, theme, sort),
        table_sort_header(JournalTradeTableColumn::Fills, theme, sort),
        table_sort_header(JournalTradeTableColumn::Fees, theme, sort),
        table_sort_header(JournalTradeTableColumn::Pnl, theme, sort),
        table_header_cell("Actions", ACTIONS_COL, theme),
    ]))
    .width(Fill)
    .height(Length::Fixed(TABLE_HEADER_HEIGHT))
    .into()
}

fn table_header_cell(label: &'static str, width: f32, theme: &Theme) -> Element<'static, Message> {
    let label = text(label)
        .size(10)
        .font(crate::app_fonts::monospace_font())
        .color(theme.extended_palette().background.weak.text)
        .width(Fill)
        .align_x(alignment::Horizontal::Center);

    container(label)
        .width(Length::Fixed(width))
        .padding([6, 6])
        .into()
}

fn table_sort_header(
    column: JournalTradeTableColumn,
    theme: &Theme,
    active_sort: journal::JournalSort,
) -> Element<'static, Message> {
    let is_active = column.is_active(active_sort);
    let label = text(column.header_label(active_sort))
        .size(10)
        .font(crate::app_fonts::monospace_font())
        .color(if is_active {
            theme.palette().primary
        } else {
            theme.extended_palette().background.weak.text
        })
        .width(Fill)
        .align_x(alignment::Horizontal::Center);

    container(
        button(label)
            .on_press(Message::JournalSortChanged(column.next_sort(active_sort)))
            .padding(0)
            .width(Fill)
            .style(button::text),
    )
    .width(Length::Fixed(column.width()))
    .padding([6, 6])
    .into()
}

fn table_divided_row<'a>(cells: Vec<Element<'a, Message>>) -> iced::widget::Row<'a, Message> {
    let mut cells = cells.into_iter();
    let first = cells
        .next()
        .expect("journal trade table rows always have at least one cell");
    let mut row = row![first].spacing(0).align_y(iced::Alignment::Center);

    for cell in cells {
        row = row.push(table_column_divider()).push(cell);
    }

    row
}

fn table_column_divider<'a>() -> Element<'a, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.14,
            ..theme.extended_palette().background.strong.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .height(Fill)
    .width(Length::Fixed(1.0))
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JournalTradeTableColumn {
    Asset,
    Position,
    Status,
    Opened,
    Duration,
    Fills,
    Fees,
    Pnl,
}

impl JournalTradeTableColumn {
    fn label(self) -> &'static str {
        match self {
            Self::Asset => "Asset",
            Self::Position => "Position",
            Self::Status => "Status",
            Self::Opened => "Opened",
            Self::Duration => "Duration",
            Self::Fills => "Fills",
            Self::Fees => "Fees",
            Self::Pnl => "PnL",
        }
    }

    fn width(self) -> f32 {
        match self {
            Self::Asset => ASSET_COL,
            Self::Position => POSITION_COL,
            Self::Status => STATUS_COL,
            Self::Opened => OPENED_COL,
            Self::Duration => DURATION_COL,
            Self::Fills => FILLS_COL,
            Self::Fees => FEES_COL,
            Self::Pnl => PNL_COL,
        }
    }

    fn descending_sort(self) -> journal::JournalSort {
        match self {
            Self::Asset => journal::JournalSort::AssetDesc,
            Self::Position => journal::JournalSort::PositionDesc,
            Self::Status => journal::JournalSort::StatusDesc,
            Self::Opened => journal::JournalSort::TimeDesc,
            Self::Duration => journal::JournalSort::DurationDesc,
            Self::Fills => journal::JournalSort::FillsDesc,
            Self::Fees => journal::JournalSort::FeesDesc,
            Self::Pnl => journal::JournalSort::PnlDesc,
        }
    }

    fn ascending_sort(self) -> journal::JournalSort {
        match self {
            Self::Asset => journal::JournalSort::AssetAsc,
            Self::Position => journal::JournalSort::PositionAsc,
            Self::Status => journal::JournalSort::StatusAsc,
            Self::Opened => journal::JournalSort::TimeAsc,
            Self::Duration => journal::JournalSort::DurationAsc,
            Self::Fills => journal::JournalSort::FillsAsc,
            Self::Fees => journal::JournalSort::FeesAsc,
            Self::Pnl => journal::JournalSort::PnlAsc,
        }
    }

    fn default_sort(self) -> journal::JournalSort {
        match self {
            Self::Asset | Self::Status => self.ascending_sort(),
            Self::Position
            | Self::Opened
            | Self::Duration
            | Self::Fills
            | Self::Fees
            | Self::Pnl => self.descending_sort(),
        }
    }

    fn is_active(self, sort: journal::JournalSort) -> bool {
        sort == self.descending_sort() || sort == self.ascending_sort()
    }

    fn header_label(self, sort: journal::JournalSort) -> String {
        if sort == self.descending_sort() {
            format!("{} v", self.label())
        } else if sort == self.ascending_sort() {
            format!("{} ^", self.label())
        } else {
            self.label().to_string()
        }
    }

    fn next_sort(self, sort: journal::JournalSort) -> journal::JournalSort {
        if sort == self.descending_sort() {
            self.ascending_sort()
        } else if sort == self.ascending_sort() {
            self.descending_sort()
        } else {
            self.default_sort()
        }
    }
}

fn trade_duration_ms(trade: &AggregatedTrade, current_time_ms: u64) -> u64 {
    trade
        .end_time
        .unwrap_or(current_time_ms)
        .saturating_sub(trade.start_time)
}

fn compare_f64_asc(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

fn compare_f64_desc(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}

fn table_text_cell(
    value: String,
    width: f32,
    color: Color,
    monospace: bool,
    align_right: bool,
) -> Element<'static, Message> {
    let mut cell = text(value).size(11).color(color).width(Fill);
    if monospace {
        cell = cell.font(crate::app_fonts::monospace_font());
    }
    if align_right {
        cell = cell.align_x(alignment::Horizontal::Right);
    }
    container(cell)
        .width(Length::Fixed(width))
        .padding([5, 6])
        .into()
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

    fn sortable_trade(
        coin: &str,
        start_time: u64,
        max_position: f64,
        status: &str,
        duration_ms: u64,
        fill_count: usize,
        fee: f64,
        pnl: f64,
    ) -> AggregatedTrade {
        AggregatedTrade {
            end_time: Some(start_time + duration_ms),
            max_position,
            status: status.to_string(),
            fill_count,
            fee,
            pnl,
            ..trade(coin, start_time)
        }
    }

    fn sorted_coins(terminal: &mut TradingTerminal, sort: journal::JournalSort) -> Vec<String> {
        terminal.journal.sort = sort;
        terminal
            .filtered_journal_trades()
            .iter()
            .map(|trade| trade.coin.clone())
            .collect()
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

    #[test]
    fn journal_table_sorts_visible_columns() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.journal.trades = vec![
            sortable_trade("BTC", 300, 3.0, "CLOSED", 10, 2, 5.0, -4.0),
            sortable_trade("ETH", 200, 1.0, "OPEN", 40, 8, 1.0, 7.0),
            sortable_trade("SOL", 100, 2.0, "CLOSED", 20, 4, 3.0, 0.0),
        ];

        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::AssetAsc),
            vec!["BTC", "ETH", "SOL"]
        );
        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::AssetDesc),
            vec!["SOL", "ETH", "BTC"]
        );
        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::PositionDesc),
            vec!["BTC", "SOL", "ETH"]
        );
        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::StatusDesc),
            vec!["ETH", "BTC", "SOL"]
        );
        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::DurationDesc),
            vec!["ETH", "SOL", "BTC"]
        );
        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::FillsAsc),
            vec!["BTC", "SOL", "ETH"]
        );
        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::FeesDesc),
            vec!["BTC", "SOL", "ETH"]
        );
        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::PnlDesc),
            vec!["ETH", "SOL", "BTC"]
        );
    }
}
