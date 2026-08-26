use crate::account_analytics::IncomeSnapshot;
use crate::account_views::portfolio::tokens;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::portfolio_state::IncomePaneView;
use iced::widget::{button, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_income_tabs(
        &self,
        data: Option<&IncomeSnapshot>,
    ) -> Element<'static, Message> {
        let token_count = data.map_or(0, |snapshot| snapshot.token_rows.len());
        let payment_count = data.map_or(0, |snapshot| snapshot.recent_hourly_payments.len());
        let active = self.income.view;

        row![
            income_tab(
                IncomePaneView::Overview.label().to_string(),
                IncomePaneView::Overview,
                active,
            ),
            income_tab(
                format!("{}  {token_count}", IncomePaneView::Tokens.label()),
                IncomePaneView::Tokens,
                active,
            ),
            income_tab(
                format!("{}  {payment_count}", IncomePaneView::Payments.label()),
                IncomePaneView::Payments,
                active,
            ),
        ]
        .spacing(3)
        .width(Fill)
        .into()
    }
}

fn income_tab(
    label: String,
    view: IncomePaneView,
    active: IncomePaneView,
) -> Element<'static, Message> {
    let selected = view == active;
    button(text(label).size(11).font(tokens::mono()).center())
        .on_press(Message::SetIncomePaneView(view))
        .padding([5, 9])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let background = if selected {
                tokens::accent_wash(theme)
            } else if hovered {
                Color {
                    a: 0.06,
                    ..tokens::text(theme)
                }
            } else {
                Color::TRANSPARENT
            };
            button::Style {
                background: Some(background.into()),
                text_color: if selected {
                    tokens::accent_soft(theme)
                } else {
                    tokens::muted(theme)
                },
                border: iced::Border {
                    color: if selected {
                        tokens::accent_border(theme)
                    } else {
                        Color::TRANSPARENT
                    },
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..button::Style::default()
            }
        })
        .into()
}
