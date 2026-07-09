use crate::app_state::TradingTerminal;
use crate::helpers::{
    self, format_usd, invalid_data_placeholder, optional_value_color, order_type_button,
    parse_finite_number,
};
use crate::message::Message;
use crate::order_execution::OrderLeverageSubmissionSnapshot;
use crate::signing::OrderKind;
use iced::widget::{Space, button, container, row, text, text_input};
use iced::{Color, Element, Fill, Length, Theme};

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(super) fn view_order_entry_symbol_row(
        &self,
        theme: &Theme,
    ) -> (Element<'_, Message>, Option<f64>) {
        let mut symbol_badge = row![];
        if let Some(icon) =
            helpers::symbol_icon(&self.active_symbol_display, 20, theme.palette().text)
        {
            symbol_badge = symbol_badge.push(icon).push(Space::new().width(6.0));
        }
        symbol_badge = symbol_badge
            .push(
                text(self.active_symbol_display.to_uppercase().to_string())
                    .size(16)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..iced::Font::DEFAULT
                    })
                    .color(theme.palette().text),
            )
            .align_y(iced::Alignment::Center);

        let symbol_label = container(symbol_badge)
            .padding([4, 12])
            .style(move |theme: &Theme| container::Style {
                background: Some(
                    Color {
                        a: 0.2,
                        ..theme.palette().primary
                    }
                    .into(),
                ),
                border: iced::Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: theme.palette().primary,
                },
                ..Default::default()
            });

        let mut symbol_row = row![symbol_label]
            .spacing(8)
            .align_y(iced::Alignment::Center);
        let mut margin_used = Some(0.0);

        if let Some((_, data)) = self.connected_order_account_snapshot() {
            let original_coin = self.active_symbol.as_str();
            if let Some(pos) = data
                .clearinghouse
                .asset_positions
                .iter()
                .find(|p| p.position.coin == original_coin)
            {
                margin_used = parse_order_header_number(&pos.position.margin_used);
            }

            if let Some((is_cross, lev, is_actual)) =
                data.get_leverage_for(&self.active_symbol, &self.exchange_symbols)
            {
                let leverage_label = order_leverage_label(is_cross, lev, is_actual);
                symbol_row = symbol_row.push(order_leverage_badge(
                    leverage_label,
                    self.order_leverage_dropdown_open,
                ));
            }
        }

        (symbol_row.into(), margin_used)
    }

    pub(super) fn view_order_entry_context_row(
        &self,
        margin_used: Option<f64>,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let available_margin = self
            .connected_order_account_snapshot()
            .map(|(_, data)| data)
            .map(|data| {
                if self.is_spot_coin(&self.active_symbol) {
                    self.spot_spendable_quote_balance(&self.active_symbol, data)
                } else if self.is_outcome_coin(&self.active_symbol) {
                    data.available_margin_for_token(
                        self.outcome_quote_token_index_for_coin(&self.active_symbol),
                    )
                } else {
                    self.visible_available_margin_usdc(data)
                }
            })
            .unwrap_or(Some(0.0));
        let spot_quote_label = self
            .exchange_symbol_for_key(&self.active_symbol)
            .filter(|symbol| symbol.market_type == crate::api::MarketType::Spot)
            .and_then(|symbol| symbol.display_name.as_deref())
            .and_then(|display| display.rsplit_once('/'))
            .map(|(_, quote)| quote.to_string());
        let available_label = match spot_quote_label.as_deref() {
            Some(_) if !self.spot_usd_denomination_supported(&self.active_symbol) => {
                format_optional_token_amount(available_margin, spot_quote_label.as_deref())
            }
            _ => format_optional_usd(available_margin),
        };
        let weak_color = theme.extended_palette().background.weak.text;
        let invalid_color = theme.palette().warning;
        let available_color = optional_value_color(available_margin, weak_color, invalid_color);
        let margin_used_color = optional_value_color(margin_used, weak_color, invalid_color);

        row![
            text(format!("Avail: {available_label}"))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(available_color),
            Space::new().width(Fill),
            text(format!("Used: {}", format_optional_usd(margin_used)))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(margin_used_color),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }

    pub(super) fn view_order_entry_leverage_dropdown(
        &self,
        can_update: bool,
    ) -> Option<Element<'_, Message>> {
        let (max_leverage, cross_allowed) = self.active_order_leverage_constraints()?;
        let theme = self.theme();
        let cross_selected = self.order_leverage_is_cross && cross_allowed;
        let isolated_selected = !cross_selected;
        let pending = self.pending_leverage_update.is_some();
        let can_apply = can_update && !pending && !self.order_leverage_input.is_empty();

        let mut input = text_input("", &self.order_leverage_input)
            .style(helpers::text_input_style)
            .size(12)
            .padding(5)
            .width(Length::Fixed(46.0));
        if !pending {
            input = input.on_input(Message::OrderLeverageInputChanged);
        }

        let apply_button: Element<'_, Message> = if pending {
            container(self.view_spinner(12))
                .padding([4, 8])
                .width(Length::Fixed(58.0))
                .center_x(Fill)
                .style(leverage_apply_container_style)
                .into()
        } else {
            leverage_apply_button(can_apply, self.order_leverage_submission_snapshot())
        };

        let controls = row![
            text("Leverage")
                .size(12)
                .color(theme.extended_palette().background.weak.text),
            leverage_mode_button(
                "Cross",
                cross_selected,
                cross_allowed,
                Message::SetOrderLeverageCross(true),
            ),
            leverage_mode_button(
                "Isolated",
                isolated_selected,
                true,
                Message::SetOrderLeverageCross(false),
            ),
            Space::new().width(Fill),
            input,
            text("x")
                .size(12)
                .color(theme.extended_palette().background.weak.text),
            text(format!("Max {max_leverage}x"))
                .size(11)
                .color(theme.extended_palette().background.weak.text),
            apply_button,
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        Some(
            container(controls)
                .width(Fill)
                .padding(8)
                .style(leverage_dropdown_container_style)
                .into(),
        )
    }

    pub(super) fn view_order_entry_type_row(&self) -> Element<'static, Message> {
        let limit_selected = matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc);
        row![
            order_type_button(
                "Market",
                self.order_kind == OrderKind::Market,
                Message::SetOrderKind(OrderKind::Market),
            ),
            order_type_button(
                "Limit",
                limit_selected,
                Message::SetOrderKind(OrderKind::Limit),
            ),
            order_type_button(
                "Chase",
                self.order_kind == OrderKind::Chase,
                Message::SetOrderKind(OrderKind::Chase),
            ),
            order_type_button(
                "TWAP",
                self.order_kind == OrderKind::Twap,
                Message::SetOrderKind(OrderKind::Twap),
            ),
        ]
        .spacing(4)
        .into()
    }
}

