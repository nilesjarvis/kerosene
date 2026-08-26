use crate::account_views::portfolio::tokens;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Fill;
use iced::widget::{Column, column, container, rule, text};

impl TradingTerminal {
    pub(super) fn view_income_unavailable(&self) -> Column<'_, Message> {
        let theme = self.theme();
        column![
            self.view_income_tabs(None),
            rule::horizontal(1),
            container(
                text("Income is available in Portfolio Margin mode only")
                    .size(12)
                    .font(tokens::mono())
                    .color(tokens::muted(&theme)),
            )
            .width(Fill)
            .height(180)
            .center(Fill),
        ]
        .spacing(8)
    }

    pub(super) fn view_income_loading(&self) -> Column<'_, Message> {
        column![
            self.view_income_tabs(None),
            rule::horizontal(1),
            self.loading_overlay("Loading income...")
        ]
        .spacing(8)
    }

    pub(super) fn view_income_empty(&self) -> Column<'_, Message> {
        let theme = self.theme();
        let mut content = column![
            self.view_income_tabs(None),
            rule::horizontal(1),
            container(
                text("No income data available")
                    .size(12)
                    .font(tokens::mono())
                    .color(tokens::muted(&theme))
            )
            .width(Fill)
            .height(200)
            .center(Fill),
        ]
        .spacing(8);

        if let Some(err) = &self.income.last_error {
            content = content.push(
                text(format!("Stale data: {err}"))
                    .size(11)
                    .font(tokens::mono())
                    .color(theme.palette().primary),
            );
        }

        content
    }
}
