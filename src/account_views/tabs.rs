use crate::account_state::BottomTab;
use crate::app_state::TradingTerminal;
use crate::helpers::tab_button;
use crate::message::Message;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Account Bottom Tabs
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_bottom_tabs(&self, active_tab: BottomTab) -> Element<'_, Message> {
        let positions_btn = tab_button(
            "Positions",
            active_tab == BottomTab::Positions,
            Message::SwitchBottomTab(BottomTab::Positions),
        );
        let orders_btn = tab_button(
            "Open Orders",
            active_tab == BottomTab::OpenOrders,
            Message::SwitchBottomTab(BottomTab::OpenOrders),
        );
        let balances_btn = tab_button(
            "Balances",
            active_tab == BottomTab::Balances,
            Message::SwitchBottomTab(BottomTab::Balances),
        );
        let history_btn = tab_button(
            "Trade History",
            active_tab == BottomTab::TradeHistory,
            Message::SwitchBottomTab(BottomTab::TradeHistory),
        );
        let funding_btn = tab_button(
            "Funding",
            active_tab == BottomTab::FundingHistory,
            Message::SwitchBottomTab(BottomTab::FundingHistory),
        );
        let journal_btn = button(
            row![text("Journal").size(12), text("\u{2197}").size(12)]
                .spacing(4)
                .align_y(iced::Alignment::Center),
        )
        .on_press(Message::AddTradingJournal)
        .padding([4, 12])
        .style(move |t: &Theme, status| {
            let mut style = button::secondary(t, status);
            style.background = Some(t.extended_palette().background.weak.color.into());
            style.text_color = t.palette().text;
            style
        });

        let tabs = row![
            positions_btn,
            orders_btn,
            balances_btn,
            history_btn,
            funding_btn,
            Space::new().width(Fill),
            journal_btn,
        ]
        .spacing(4);

        let body: Element<Message> = match active_tab {
            BottomTab::Positions => self.view_positions(),
            BottomTab::OpenOrders => self.view_open_orders(),
            BottomTab::Balances => self.view_balances(),
            BottomTab::TradeHistory => self.view_trade_history(),
            BottomTab::FundingHistory => self.view_funding_history(),
        };

        let divider = container(Space::new().width(Fill).height(2)).style(|theme: &Theme| {
            iced::widget::container::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                ..Default::default()
            }
        });

        let content = column![
            container(tabs).padding(iced::Padding {
                top: 10.0,
                right: 10.0,
                bottom: 0.0,
                left: 10.0
            }),
            divider,
            container(body)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 10.0,
                    bottom: 10.0,
                    left: 10.0
                })
                .width(Fill)
                .height(Fill)
        ]
        .spacing(6);

        container(content).width(Fill).height(Fill).into()
    }
}
