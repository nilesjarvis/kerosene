use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId};
use crate::config::{MAX_QUICK_TRADE_ACTIONS, QuickTradeSide};
use crate::message::Message;
use crate::order_execution::QuickTradeOrderRequest;

use iced::widget::{
    Space, button, column, container, row, rule, scrollable, text, text::Wrapping, text_input,
    tooltip,
};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Chart Quick Trade Panel
// ---------------------------------------------------------------------------

const QUICK_TRADE_PANEL_HEIGHT: f32 = 56.0;

impl TradingTerminal {
    pub(super) fn view_quick_trade_panel(
        &self,
        chart_id: ChartId,
        instance: &ChartInstance,
        surface_id: ChartSurfaceId,
    ) -> Element<'_, Message> {
        let pending = self.has_pending_trading_request();
        let status = if pending { "PENDING" } else { "MARKET" };
        let status_color = if pending {
            self.theme().palette().warning
        } else {
            self.theme().extended_palette().background.weak.text
        };

        let header = column![
            text("QUICK TRADE")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            text(status)
                .size(9)
                .font(crate::app_fonts::monospace_font())
                .color(status_color),
        ]
        .spacing(1)
        .width(Length::Fixed(76.0));

        let mut actions = row![].spacing(6).align_y(Alignment::Center);
        if instance.quick_trade_actions.is_empty() {
            actions = actions.push(
                text("No actions configured")
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(self.theme().extended_palette().background.weak.text),
            );
        } else {
            for (index, action) in instance.quick_trade_actions.iter().enumerate() {
                let request = QuickTradeOrderRequest {
                    chart_id,
                    surface_id,
                    symbol_key: instance.symbol.clone(),
                    action_index: index,
                    action: action.clone(),
                };
                let side = action.side;
                let action_button = button(
                    text(action.button_label())
                        .size(11)
                        .font(crate::app_fonts::monospace_font()),
                )
                .on_press_maybe((!pending).then_some(Message::SubmitQuickTradeOrder(request)))
                .padding([5, 10])
                .style(move |theme: &Theme, state| {
                    quick_trade_action_button_style(theme, state, side, pending)
                });
                actions = actions.push(tooltip(
                    action_button,
                    text(match action.denomination {
                        crate::config::QuickTradeDenomination::Usd => {
                            "Submit this USD-notional market order immediately"
                        }
                        crate::config::QuickTradeDenomination::Coin => {
                            "Submit this coin-denominated market order immediately"
                        }
                    })
                    .size(10)
                    .font(crate::app_fonts::monospace_font()),
                    tooltip::Position::Top,
                ));
            }
        }

