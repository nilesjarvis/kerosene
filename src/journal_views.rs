mod analytics;
mod chrome;
mod cockpit;
mod detail;
mod status;
mod style;
mod summary;
mod trade_card;
mod trade_list;
mod trades;

use crate::app_state::TradingTerminal;
use crate::journal_views::style::{
    journal_hairline, journal_muted, journal_rule_style, journal_window_style,
};
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Column, column, container, row, rule, text};
use iced::{Border, Color, Element, Fill, Length, Theme};

const TRADE_LIST_WIDTH: f32 = 404.0;

// ---------------------------------------------------------------------------
// Trading journal — fixed-chrome master/detail window
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_journal(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let filtered = self.filtered_journal_trades();
        let kpis = analytics::journal_kpis(&filtered, self.journal.include_fees_in_pnl);
        let (visible_fill_count, visible_trade_count) = self.journal_visible_counts();

        let body: Element<'_, Message> = if self.journal.loading && self.journal.trades.is_empty() {
            journal_center_message("Loading trades…".to_string(), theme.palette().success)
        } else if let Some(error) = &self.journal.error {
            journal_center_message(format!("Error: {error}"), theme.palette().danger)
        } else if self.journal.trades.is_empty() {
            journal_center_message("No trades found.".to_string(), journal_muted(&theme))
        } else {
            // Look up the selection within the *filtered* set so the detail
            // pane stays consistent with the visible list (a selection hidden
            // by the current filter falls back to the cockpit).
            let selected_trade = self
                .journal
                .selected_trade_id
                .as_ref()
                .and_then(|id| filtered.iter().copied().find(|trade| &trade.id == id));

            let right: Element<'_, Message> = match selected_trade {
                Some(trade) => self.view_journal_detail(trade, &kpis),
                None => self.view_journal_cockpit(&filtered),
            };

            row![
                container(self.view_journal_trade_list(&filtered, kpis.r_unit))
                    .width(Length::Fixed(TRADE_LIST_WIDTH))
                    .height(Fill),
                rule::vertical(1).style(journal_rule_style),
                container(right).width(Fill).height(Fill),
            ]
            .width(Fill)
            .height(Fill)
            .into()
        };

        let mut chrome: Column<'_, Message> = column![
            self.view_journal_title_bar(),
            rule::horizontal(1).style(journal_rule_style),
            self.view_journal_toolbar(visible_fill_count, visible_trade_count),
            rule::horizontal(1).style(journal_rule_style),
            self.view_journal_kpi_strip(&kpis),
            rule::horizontal(1).style(journal_rule_style),
        ]
        .width(Fill)
        .height(Fill);

        // Surface data-quality warnings (partial history, pagination gaps,
        // "showing cached data") that the redesign would otherwise drop.
        if let Some(warning) = self.journal.warning.as_deref() {
            chrome = chrome
                .push(journal_warning_bar(warning))
                .push(rule::horizontal(1).style(journal_rule_style));
        }

        chrome = chrome.push(container(body).width(Fill).height(Fill));

        container(chrome)
            .width(Fill)
            .height(Fill)
            .style(journal_window_style)
            .into()
    }
}

fn journal_warning_bar(warning: &str) -> Element<'_, Message> {
    container(
        text(warning.to_string())
            .size(11)
            .font(crate::app_fonts::monospace_font()),
    )
    .width(Fill)
    .padding([6, 16])
    .style(|theme: &Theme| container_style::Style {
        background: Some(
            Color {
                a: 0.08,
                ..theme.palette().warning
            }
            .into(),
        ),
        text_color: Some(theme.palette().warning),
        border: Border {
            color: journal_hairline(theme),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn journal_center_message(message: String, color: Color) -> Element<'static, Message> {
    container(
        text(message)
            .size(14)
            .font(crate::app_fonts::monospace_font())
            .color(color),
    )
    .width(Fill)
    .height(Fill)
    .center(Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{
        AggregatedTrade, JournalAttributedFill, JournalNote, JournalTradeDetails,
    };

    fn sample_trade(coin: &str, pnl: f64, is_long: bool, start_time: u64) -> AggregatedTrade {
        AggregatedTrade {
            id: format!("perp:{coin}:{start_time}"),
            legacy_note_ids: Vec::new(),
            coin: coin.to_string(),
            start_time,
            end_time: Some(start_time + 7_200_000),
            max_position: 0.4,
            volume: 1_000.0,
            fee: 12.5,
            pnl,
            status: "CLOSED".to_string(),
            fill_count: 4,
            avg_entry_price: 2_041.10,
            total_entry_notional: 816.0,
            total_entry_size: 0.4,
            is_long,
            basis_complete: true,
        }
    }

    fn details_with_fills(trade: &AggregatedTrade) -> JournalTradeDetails {
        let fill = |price: f64, time_ms: u64| JournalAttributedFill {
            identity: crate::journal::FillIdentity {
                time: time_ms,
                tid: time_ms,
                oid: time_ms,
                hash: "0x1".to_string(),
                coin: trade.coin.clone(),
                side: "B".to_string(),
                px: price.to_string(),
                sz: "0.2".to_string(),
            },
            time_ms,
            price,
            raw_size: 0.2,
            attributed_size: 0.2,
            side: "B".to_string(),
            role: crate::journal::JournalAttributedFillRole::Increase,
            fee: 1.0,
            closed_pnl: 0.0,
        };
        JournalTradeDetails {
            trade_id: trade.id.clone(),
            coin: trade.coin.clone(),
            attributed_fills: vec![
                fill(2_041.10, trade.start_time),
                fill(2_108.65, trade.start_time + 3_600_000),
            ],
        }
    }

    fn terminal_with_trades() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        let trades = vec![
            sample_trade("BTC", 1_240.55, true, 1_700_000_000_000),
            sample_trade("ETH", -96.40, false, 1_700_000_200_000),
            sample_trade("@107", -133.47, true, 1_700_000_400_000),
        ];
        for trade in &trades {
            terminal
                .journal
                .trade_details
                .insert(trade.id.clone(), details_with_fills(trade));
        }
        terminal.journal.trades = trades;
        terminal.journal.last_refresh_time = Some(1_700_000_500_000);
        terminal
    }

    #[test]
    fn journal_view_builds_cockpit_detail_and_editor_states() {
        let mut terminal = terminal_with_trades();
        let selected = terminal.journal.trades[0].id.clone();

        // Cockpit (no trade selected) builds the full analytics tree.
        let _ = terminal.view_journal();

        // Detail inspector for the selected trade.
        terminal.journal.selected_trade_id = Some(selected.clone());
        let _ = terminal.view_journal();

        // Reflection editor with a tag buffer in flight.
        terminal.journal.edit_modes.insert(selected.clone(), true);
        terminal.journal.edit_buffers.insert(
            selected.clone(),
            JournalNote {
                open: "Breakout thesis".to_string(),
                close: String::new(),
                tags: vec!["breakout".to_string(), "momentum".to_string()],
            },
        );
        terminal
            .journal
            .edit_tag_raw
            .insert(selected, "breakout momentum".to_string());
        let _ = terminal.view_journal();
    }

    #[test]
    fn journal_view_handles_empty_and_filtered_states() {
        let empty = TradingTerminal::boot().0;
        // No trades.
        let _ = empty.view_journal();

        // Spot-only filter with perp/spot mix.
        let mut terminal = terminal_with_trades();
        terminal.journal.filter = crate::journal::JournalFilter::Spot;
        let _ = terminal.view_journal();
    }
}
