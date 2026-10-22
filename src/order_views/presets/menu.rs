use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::OrderKind;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, button, container, row, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::order_views) fn push_order_presets_menu<'a>(
        &'a self,
        mut form: Column<'a, Message>,
        active_is_outcome: bool,
    ) -> Column<'a, Message> {
        let pane_corner_radius = self.pane_corner_radius;
        let presets_toggle = button(
            row![
                text("Quick trade presets").size(12),
                crate::helpers::icons::icon(
                    if self.presets_menu_expanded {
                        crate::helpers::icons::CHEVRON_UP
                    } else {
                        crate::helpers::icons::CHEVRON_DOWN
                    },
                    10.0,
                    crate::helpers::icons::IconColor::Muted,
                ),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::TogglePresetsMenu)
        .height(24)
        .padding([0, 8])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            crate::helpers::design::compact_button_style(theme, status, false, pane_corner_radius)
        });

        form = form.push(presets_toggle);

        if self.presets_menu_expanded {
            form = form.push(self.view_order_presets_menu(active_is_outcome));
        }

        form
    }

    fn view_order_presets_menu(&self, active_is_outcome: bool) -> Element<'_, Message> {
        let theme = self.theme();
        let mut presets_col = Column::new().spacing(12);
        let denomination_label = match (active_is_outcome, self.preset_is_usd) {
            (true, true) => format!(
                "$ {}",
                self.outcome_quote_symbol_for_coin(&self.active_symbol)
            ),
            (true, false) => "Contracts".to_string(),
            (false, true) => "$ USD".to_string(),
            (false, false) => "Coin".to_string(),
        };

        let currency_toggle = row![
            text("Size Denomination:")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
            Space::new().width(Fill),
            button(
                text(denomination_label)
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
                .color(theme.extended_palette().background.weak.text)
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
                ));
            if !active_is_outcome {
                presets_col = presets_col.push(self.view_order_preset_row(
                    "Chase",
                    &self.order_presets.chase_usd,
                    OrderKind::Chase,
                ));
            }
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
                ));
            if !active_is_outcome {
                presets_col = presets_col.push(self.view_order_preset_row(
                    "Chase",
                    &self.order_presets.chase_coin,
                    OrderKind::Chase,
                ));
            }
        }

        container(presets_col)
            .padding(8)
            .width(Fill)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }
}
