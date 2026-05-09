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
            | Message::SetAddWidgetPlacement(_) => self.update_pane_menu(message),
            Message::AddTradingJournal => self.add_trading_journal_window(),
            Message::AddPortfolioPane
            | Message::AddIncomePane
            | Message::AddCalendarPane
            | Message::AddLiquidationsPane
            | Message::AddTrackedTradesPane
            | Message::AddOutcomesPane => self.add_widget_pane(message),
            _ => Task::none(),
        }
    }
}
