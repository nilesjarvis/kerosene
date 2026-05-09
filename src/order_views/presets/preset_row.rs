use crate::app_state::TradingTerminal;
use crate::config::OrderPreset;
use crate::helpers;
use crate::message::Message;
use crate::signing::OrderKind;
use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::{Element, Theme, color};

impl TradingTerminal {
    pub(super) fn view_order_preset_row<'a>(
        &'a self,
        title: &'static str,
        presets: &'a [OrderPreset],
        kind: OrderKind,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let mut row_items = row![].spacing(6);

        for (idx, p) in presets.iter().enumerate() {
            let preset = p.clone();

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
                    let edit_btn = button(text(p.label.clone()).size(10).color(color!(0xbbbbbb)))
                        .on_press(Message::EditPresetStart(kind, idx, p.size.to_string()))
                        .padding([3, 6])
                        .style(|theme: &Theme, status| {
                            let bg = match status {
                                button::Status::Hovered => {
                                    theme.extended_palette().background.strong.color
                                }
                                _ => color!(0x222222),
                            };
                            button::Style {
                                background: Some(bg.into()),
                                text_color: theme.palette().text,
                                border: iced::Border {
                                    radius: 3.0.into(),
                                    width: 1.0,
                                    color: color!(0x444444),
                                },
                                ..Default::default()
                            }
                        });
                    row_items = row_items.push(edit_btn);
                }
            } else {
                let buy_btn = button(
                    text(format!("Buy {}", p.label))
                        .size(10)
                        .color(color!(0x50fa7b)),
                )
                .on_press(Message::ExecutePreset(kind, preset.clone(), true))
                .padding([3, 6])
                .style(|theme: &Theme, status| {
                    let bg = match status {
                        button::Status::Hovered => color!(0x1a3a25),
                        _ => theme.extended_palette().background.weak.color,
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
                    text(format!("Sell {}", p.label))
                        .size(10)
                        .color(color!(0xff5555)),
                )
                .on_press(Message::ExecutePreset(kind, preset.clone(), false))
                .padding([3, 6])
                .style(|theme: &Theme, status| {
                    let bg = match status {
                        button::Status::Hovered => color!(0x3a1a1a),
                        _ => theme.extended_palette().background.weak.color,
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

                row_items = row_items.push(row![buy_btn, sell_btn].spacing(1));
            }
        }

        column![
            text(title).size(11).color(color!(0x888888)),
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
