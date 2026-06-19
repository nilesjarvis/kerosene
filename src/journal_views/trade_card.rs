mod components;
mod editor;
mod sections;
mod snapshot;

use self::sections::{
    journal_trade_card_details, journal_trade_card_header, push_journal_trade_notes,
};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::journal::{self, AggregatedTrade};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, container, text};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Trade Cards
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_journal_trade_card<'a>(
        &'a self,
        trade: &'a AggregatedTrade,
        current_time_ms: u64,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let is_open = trade.status == "OPEN";
        let status_color = if is_open {
            theme.palette().success
        } else {
            theme.palette().text
        };
        let denomination = self.display_denomination_context();

        let display_coin = self.display_coin_for_journal(&trade.coin);
        let opened_time_str = helpers::format_timestamp_exact(trade.start_time);
        let duration_ms = trade
            .end_time
            .unwrap_or(current_time_ms)
            .saturating_sub(trade.start_time);
        let duration_str = helpers::format_duration(duration_ms);
        let max_position_label = self.journal_max_position_label(trade);
        let note_key = journal::note_key_for_trade(&self.journal.entries, trade);
        let snapshot_expanded = self.journal.expanded_snapshot_trade_ids.contains(&trade.id);

        let header = journal_trade_card_header(
            &trade.coin,
            display_coin,
            trade.status.clone(),
            trade.pnl,
            status_color,
            &denomination,
            &theme,
        );
        let details = journal_trade_card_details(
            trade.id.clone(),
            note_key.clone(),
            snapshot_expanded,
            max_position_label,
            trade.fill_count,
            trade.fee,
            opened_time_str,
            duration_str,
            &denomination,
            &theme,
        );

        let mut card = Column::new().spacing(4).push(header).push(details);

        if !trade.basis_complete {
            card = card.push(
                text("Partial history: opening fills are outside the loaded data.")
                    .size(11)
                    .color(theme.palette().primary),
            );
        }

        if snapshot_expanded {
            card = card.push(self.view_journal_trade_snapshot(trade));
        }

        let is_editing = self
            .journal
            .edit_modes
            .get(&trade.id)
            .copied()
            .unwrap_or(false);

        if is_editing {
            card = self.push_journal_trade_editor(card, trade);
        } else if let Some(note) = journal::note_for_trade(&self.journal.entries, trade) {
            card = push_journal_trade_notes(card, note, &theme);
        }

        container(card)
            .width(Fill)
            .padding(12)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
                border: iced::Border {
                    color: theme.extended_palette().background.strong.color,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    pub(super) fn journal_max_position_label(&self, trade: &AggregatedTrade) -> String {
        let side_label = if trade.coin.starts_with('@') {
            "Spot"
        } else if trade.coin.starts_with('#') {
            "Outcome"
        } else if trade.is_long {
            "Long"
        } else {
            "Short"
        };
        let max_position = trade.max_position.abs();
        if self.is_outcome_coin(&trade.coin) {
            format!("{} {:.0}", side_label, max_position)
        } else {
            format!("{} {:.2}", side_label, max_position)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aggregated_trade(coin: &str, max_position: f64, is_long: bool) -> AggregatedTrade {
        AggregatedTrade {
            id: "trade-1".to_string(),
            legacy_note_ids: Vec::new(),
            coin: coin.to_string(),
            start_time: 0,
            end_time: None,
            max_position,
            volume: 0.0,
            fee: 0.0,
            pnl: 0.0,
            status: "OPEN".to_string(),
            fill_count: 1,
            avg_entry_price: 0.0,
            total_entry_notional: 0.0,
            total_entry_size: 0.0,
            is_long,
            basis_complete: true,
        }
    }

    #[test]
    fn journal_max_position_label_uses_whole_units_for_outcome_contracts() {
        let terminal = TradingTerminal::boot().0;

        assert_eq!(
            terminal.journal_max_position_label(&aggregated_trade("#950", 30.0, true)),
            "Outcome 30"
        );
    }

    #[test]
    fn journal_max_position_label_keeps_two_decimals_for_other_markets() {
        let terminal = TradingTerminal::boot().0;

        assert_eq!(
            terminal.journal_max_position_label(&aggregated_trade("BTC", 0.5, false)),
            "Short 0.50"
        );
        assert_eq!(
            terminal.journal_max_position_label(&aggregated_trade("@107", 1.25, true)),
            "Spot 1.25"
        );
    }
}
