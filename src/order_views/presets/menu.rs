use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::OrderKind;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, button, container, row, text};
use iced::{Element, Fill, Theme, color};

impl TradingTerminal {
    pub(in crate::order_views) fn push_order_presets_menu<'a>(
        &'a self,
        mut form: Column<'a, Message>,
        active_is_outcome: bool,
    ) -> Column<'a, Message> {
        if active_is_outcome {
            return form;
        }

        let presets_toggle = button(
            text(if self.presets_menu_expanded {
                "Presets  \u{2212}"
            } else {
                "Presets  +"
            })
            .size(11)
            .center(),
        )
        .on_press(Message::TogglePresetsMenu)
        .padding([4, 8])
        .width(Fill)
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => color!(0x222222),
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

        form = form.push(Space::new().height(8.0)).push(presets_toggle);

        if self.presets_menu_expanded {
            form = form.push(self.view_order_presets_menu());
        }

        form
    }

    fn view_order_presets_menu(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let mut presets_col = Column::new().spacing(12);

        let currency_toggle = row![
            text("Size Denomination:").size(11).color(color!(0x888888)),
            Space::new().width(Fill),
            button(
                text(if self.preset_is_usd { "$ USD" } else { "Coin" })
                    .size(10)
                    .color(theme.palette().primary)
            )
            .padding([2, 6])
            .on_press(Message::TogglePresetCurrency)
            .style(button::secondary),
            Space::new().width(8.0),
            button(
                text(if self.preset_edit_mode {
                    "Done"
                } else {
                    "Edit"
                })
                .size(10)
                .color(color!(0xbbbbbb))
            )
            .padding([2, 6])
            .on_press(Message::TogglePresetEditMode)
            .style(button::text),
        ]
        .align_y(iced::Alignment::Center);
        presets_col = presets_col.push(currency_toggle);

        if self.preset_is_usd {
            presets_col = presets_col
                .push(self.view_order_preset_row(
                    "Market",
                    &self.order_presets.market_usd,
                    OrderKind::Market,
                ))
                .push(self.view_order_preset_row(
                    "Limit",
                    &self.order_presets.limit_usd,
                    OrderKind::Limit,
                ))
                .push(self.view_order_preset_row(
                    "Chase",
                    &self.order_presets.chase_usd,
                    OrderKind::Chase,
                ));
        } else {
            presets_col = presets_col
                .push(self.view_order_preset_row(
                    "Market",
                    &self.order_presets.market_coin,
                    OrderKind::Market,
                ))
                .push(self.view_order_preset_row(
                    "Limit",
                    &self.order_presets.limit_coin,
                    OrderKind::Limit,
                ))
                .push(self.view_order_preset_row(
                    "Chase",
                    &self.order_presets.chase_coin,
                    OrderKind::Chase,
                ));
        }

        container(presets_col)
            .padding(8)
            .width(Fill)
            .style(|_theme: &Theme| container_style::Style {
                background: Some(color!(0x151515).into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }
}