        let action_strip = scrollable(actions).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new()
                .width(3)
                .margin(0)
                .scroller_width(3),
        ));
        let configure = tooltip(
            button(
                text("EDIT")
                    .size(10)
                    .font(crate::app_fonts::monospace_font()),
            )
            .on_press(Message::OpenQuickTradeEditor(chart_id))
            .padding([5, 8])
            .style(button::secondary),
            text("Configure Quick Trade actions")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        );

        container(
            row![header, action_strip, configure]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Fill),
        )
        .width(Fill)
        .height(QUICK_TRADE_PANEL_HEIGHT)
        .padding([7, 8])
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            border: iced::Border {
                color: Color {
                    a: 0.18,
                    ..theme.extended_palette().background.weak.text
                },
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    // -----------------------------------------------------------------------
    // Dedicated Quick Trade Editor Window
    // -----------------------------------------------------------------------

    pub(crate) fn view_quick_trade_editor_window(&self) -> Element<'_, Message> {
        let Some(editor) = &self.quick_trade_editor else {
            return Space::new().into();
        };
        let theme = self.theme();
        let chart_label = self
            .charts
            .get(&editor.chart_id)
            .map(|instance| instance.symbol_display.as_str())
            .unwrap_or("Removed chart");

        let mut action_rows = column![].spacing(8).width(Fill);
        if editor.actions.is_empty() {
            action_rows = action_rows.push(
                container(
                    column![
                        text("No Quick Trade actions yet").size(13),
                        text("Add an action to populate the chart panel.")
                            .size(11)
                            .color(theme.extended_palette().background.weak.text),
                    ]
                    .spacing(4)
                    .align_x(Alignment::Center),
                )
                .width(Fill)
                .padding(28)
                .center_x(Fill)
                .style(editor_card_style),
            );
        } else {
            for (index, action) in editor.actions.iter().enumerate() {
                let side = action.side;
                let side_button = button(
                    text(side.label())
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .center()
                        .width(Fill),
                )
                .on_press(Message::QuickTradeActionSideToggled(index))
                .width(Length::Fixed(68.0))
                .padding([7, 8])
                .style(move |theme: &Theme, state| editor_side_button_style(theme, state, side));
                let quantity = text_input("Amount", &action.quantity)
                    .on_input(move |value| {
                        Message::QuickTradeActionQuantityChanged(index, value.into())
                    })
                    .size(12)
                    .padding([7, 9])
                    .width(Fill)
                    .style(crate::helpers::text_input_style);
                let denomination = button(
                    text(action.denomination.label())
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .center()
                        .width(Fill),
                )
                .on_press(Message::QuickTradeActionDenominationToggled(index))
                .width(Length::Fixed(72.0))
                .padding([7, 8])
                .style(button::secondary);
                let remove = button(text("Remove").size(10))
                    .on_press(Message::QuickTradeActionRemoved(index))
                    .padding([7, 9])
                    .style(destructive_secondary_button_style);

                action_rows = action_rows.push(
                    container(
                        row![
                            text(format!("{:02}", index + 1))
                                .size(10)
                                .font(crate::app_fonts::monospace_font())
                                .color(theme.extended_palette().background.weak.text)
                                .width(Length::Fixed(24.0)),
                            side_button,
                            quantity,
                            denomination,
                            remove,
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .padding(8)
                    .width(Fill)
                    .style(editor_card_style),
                );
            }
        }

        let can_add = editor.actions.len() < MAX_QUICK_TRADE_ACTIONS;
        let add_button = button(text("+ Add Action").size(11))
            .on_press_maybe(can_add.then_some(Message::QuickTradeActionAdded))
            .padding([7, 12])
            .style(button::secondary);
        let cancel_button = button(text("Cancel").size(11))
            .on_press(Message::CloseQuickTradeEditor)
            .padding([7, 14])
            .style(button::secondary);
        let save_button = button(text("Save Actions").size(11))
            .on_press(Message::SaveQuickTradeActions)
            .padding([7, 14])
            .style(primary_button_style);

        let mut footer = column![
            row![
                add_button,
                text(format!(
                    "{} / {MAX_QUICK_TRADE_ACTIONS}",
                    editor.actions.len()
                ))
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text),
                Space::new().width(Fill),
                cancel_button,
                save_button,
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(8)
        .width(Fill);
        if let Some(error) = &editor.error {
            footer = footer.push(text(error.clone()).size(11).color(theme.palette().danger));
        }

        let content = column![
            column![
                text("Quick Trade Actions").size(16),
                text(format!("Chart: {chart_label}"))
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
                text("Each button submits a market order immediately. Quantities are validated again at execution time.")
                    .size(11)
                    .width(Fill)
                    .wrapping(Wrapping::Word)
                    .color(theme.palette().warning),
            ]
            .spacing(4),
            rule::horizontal(1),
            scrollable(action_rows).direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            )),
            rule::horizontal(1),
            footer,
        ]
        .spacing(10)
        .width(Fill)
        .height(Fill);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(16)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                text_color: Some(theme.palette().text),
                ..Default::default()
            })
            .into()
    }
}

fn quick_trade_action_button_style(
    theme: &Theme,
    status: button::Status,
    side: QuickTradeSide,
    disabled: bool,
) -> button::Style {
    let accent = match side {
        QuickTradeSide::Buy => theme.palette().success,
        QuickTradeSide::Sell => theme.palette().danger,
    };
    let alpha = if disabled {
        0.06
    } else if matches!(status, button::Status::Hovered) {
        0.24
    } else {
        0.13
    };
    button::Style {
        background: Some(Color { a: alpha, ..accent }.into()),
        text_color: if disabled {
            theme.extended_palette().background.weak.text
        } else {
            accent
        },
        border: iced::Border {
            color: Color {
                a: if disabled { 0.12 } else { 0.45 },
                ..accent
            },
            width: 1.0,
            radius: 3.0.into(),
        },
        ..Default::default()
    }
}

fn editor_side_button_style(
    theme: &Theme,
    status: button::Status,
    side: QuickTradeSide,
) -> button::Style {
    quick_trade_action_button_style(theme, status, side, false)
}

fn editor_card_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            color: Color {
                a: 0.24,
                ..theme.extended_palette().background.weak.text
            },
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn destructive_secondary_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let danger = theme.palette().danger;
    button::Style {
        background: Some(
            Color {
                a: if matches!(status, button::Status::Hovered) {
                    0.18
                } else {
                    0.08
                },
                ..danger
            }
            .into(),
        ),
        text_color: danger,
        border: iced::Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn primary_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let primary = theme.palette().primary;
    button::Style {
        background: Some(
            Color {
                a: if matches!(status, button::Status::Hovered) {
                    0.92
                } else {
                    0.78
                },
                ..primary
            }
            .into(),
        ),
        text_color: crate::helpers::text_color_for_bg(primary),
        border: iced::Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
