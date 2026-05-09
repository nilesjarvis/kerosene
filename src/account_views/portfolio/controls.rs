use crate::app_state::TradingTerminal;
use crate::helpers::{timeframe_button, vertical_spacer};
use crate::message::Message;
use crate::portfolio_state::{PORTFOLIO_WINDOWS, PortfolioScope};
use iced::Element;
use iced::widget::{button, container, row, rule, scrollable, text};

impl TradingTerminal {
    pub(super) fn view_portfolio_title(&self) -> Element<'static, Message> {
        row![
            text("Portfolio PnL").size(13),
            vertical_spacer(),
            button(text("Refresh").size(10))
                .on_press(Message::RefreshPortfolio)
                .padding([2, 8])
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }

    pub(super) fn view_portfolio_window_controls(&self) -> Element<'static, Message> {
        let scope_all = timeframe_button(
            "All PnL",
            self.portfolio.scope == PortfolioScope::All,
            Message::SetPortfolioScope(PortfolioScope::All),
        );
        let scope_perp = timeframe_button(
            "Perp PnL",
            self.portfolio.scope == PortfolioScope::Perp,
            Message::SetPortfolioScope(PortfolioScope::Perp),
        );

        let mut window_row = row![scope_all, scope_perp].spacing(4);
        window_row = window_row.push(container(rule::vertical(1)).height(16).width(8));
        for &window in PORTFOLIO_WINDOWS {
            window_row = window_row.push(timeframe_button(
                window.label(),
                self.portfolio.window == window,
                Message::SetPortfolioWindow(window),
            ));
        }

        scrollable(window_row)
            .direction(iced::widget::scrollable::Direction::Horizontal(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4.0)
                    .scroller_width(4.0)
                    .margin(0.0),
            ))
            .into()
    }
}
