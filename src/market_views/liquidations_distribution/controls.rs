use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::liquidations_distribution_state::LIQUIDATION_DISTRIBUTION_ZOOM_STEP;
use crate::message::Message;
use iced::widget::{button, column, container, row, text, text_input, tooltip};
use iced::{Alignment, Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_liquidations_distribution_refresh_button(
        &self,
        loading: bool,
    ) -> Element<'static, Message> {
        let button = button(
            text("\u{21bb}")
                .size(13)
                .center()
                .font(crate::app_fonts::monospace_font()),
        )
        .padding([2, 7])
        .style(move |theme: &Theme, status| {
            if loading {
                subtle_liquidations_distribution_header_button(theme, button::Status::Disabled)
            } else {
                subtle_liquidations_distribution_header_button(theme, status)
            }
        });

        let button = if loading {
            button
        } else {
            button.on_press(Message::RefreshLiquidationsDistribution)
        };

        tooltip(
            button,
            text(if loading { "Refreshing" } else { "Refresh" }).size(10),
            tooltip::Position::Top,
        )
        .into()
    }

    pub(super) fn view_liquidations_distribution_symbol_button(
        &self,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let state = &self.liquidation_distribution;
        let selected = state.symbol.trim();
        let label = if selected.is_empty() {
            "Select market".to_string()
        } else {
            format!(
                "{} / USD",
                self.liquidation_distribution_symbol_display(selected)
            )
        };

        let mut content = row![].spacing(6).align_y(Alignment::Center);
        if !selected.is_empty()
            && let Some(icon) = helpers::symbol_icon(selected, 14, theme.palette().text)
        {
            content = content.push(icon);
        }
        content = content.push(
            text(label)
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text),
        );
        if let Some(dex) = helpers::hip3_dex(selected) {
            content = content.push(
                text(dex)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }
        content = content.push(
            text(if state.symbol_picker_open {
                "\u{25b2}"
            } else {
                "\u{25be}"
            })
            .size(8)
            .color(theme.extended_palette().background.weak.text),
        );

        button(content)
            .on_press(Message::ToggleLiquidationsDistributionSymbolPicker)
            .padding([2, 7])
            .style(move |theme: &Theme, status| {
                let bg = match (state.symbol_picker_open, status) {
                    (_, button::Status::Hovered) => theme.extended_palette().background.weak.color,
                    (true, _) => theme.extended_palette().background.weak.color,
                    (false, _) => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        radius: 3.0.into(),
                        width: if state.symbol_picker_open { 1.0 } else { 0.0 },
                        color: Color {
                            a: 0.35,
                            ..theme.palette().primary
                        },
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    pub(super) fn view_liquidations_distribution_symbol_dropdown(
        &self,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let state = &self.liquidation_distribution;
        let search = text_input("Search perp market...", &state.symbol_search_query)
            .style(helpers::text_input_style)
            .on_input(Message::LiquidationsDistributionSearchChanged)
            .size(12)
            .padding([5, 8]);

        let query = state.symbol_search_query.trim().to_lowercase();
        let mut matches: Vec<&ExchangeSymbol> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| symbol.market_type == MarketType::Perp)
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .filter(|symbol| liquidation_distribution_symbol_matches(symbol, &query))
            .collect();
        matches.sort_by(|a, b| {
            a.ticker
                .cmp(&b.ticker)
                .then_with(|| helpers::compare_symbol_keys_for_same_ticker(&a.key, &b.key))
        });
        matches.truncate(6);

        let mut results = column![].spacing(3);
        for symbol in matches {
            results = results.push(self.view_liquidations_distribution_symbol_row(symbol, theme));
        }

        container(column![search, results].spacing(5).padding(6))
            .width(Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                text_color: Some(theme.palette().text),
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: theme.extended_palette().background.weak.color,
                },
                ..Default::default()
            })
            .into()
    }

    fn view_liquidations_distribution_symbol_row<'a>(
        &'a self,
        symbol: &'a ExchangeSymbol,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let sym_key = &symbol.key;
        let display = Self::exchange_symbol_display_name(symbol);
        let mut content = row![].spacing(6).align_y(Alignment::Center);
        if let Some(icon) = helpers::symbol_icon(sym_key, 14, theme.palette().text) {
            content = content.push(icon);
        }
        content = content.push(
            text(display)
                .size(12)
                .color(theme.palette().text)
                .width(Fill),
        );
        if let Some(dex) = helpers::hip3_dex(sym_key) {
            content = content.push(
                text(dex)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }
        if symbol.growth_mode {
            content = content.push(helpers::growth_mode_chip());
        }

        button(content)
            .on_press(Message::LiquidationsDistributionSymbolSelected(
                symbol.key.clone(),
            ))
            .padding([4, 8])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
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
            })
            .width(Fill)
            .into()
    }

    pub(super) fn view_liquidations_distribution_zoom_controls(
        &self,
        zoom: f64,
        theme: &Theme,
    ) -> Element<'static, Message> {
        row![
            button(text("-").size(11).center())
                .padding([3, 7])
                .style(subtle_liquidations_distribution_header_button)
                .on_press(Message::LiquidationsDistributionZoomed {
                    factor: 1.0 / LIQUIDATION_DISTRIBUTION_ZOOM_STEP,
                    anchor: None,
                }),
            text(format!("{:.0}%", zoom * 100.0))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text),
            button(text("+").size(11).center())
                .padding([3, 7])
                .style(subtle_liquidations_distribution_header_button)
                .on_press(Message::LiquidationsDistributionZoomed {
                    factor: LIQUIDATION_DISTRIBUTION_ZOOM_STEP,
                    anchor: None,
                }),
            button(text("Reset").size(11).center())
                .padding([3, 7])
                .style(subtle_liquidations_distribution_header_button)
                .on_press(Message::ResetLiquidationsDistributionZoom),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
    }
}

fn subtle_liquidations_distribution_header_button(
    theme: &Theme,
    status: button::Status,
) -> button::Style {
    let background = match status {
        button::Status::Hovered => Some(
            Color {
                a: 0.06,
                ..theme.palette().text
            }
            .into(),
        ),
        _ => Some(Color::TRANSPARENT.into()),
    };

    button::Style {
        background,
        text_color: theme.extended_palette().background.weak.text,
        border: iced::Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn liquidation_distribution_symbol_matches(symbol: &ExchangeSymbol, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let display = symbol
        .display_name
        .as_deref()
        .unwrap_or(symbol.ticker.as_str())
        .to_lowercase();
    display.contains(query)
        || symbol.ticker.to_lowercase().contains(query)
        || symbol.key.to_lowercase().contains(query)
        || symbol
            .keywords
            .iter()
            .any(|keyword| keyword.to_lowercase().contains(query))
}
