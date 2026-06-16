use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::message::Message;
use crate::order_execution::QuickOrderForm;
use iced::widget::{Row, button, row, text};
use iced::{Fill, Theme, color};

impl TradingTerminal {
    pub(in crate::order_views::quick_order) fn quick_order_action_row<'a>(
        &self,
        chart_id: ChartId,
        surface_id: ChartSurfaceId,
        form: &QuickOrderForm,
    ) -> Row<'a, Message> {
        let theme = self.theme();
        let snapshot = self.quick_order_submission_snapshot(chart_id, surface_id, form);
        let buy_btn = button(
            text("BUY")
                .size(12)
                .color(theme.palette().text)
                .center()
                .width(Fill),
        )
        .on_press(Message::SubmitQuickOrder {
            chart_id,
            is_buy: true,
            snapshot: snapshot.clone(),
        })
        .padding([6, 12])
        .width(Fill)
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.palette().success,
                _ => color!(0x30a050),
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

        let sell_btn = button(
            text("SELL")
                .size(12)
                .color(theme.palette().text)
                .center()
                .width(Fill),
        )
        .on_press(Message::SubmitQuickOrder {
            chart_id,
            is_buy: false,
            snapshot,
        })
        .padding([6, 12])
        .width(Fill)
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.palette().danger,
                _ => color!(0xdd3333),
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

        let close_btn = button(text("X").size(10).center())
            .on_press(Message::CloseQuickOrder(chart_id))
            .padding([3, 6])
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(color!(0x3a3a3a).into()),
                text_color: color!(0xaaaaaa),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        row![buy_btn, sell_btn, close_btn]
            .spacing(4)
            .align_y(iced::Alignment::Center)
    }
}
