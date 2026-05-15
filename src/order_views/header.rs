use crate::app_state::TradingTerminal;
use crate::helpers::{self, format_usd, order_type_button};
use crate::message::Message;
use crate::signing::OrderKind;
use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Theme};

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

        if let Some(data) = &self.account_data {
            let mut search_coin = self.active_symbol.as_str();
            if let Some((_, suffix)) = search_coin.split_once(':') {
                search_coin = suffix;
            }
            if let Some(pos) = data
                .clearinghouse
                .asset_positions
                .iter()
                .find(|p| p.position.coin == search_coin)
            {
                margin_used = parse_order_header_number(&pos.position.margin_used);
            }

            if let Some((is_cross, lev, is_actual)) =
                data.get_leverage_for(&self.active_symbol, &self.exchange_symbols)
            {
                symbol_row = symbol_row.push(order_leverage_badge(order_leverage_label(
                    is_cross, lev, is_actual,
                )));
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
            .account_data
            .as_ref()
            .map(|data| self.visible_available_margin_usdc(data))
            .unwrap_or(Some(0.0));
        let weak_color = theme.extended_palette().background.weak.text;
        let available_color = if available_margin.is_some() {
            weak_color
        } else {
            theme.palette().warning
        };
        let margin_used_color = if margin_used.is_some() {
            weak_color
        } else {
            theme.palette().warning
        };

        row![
            text(format!("Avail: {}", format_optional_usd(available_margin)))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(available_color),
            Space::new().width(Fill),
            text(format!("Used: {}", format_optional_usd(margin_used)))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(margin_used_color),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
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
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn format_optional_usd(value: Option<f64>) -> String {
    value
        .map(|value| format_usd(&format!("{value:.2}")))
        .unwrap_or_else(|| "Invalid data".to_string())
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

fn order_leverage_badge(label: String) -> Element<'static, Message> {
    container(text(label).size(10).color(Color::WHITE))
        .padding([2, 6])
        .style(move |theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.strong.color.into()),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
