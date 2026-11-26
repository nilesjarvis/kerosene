use crate::app_state::TradingTerminal;
use crate::config::OrderPreset;
use crate::helpers;
use crate::message::Message;
use crate::signing::OrderKind;
use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::{Color, Element, Theme};

impl TradingTerminal {
    pub(super) fn view_order_preset_row<'a>(
        &'a self,
        title: &'static str,
        presets: &'a [OrderPreset],
        kind: OrderKind,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let mut row_items = row![].spacing(6);

        let standard = matches!(
            kind,
            OrderKind::Market | OrderKind::Limit | OrderKind::LimitIoc
        );
        let enabled = !standard
            || (self.order_entry_unavailable_hint().is_none()
                && self.standard_ticket_wait().is_none());
        for (idx, p) in presets.iter().enumerate() {
            if self.preset_edit_mode {
                if self.preset_edit_idx == Some((kind, idx)) {
                    let input = text_input("Size", &self.preset_edit_buffer)
                        .style(helpers::text_input_style)
                        .on_input(Message::EditPresetChanged)
                        .on_submit(Message::EditPresetSave(kind, idx))
                        .size(11)
                        .padding([2, 4]);

                    let save_btn = button(text("OK").size(10).color(theme.palette().success))
                        .on_press(Message::EditPresetSave(kind, idx))
                        .padding([2, 6])
                        .style(button::secondary);

                    row_items = row_items.push(row![input, save_btn].spacing(4));
                } else {
                    let edit_btn = button(
                        text(p.label.clone())
                            .size(10)
                            .color(theme.extended_palette().background.weak.text),
                    )
                    .on_press(Message::EditPresetStart(kind, idx, p.size.to_string()))
                    .padding([3, 6])
                    .style(|theme: &Theme, status| {
                        let bg = match status {
                            button::Status::Hovered => {
                                theme.extended_palette().background.strong.color
                            }
                            _ => theme.extended_palette().background.weak.color,
                        };
                        button::Style {
                            background: Some(bg.into()),
                            text_color: theme.palette().text,
                            border: iced::Border {
                                radius: 3.0.into(),
                                width: 1.0,
                                color: theme.extended_palette().background.strong.color,
                            },
                            ..Default::default()
                        }
                    });
                    row_items = row_items.push(edit_btn);
                }
            } else {
                let action = |is_buy, side| -> Element<'_, Message> {
                    let control = button(text(format!("{side} {}", p.label)).size(10))
                        .on_press_maybe(enabled.then(|| Message::ExecutePreset {
                            kind,
                            preset: p.clone(),
                            is_buy,
                            account_generation: self.account_data_request_generation,
                        }))
                        .padding([3, 6])
                        .style(move |theme: &Theme, status| {
                            let accent = if is_buy {
                                theme.palette().success
                            } else {
                                theme.palette().danger
                            };
                            let mut style = button::secondary(theme, status);
                            if status != button::Status::Disabled {
                                style.background = Some(
                                    if status == button::Status::Hovered {
                                        Color { a: 0.15, ..accent }
                                    } else {
                                        theme.extended_palette().background.weak.color
                                    }
                                    .into(),
                                );
                                style.text_color = accent;
                            }
                            style
                        });
                    if enabled {
                        control.into()
                    } else {
                        column![control].into()
                    }
                };
                row_items =
                    row_items.push(row![action(true, "Buy"), action(false, "Sell")].spacing(1));
            }
        }

        column![
            text(title)
                .size(11)
                .color(theme.extended_palette().background.weak.text),
            scrollable(row_items).direction(iced::widget::scrollable::Direction::Horizontal(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .margin(0)
                    .scroller_width(4)
            ))
        ]
        .spacing(4)
        .into()
    }
}