fn parse_order_header_number(raw: &str) -> Option<f64> {
    parse_finite_number(raw)
}

fn format_optional_usd(value: Option<f64>) -> String {
    value
        .map(|value| format_usd(&format!("{value:.2}")))
        .unwrap_or_else(invalid_data_placeholder)
}

fn format_optional_token_amount(value: Option<f64>, token: Option<&str>) -> String {
    match (value, token.filter(|token| !token.trim().is_empty())) {
        (Some(value), Some(token)) if value.is_finite() => {
            format!(
                "{} {token}",
                helpers::trim_decimal_zeros(format!("{value:.8}"))
            )
        }
        _ => invalid_data_placeholder(),
    }
}

fn order_leverage_label(is_cross: bool, leverage: u32, is_actual: bool) -> String {
    if is_actual {
        if is_cross {
            format!("Cross {leverage}x")
        } else {
            format!("Isolated {leverage}x")
        }
    } else {
        // This is only for display purposes: the asset's max limit, not the account setting.
        format!("Max {leverage}x")
    }
}

fn order_leverage_badge(label: String, open: bool) -> Element<'static, Message> {
    button(text(label).size(10).color(Color::WHITE))
        .padding([2, 6])
        .on_press(Message::ToggleOrderLeverageDropdown)
        .style(move |theme: &Theme, status| {
            let background = if matches!(status, button::Status::Hovered) {
                theme.palette().primary
            } else {
                theme.extended_palette().background.strong.color
            };
            button::Style {
                background: Some(background.into()),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: 4.0.into(),
                    width: if open { 1.0 } else { 0.0 },
                    color: if open {
                        theme.palette().primary
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
}

fn leverage_mode_button(
    label: &'static str,
    active: bool,
    enabled: bool,
    msg: Message,
) -> Element<'static, Message> {
    let mut btn = button(text(label).size(11).center()).padding([4, 8]).style(
        move |theme: &Theme, status| {
            let palette = theme.palette();
            let extended = theme.extended_palette();
            let bg = if active {
                Color {
                    a: 0.15,
                    ..palette.primary
                }
            } else if matches!(status, button::Status::Hovered) && enabled {
                extended.background.strong.color
            } else {
                extended.background.weak.color
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if active {
                    palette.primary
                } else if enabled {
                    extended.background.weak.text
                } else {
                    Color {
                        a: 0.45,
                        ..extended.background.weak.text
                    }
                },
                border: iced::Border {
                    radius: 4.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active {
                        Color {
                            a: 0.3,
                            ..palette.primary
                        }
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        },
    );

    if enabled {
        btn = btn.on_press(msg);
    }

    btn.into()
}

fn leverage_apply_button(
    enabled: bool,
    snapshot: OrderLeverageSubmissionSnapshot,
) -> Element<'static, Message> {
    let mut btn = button(text("Apply").size(11).center())
        .padding([4, 8])
        .width(Length::Fixed(58.0))
        .style(|theme: &Theme, status| {
            let palette = theme.palette();
            let bg = match status {
                button::Status::Hovered => Color {
                    a: 0.85,
                    ..palette.primary
                },
                button::Status::Disabled => theme.extended_palette().background.weak.color,
                _ => palette.primary,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if matches!(status, button::Status::Disabled) {
                    Color {
                        a: 0.45,
                        ..theme.extended_palette().background.weak.text
                    }
                } else {
                    crate::helpers::text_color_for_bg(palette.primary)
                },
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    if enabled {
        btn = btn.on_press(Message::SubmitOrderLeverage(snapshot));
    }

    btn.into()
}

fn leverage_apply_container_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.palette().primary.into()),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn leverage_dropdown_container_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color {
                a: 0.25,
                ..theme.extended_palette().background.strong.text
            },
        },
        ..Default::default()
    }
}
