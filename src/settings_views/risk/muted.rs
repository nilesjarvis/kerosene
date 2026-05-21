use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{Column, button, column, row, text, text_input};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(super) fn view_muted_ticker_input(&self) -> Element<'_, Message> {
        let current_theme = self.theme();
        let mute_button_message = Self::normalize_muted_ticker_input(&self.muted_ticker_input)
            .map(|_| Message::MuteTicker);

        column![
            text("Muted Tickers")
                .size(14)
                .color(current_theme.palette().text),
            row![
                text_input("Ticker or symbol key", &self.muted_ticker_input)
                    .style(helpers::text_input_style)
                    .on_input(Message::MutedTickerInputChanged)
                    .on_submit(Message::MuteTicker)
                    .size(12)
                    .padding(6)
                    .width(Fill),
                button(text("Mute").size(12))
                    .padding([6, 12])
                    .on_press_maybe(mute_button_message),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            text("Hidden from symbol search, watchlists, charts, account tables, wallet details, and Hydromancer feeds.")
                .size(11)
                .color(current_theme.extended_palette().background.weak.text),
        ]
        .spacing(8)
        .into()
    }

    pub(super) fn view_muted_ticker_list(&self) -> Column<'_, Message> {
        let current_theme = self.theme();
        let mut muted_list = Column::new().spacing(4);
        let sorted_muted = self.sorted_muted_tickers();

        if sorted_muted.is_empty() {
            muted_list = muted_list.push(
                text("No muted tickers")
                    .size(12)
                    .color(current_theme.extended_palette().background.weak.text),
            );
        } else {
            for ticker in sorted_muted {
                let remove_ticker = ticker.clone();
                muted_list = muted_list.push(
                    row![
                        text(ticker)
                            .size(12)
                            .font(crate::app_fonts::monospace_font())
                            .color(current_theme.palette().text)
                            .width(Fill),
                        button(text("Unmute").size(11))
                            .padding([4, 10])
                            .on_press(Message::UnmuteTicker(remove_ticker)),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                );
            }
        }

        muted_list
    }
}
