use crate::app_state::TradingTerminal;
use crate::canvas_state::WorkspaceId;
use crate::message::Message;
use crate::order_execution::{existing_tpsl_kind, position_trigger_order};
use crate::signing::TpslKind;
use iced::widget::{
    Space, button, column, container, mouse_area, opaque, row, scrollable, stack, text, text_input,
};
use iced::{Color, Element, Fill, Theme};

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn view_position_tpsl_modal(
        &self,
        workspace: WorkspaceId,
    ) -> Option<Element<'_, Message>> {
        let form = self.position_tpsl.form.as_ref().filter(|form| {
            form.visible
                && form.workspace == workspace
                && self.connected_order_account_matches(&form.account_address)
        })?;
        let theme = self.theme();
        let busy = self.has_pending_trading_request() || self.account_loading;
        let mut content = column![
            row![
                text(format!(
                    "{} · TP/SL",
                    self.display_name_for_symbol(&form.coin)
                ))
                .size(15),
                Space::new().width(Fill),
                button(text("×").size(16))
                    .on_press(Message::ClosePositionTpsl)
                    .style(button::text),
            ]
            .align_y(iced::Alignment::Center),
            text("Entire position · Market · 10% slippage")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(10);

        if let Some(data) = self.account_data_for_order_account(&form.account_address) {
            for order in data
                .open_orders
                .iter()
                .filter(|order| position_trigger_order(order, &form.coin))
            {
                let label = existing_tpsl_kind(order)
                    .map(TpslKind::label)
                    .unwrap_or("Trigger");
                let price = order.trigger_px.as_deref().unwrap_or("—");
                content = content.push(
                    row![
                        text(format!("{label} · {price}")).size(12),
                        Space::new().width(Fill),
                        button(text("Cancel").size(11))
                            .style(button::text)
                            .on_press_maybe((!busy).then(|| Message::CancelOrder {
                                coin: form.coin.clone(),
                                oid: order.oid
                            })),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                );
            }
        }
        for (kind, input) in [
            (TpslKind::TakeProfit, &form.take_profit),
            (TpslKind::StopLoss, &form.stop_loss),
        ] {
            content = content.push(
                column![
                    text(kind.label()).size(12),
                    text_input("Trigger price (optional)", input)
                        .id(match kind {
                            TpslKind::TakeProfit => "position_tpsl_tp",
                            TpslKind::StopLoss => "position_tpsl_sl",
                        })
                        .on_input_maybe((!busy).then_some(move |value: String| {
                            Message::PositionTpslInputChanged(kind, value.into())
                        }))
                        .size(13)
                        .padding(8),
                ]
                .spacing(4),
            );
        }
        content = content.push(
            text("Triggers on mark price")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        );
        if let Some((status, error)) = &form.status {
            content = content.push(text(status).size(11).color(if *error {
                theme.palette().danger
            } else {
                theme.palette().primary
            }));
        }
        let action = if let Some(pending) = &self.position_tpsl.pending {
            button(
                text(if pending.in_flight {
                    "Working…"
                } else {
                    "Check status"
                })
                .size(12),
            )
            .on_press_maybe((!pending.in_flight).then_some(Message::CheckPositionTpsl))
        } else {
            button(text("Set TP/SL").size(12)).on_press_maybe(
                (!busy
                    && (!form.take_profit.trim().is_empty() || !form.stop_loss.trim().is_empty()))
                .then_some(Message::SubmitPositionTpsl),
            )
        };
        content = content.push(
            row![
                Space::new().width(Fill),
                button(text("Close").size(12))
                    .style(button::secondary)
                    .on_press(Message::ClosePositionTpsl),
                action,
            ]
            .spacing(8),
        );
        let card = container(scrollable(content))
            .padding(16)
            .width(Fill)
            .max_width(380)
            .max_height(460)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                border: iced::Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: theme.extended_palette().background.strong.color,
                },
                ..Default::default()
            });
        let backdrop = mouse_area(container(Space::new()).width(Fill).height(Fill).style(
            |theme: &Theme| {
                container::Style {
                    background: Some(
                        Color {
                            a: 0.72,
                            ..theme.extended_palette().background.strong.color
                        }
                        .into(),
                    ),
                    ..Default::default()
                }
            },
        ))
        .on_press(Message::ClosePositionTpsl);
        Some(
            stack![
                opaque(backdrop),
                container(opaque(card)).padding(12).center(Fill),
            ]
            .width(Fill)
            .height(Fill)
            .into(),
        )
    }
}
