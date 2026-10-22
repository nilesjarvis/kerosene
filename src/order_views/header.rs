use crate::app_state::TradingTerminal;
use crate::config::effective_radius;
use crate::helpers::design::{
    UiColors, compact_button_style, compact_input_style, filled_action_pair, mix_color,
};
use crate::helpers::{
    self, format_usd, invalid_data_placeholder, optional_value_color, order_type_button,
    parse_finite_number,
};
use crate::message::Message;
use crate::order_execution::OrderLeverageSubmissionSnapshot;
use crate::signing::OrderKind;
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Element, Fill, Length, Theme};

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(super) fn view_order_entry_symbol_row(
        &self,
        theme: &Theme,
    ) -> (Element<'_, Message>, Option<f64>) {
        let mut symbol_badge = row![];
        if let Some(icon) =
            helpers::symbol_icon(&self.active_symbol_display, 16, theme.palette().text)
        {
            symbol_badge = symbol_badge.push(icon).push(Space::new().width(6.0));
        }
        symbol_badge = symbol_badge
            .push(
                text(self.active_symbol_display.to_uppercase().to_string())
                    .size(13)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..iced::Font::DEFAULT
                    })
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).text),
                    }),
            )
            .align_y(iced::Alignment::Center);

        let mut symbol_row = row![symbol_badge, Space::new().width(Fill)]
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
                    self.pane_corner_radius,
                ));
            }
        }

        (symbol_row.into(), margin_used)
    }

    pub(super) fn view_order_entry_context_row(
        &self,
        margin_used: Option<f64>,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let available_margin = self
            .connected_order_account_snapshot()
            .map(|(_, data)| data)
            .map(|data| {
                if self.is_spot_coin(&self.active_symbol) {
                    self.spot_spendable_quote_balance(&self.active_symbol, data)
                } else if self.is_outcome_coin(&self.active_symbol) {
                    self.outcome_quote_token_index_for_coin(&self.active_symbol)
                        .and_then(|index| data.available_margin_for_token(index))
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
        // Missing account data is a freshness condition, so it keeps the
        // warning treatment; present values resolve through shared roles.
        let colors = UiColors::new(theme);
        let invalid_color = colors.warning;
        let available_color = optional_value_color(available_margin, colors.muted, invalid_color);
        let margin_used_color = optional_value_color(margin_used, colors.muted, invalid_color);

        row![
            text(format!("Available: {available_label}"))
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
        let pane_corner_radius = self.pane_corner_radius;
        let cross_selected = self.order_leverage_is_cross && cross_allowed;
        let isolated_selected = !cross_selected;
        let pending = self.pending_leverage_update.is_some();
        let can_apply = can_update && !pending && !self.order_leverage_input.is_empty();

        let mut input = text_input("", &self.order_leverage_input)
            .size(12)
            .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(16.0)))
            .padding([4, 8])
            .width(Length::Fixed(46.0))
            .style(move |theme, status| {
                compact_input_style(theme, status, false, pane_corner_radius)
            });
        if !pending {
            input = input.on_input(Message::OrderLeverageInputChanged);
        }

        let apply_button: Element<'_, Message> = if pending {
            // Busy keeps the Apply frame: spinner inside the same primary
            // surface instead of a differently sized container.
            container(self.view_spinner(12))
                .height(24)
                .width(Length::Fixed(58.0))
                .center_y(24)
                .center_x(Fill)
                .style(move |theme: &Theme| {
                    let (fill, foreground) = filled_action_pair(theme.palette().primary);
                    container::Style {
                        background: Some(fill.into()),
                        text_color: Some(foreground),
                        border: iced::Border {
                            radius: effective_radius(pane_corner_radius, 4.0).into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .into()
        } else {
            leverage_apply_button(
                can_apply,
                self.order_leverage_submission_snapshot(),
                pane_corner_radius,
            )
        };

        let mode_controls = row![
            text("Leverage").size(12).style(move |theme: &Theme| {
                text::Style {
                    color: Some(UiColors::new(theme).muted),
                }
            }),
            leverage_mode_button(
                "Cross",
                cross_selected,
                cross_allowed,
                Message::SetOrderLeverageCross(true),
                pane_corner_radius,
            ),
            leverage_mode_button(
                "Isolated",
                isolated_selected,
                true,
                Message::SetOrderLeverageCross(false),
                pane_corner_radius,
            ),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);
        let leverage_controls = row![
            input,
            text("x").size(12).style(move |theme: &Theme| text::Style {
                color: Some(UiColors::new(theme).muted),
            }),
            text(format!("Max {max_leverage}x"))
                .size(11)
                .style(move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).muted),
                }),
            Space::new().width(Fill),
            apply_button,
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        Some(
            container(column![mode_controls, leverage_controls].spacing(6))
                .width(Fill)
                .padding(8)
                .style(move |theme: &Theme| container::Style {
                    background: Some(UiColors::new(theme).inset.into()),
                    border: iced::Border {
                        radius: effective_radius(pane_corner_radius, 6.0).into(),
                        width: 1.0,
                        color: UiColors::new(theme).divider,
                    },
                    ..Default::default()
                })
                .into(),
        )
    }

    pub(super) fn view_order_entry_type_row(&self) -> Element<'static, Message> {
        let pane_corner_radius = self.pane_corner_radius;
        let limit_selected = matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc);
        // Order type is one segmented choice on the shared inset track:
        // equal widths, fixed 12px labels, stable selection geometry.
        let segments = row![
            order_type_button(
                "Market",
                self.order_kind == OrderKind::Market,
                Message::SetOrderKind(OrderKind::Market),
                pane_corner_radius,
            ),
            order_type_button(
                "Limit",
                limit_selected,
                Message::SetOrderKind(OrderKind::Limit),
                pane_corner_radius,
            ),
            order_type_button(
                "Chase",
                self.order_kind == OrderKind::Chase,
                Message::SetOrderKind(OrderKind::Chase),
                pane_corner_radius,
            ),
            order_type_button(
                "TWAP",
                self.order_kind == OrderKind::Twap,
                Message::SetOrderKind(OrderKind::Twap),
                pane_corner_radius,
            ),
        ]
        .spacing(2);
        container(segments)
            .width(Fill)
            .padding(2)
            .style(move |theme: &Theme| container::Style {
                background: Some(UiColors::new(theme).inset.into()),
                border: iced::Border {
                    radius: effective_radius(pane_corner_radius, 4.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            })
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

fn order_leverage_badge(
    label: String,
    open: bool,
    pane_corner_radius: f32,
) -> Element<'static, Message> {
    button(text(label).size(11).center())
        .height(24)
        .padding([0, 8])
        .on_press(Message::ToggleOrderLeverageDropdown)
        .style(move |theme: &Theme, status| {
            compact_button_style(theme, status, open, pane_corner_radius)
        })
        .into()
}

fn leverage_mode_button(
    label: &'static str,
    active: bool,
    enabled: bool,
    msg: Message,
    pane_corner_radius: f32,
) -> Element<'static, Message> {
    let mut btn = button(text(label).size(12).center())
        .height(24)
        .padding([0, 8])
        .style(move |theme: &Theme, status| {
            compact_button_style(theme, status, active, pane_corner_radius)
        });

    if enabled {
        btn = btn.on_press(msg);
    }

    btn.into()
}

fn leverage_apply_button(
    enabled: bool,
    snapshot: OrderLeverageSubmissionSnapshot,
    pane_corner_radius: f32,
) -> Element<'static, Message> {
    // One leading non-trading action in this group: primary accent fill
    // with a paired foreground; unavailable keeps the shared disabled look.
    let mut btn = button(text("Apply").size(12).center())
        .height(24)
        .width(Length::Fixed(58.0))
        .style(move |theme: &Theme, status| {
            if !enabled && status != button::Status::Hovered {
                return compact_button_style(theme, status, false, pane_corner_radius);
            }
            let (fill, foreground) = filled_action_pair(theme.palette().primary);
            let amount = match status {
                button::Status::Hovered => 0.06,
                button::Status::Pressed => 0.12,
                _ => 0.0,
            };
            button::Style {
                background: Some(mix_color(fill, foreground, amount).into()),
                text_color: foreground,
                border: iced::Border {
                    radius: effective_radius(pane_corner_radius, 4.0).into(),
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
