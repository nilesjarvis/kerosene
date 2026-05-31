mod journal;
mod menu;
mod widgets;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_panes(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SwitchBottomTab(_)
            | Message::CloseAllMenus
            | Message::ToggleAddWidgetMenu
            | Message::ToggleLayoutMenu
            | Message::ToggleTickerTape
            | Message::SetAddWidgetPlacement(_) => self.update_pane_menu(message),
            Message::AddTradingJournal => self.add_trading_journal_window(),
            Message::AddPortfolioPane
            | Message::AddIncomePane
            | Message::AddCalendarPane
            | Message::AddLiquidationsPane
            | Message::AddLiquidationsDistributionPane
            | Message::AddTrackedTradesPane
            | Message::AddAdvancedOrdersPane
            | Message::AddOutcomesPane
            | Message::AddHypeEtfsPane
            | Message::AddHypeUnstakingQueuePane => self.add_widget_pane(message),
            _ => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeroseneConfig;
    use crate::pane_state::PaneKind;

    #[test]
    fn liquidations_distribution_add_message_opens_pane() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.add_widget_menu_open = true;

        let _task = terminal.update_panes(Message::AddLiquidationsDistributionPane);

        assert!(!terminal.add_widget_menu_open);
        assert!(terminal.pane_is_open(|kind| matches!(kind, PaneKind::LiquidationsDistribution)));
    }
}
