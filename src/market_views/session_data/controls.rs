use super::tooltips::tooltip_body;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::session_data_state::{SessionDataId, SessionDataInstance, SessionDataLookback};

use iced::widget::{Space, button, column, container, row, text, text_input, tooltip};
use iced::{Alignment, Color, Element, Fill, Theme};

const STATUS_ERROR_CHARS: usize = 72;

// ---------------------------------------------------------------------------
// Session Data Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_session_data_header<'a>(
        &'a self,
        instance: &'a SessionDataInstance,
        theme: &Theme,
        available_width: f32,
    ) -> Element<'a, Message> {
        let display = self.display_name_for_symbol(&instance.symbol);
        let mut symbol_content = row![].spacing(5).align_y(Alignment::Center);
        if let Some(icon) = helpers::symbol_icon(&instance.symbol, 15, theme.palette().text) {
            symbol_content = symbol_content.push(icon);
        }
        symbol_content = symbol_content
            .push(text(display).size(12).color(theme.palette().text))
            .push(
                text(if instance.symbol_picker_open {
                    "\u{25b2}"
                } else {
                    "\u{25be}"
                })
                .size(9)
                .color(theme.extended_palette().background.weak.text),
            );

        let symbol_button = button(symbol_content)
            .on_press(Message::ToggleSessionDataSymbolPicker(instance.id))
            .padding([3, 8])
            .style(move |theme: &Theme, status| compact_button_style(theme, status, true));

        let mut lookbacks = row![].spacing(3).align_y(Alignment::Center);
        for lookback in SessionDataLookback::ALL {
            lookbacks = lookbacks.push(lookback_button(
                lookback,
                instance.lookback == lookback,
                instance.id,
            ));
        }

        let refresh_label: Element<'_, Message> = if instance.loading {
            self.view_spinner(14)
        } else {
            text("\u{21bb}")
                .size(13)
                .center()
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text)
                .into()
        };
        let refresh = tooltip(
            button(refresh_label)
                .on_press_maybe(
                    (!instance.loading).then_some(Message::RefreshSessionData(instance.id)),
                )
                .padding([3, 8])
                .style(move |theme: &Theme, status| compact_button_style(theme, status, false)),
            tooltip_body("Re-fetch session history for this market."),
            tooltip::Position::Bottom,
        );

        let legend = view_legend_icon(theme);

        let status = session_data_status(instance);
        let status_text = text(status)
            .size(10)
            .color(theme.extended_palette().background.weak.text);

        if available_width < 520.0 {
            column![
                row![symbol_button, Space::new().width(Fill), legend, refresh]
                    .spacing(6)
                    .align_y(Alignment::Center),
                lookbacks,
                status_text
            ]
            .spacing(6)
            .into()
        } else {
            row![
                symbol_button,
                lookbacks,
                Space::new().width(Fill),
                status_text,
                legend,
                refresh
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        }
    }

    pub(super) fn view_session_data_symbol_dropdown<'a>(
        &'a self,
        instance: &'a SessionDataInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let search = text_input("Search perp or spot...", &instance.search_query)
            .style(helpers::text_input_style)
            .on_input(move |q| Message::SessionDataSearchChanged(instance.id, q))
            .size(12)
            .padding([5, 8]);

        let mut results = column![].spacing(2);
        let query = instance.search_query.trim().to_ascii_lowercase();
        for symbol in self
            .exchange_symbols
            .iter()
            .filter(|symbol| matches!(symbol.market_type, MarketType::Perp | MarketType::Spot))
            .filter(|symbol| symbol.is_user_selectable_market())
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .filter(|symbol| {
                if query.is_empty() {
                    return true;
                }
                let display = Self::exchange_symbol_display_name(symbol).to_ascii_lowercase();
                symbol.key.to_ascii_lowercase().contains(&query)
                    || symbol.ticker.to_ascii_lowercase().contains(&query)
                    || display.contains(&query)
                    || symbol.category.to_ascii_lowercase().contains(&query)
            })
            .take(12)
        {
            let display = Self::exchange_symbol_display_name(symbol);
            let market = match symbol.market_type {
                MarketType::Perp => "perp",
                MarketType::Spot => "spot",
                MarketType::Outcome => "outcome",
            };
            let row = row![
                text(display).size(11).width(Fill),
                text(symbol.key.as_str())
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(theme.extended_palette().background.weak.text),
                text(market)
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            results = results.push(
                button(row)
                    .on_press(Message::SessionDataSymbolSelected(
                        instance.id,
                        symbol.key.clone(),
                    ))
                    .padding([5, 8])
                    .width(Fill)
                    .style(|theme: &Theme, status| compact_button_style(theme, status, false)),
            );
        }

        if self.exchange_symbols.is_empty() {
            results = results.push(
                text("Symbols loading")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        container(column![search, results].spacing(6).padding(6))
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
}

fn view_legend_icon(theme: &Theme) -> Element<'static, Message> {
    tooltip(
        text("\u{24d8}")
            .size(13)
            .color(theme.extended_palette().background.weak.text),
        tooltip_body(
            "How to read the lane:\n\u{2022} Bar = average return, centered at 0% (right = gains, left = losses).\n\u{2022} Bullet = win rate, with a tick at 50%.\n\u{2022} Fainter bars mean fewer samples; dots mean no completed sessions.\n\u{2022} The marked row is the strongest day or session.\nHover any row for its exact numbers.",
        ),
        tooltip::Position::Bottom,
    )
    .into()
}

fn lookback_button(
    lookback: SessionDataLookback,
    active: bool,
    id: SessionDataId,
) -> Element<'static, Message> {
    tooltip(
        button(text(lookback.label()).size(10).center())
            .on_press(Message::SessionDataLookbackChanged(id, lookback))
            .padding([3, 7])
            .style(move |theme: &Theme, status| compact_button_style(theme, status, active)),
        tooltip_body(lookback_tooltip(lookback)),
        tooltip::Position::Bottom,
    )
    .into()
}

fn lookback_tooltip(lookback: SessionDataLookback) -> &'static str {
    match lookback {
        SessionDataLookback::FourWeeks => "Summarize the last 4 weeks of completed sessions.",
        SessionDataLookback::EightWeeks => "Summarize the last 8 weeks of completed sessions.",
        SessionDataLookback::ThreeMonths => "Summarize the last 3 months of completed sessions.",
        SessionDataLookback::SixMonths => "Summarize the last 6 months of completed sessions.",
        SessionDataLookback::OneYear => "Summarize the last 12 months of completed sessions.",
    }
}

fn compact_button_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let bg = match (active, status) {
        (true, _) => theme.extended_palette().background.strong.color,
        (false, button::Status::Hovered) | (false, button::Status::Pressed) => {
            theme.extended_palette().background.weak.color
        }
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: if active {
            theme.palette().primary
        } else {
            theme.palette().text
        },
        border: iced::Border {
            radius: 3.0.into(),
            width: if active { 1.0 } else { 0.0 },
            color: if active {
                Color {
                    a: 0.42,
                    ..theme.palette().primary
                }
            } else {
                Color::TRANSPARENT
            },
        },
        ..Default::default()
    }
}

fn session_data_status(instance: &SessionDataInstance) -> String {
    if instance.loading {
        return "Loading".to_string();
    }
    if let Some(error) = &instance.error
        && instance.bars.is_empty()
    {
        return helpers::ellipsized_text(error, STATUS_ERROR_CHARS);
    }
    format!("{} sessions", instance.bars.len())
}
