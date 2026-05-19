use crate::account_state::BottomTab;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Row, Space, button, column, container, row, rule, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Account Bottom Tabs
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_bottom_tabs(&self, active_tab: BottomTab) -> Element<'_, Message> {
        let tabs = Row::new()
            .push(bottom_tab_button(
                "Positions",
                active_tab == BottomTab::Positions,
                Message::SwitchBottomTab(BottomTab::Positions),
            ))
            .push(bottom_tab_separator())
            .push(bottom_tab_button(
                "Open Orders",
                active_tab == BottomTab::OpenOrders,
                Message::SwitchBottomTab(BottomTab::OpenOrders),
            ))
            .push(bottom_tab_separator())
            .push(bottom_tab_button(
                "Balances",
                active_tab == BottomTab::Balances,
                Message::SwitchBottomTab(BottomTab::Balances),
            ))
            .push(bottom_tab_separator())
            .push(bottom_tab_button(
                "Trade History",
                active_tab == BottomTab::TradeHistory,
                Message::SwitchBottomTab(BottomTab::TradeHistory),
            ))
            .push(bottom_tab_separator())
            .push(bottom_tab_button(
                "Funding",
                active_tab == BottomTab::FundingHistory,
                Message::SwitchBottomTab(BottomTab::FundingHistory),
            ))
            .push(Space::new().width(Fill))
            .push(bottom_tab_separator())
            .push(bottom_journal_button())
            .width(Fill)
            .spacing(0)
            .align_y(iced::Alignment::Center);

        let tabs = bottom_tab_strip(tabs);

        let body: Element<Message> = match active_tab {
            BottomTab::Positions => self.view_positions(),
            BottomTab::OpenOrders => self.view_open_orders(),
            BottomTab::Balances => self.view_balances(),
            BottomTab::TradeHistory => self.view_trade_history(),
            BottomTab::FundingHistory => self.view_funding_history(),
        };

        let content = column![
            container(tabs).padding(iced::Padding {
                top: 10.0,
                right: 10.0,
                bottom: 0.0,
                left: 10.0
            }),
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

fn bottom_tab_strip<'a>(content: Row<'a, Message>) -> Element<'a, Message> {
    container(column![content, bottom_tab_bottom_separator()].spacing(0))
        .width(Fill)
        .style(|theme: &Theme| {
            let background = Color {
                a: 0.04,
                ..theme.extended_palette().background.weak.color
            };
            container::Style {
                background: Some(background.into()),
                ..Default::default()
            }
        })
        .into()
}

fn bottom_tab_button(label: &'static str, active: bool, msg: Message) -> Element<'static, Message> {
    button(text(label).size(11).center())
        .on_press(msg)
        .padding([4, 10])
        .style(move |theme: &Theme, status| {
            let background = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ if active => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(background.into()),
                text_color: if active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 0.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            }
        })
        .into()
}

fn bottom_journal_button() -> Element<'static, Message> {
    button(
        row![text("Journal").size(12), text("\u{2197}").size(12)]
            .spacing(4)
            .align_y(iced::Alignment::Center),
    )
    .on_press(Message::AddTradingJournal)
    .padding([4, 12])
    .style(move |t: &Theme, status| {
        let background = match status {
            button::Status::Hovered => t.extended_palette().background.strong.color,
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(background.into()),
            text_color: t.extended_palette().background.weak.text,
            border: iced::Border {
                radius: 0.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..Default::default()
        }
    })
    .into()
}

fn bottom_tab_separator() -> Element<'static, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.12,
            ..theme.extended_palette().background.weak.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .height(18)
    .width(1)
    .into()
}

fn bottom_tab_bottom_separator() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.12,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}
