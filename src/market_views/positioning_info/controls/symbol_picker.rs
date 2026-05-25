use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::positioning_state::{PositioningInfoInstance, PositioningInfoPage};

use iced::widget::{Row, button, column, container, row, text, text_input};
use iced::{Alignment, Color, Element, Fill, Theme};

mod autocomplete;

// ---------------------------------------------------------------------------
// Positioning Symbol Picker
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::market_views::positioning_info) fn view_positioning_info_title<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        theme: &Theme,
        compact_symbol_picker: bool,
    ) -> Element<'a, Message> {
        let display = self.positioning_info_symbol_display(&instance.symbol);
        let mut symbol_row = Row::new().spacing(6).align_y(Alignment::Center);
        if compact_symbol_picker {
            let mut picker_content = Row::new().spacing(5).align_y(Alignment::Center);
            if let Some(icon) = helpers::symbol_icon(&instance.symbol, 14, theme.palette().text) {
                picker_content = picker_content.push(icon);
            }
            picker_content = picker_content
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

            symbol_row = symbol_row.push(
                button(picker_content)
                    .on_press(Message::TogglePositioningInfoSymbolPicker(instance.id))
                    .padding([2, 7])
                    .style(move |theme: &Theme, status| {
                        let bg = match (instance.symbol_picker_open, status) {
                            (_, button::Status::Hovered) => {
                                theme.extended_palette().background.strong.color
                            }
                            (true, _) => theme.extended_palette().background.strong.color,
                            (false, _) => theme.extended_palette().background.weak.color,
                        };
                        button::Style {
                            background: Some(bg.into()),
                            text_color: theme.palette().text,
                            border: iced::Border {
                                radius: 3.0.into(),
                                width: if instance.symbol_picker_open {
                                    1.0
                                } else {
                                    0.0
                                },
                                color: Color {
                                    a: 0.45,
                                    ..theme.palette().primary
                                },
                            },
                            ..Default::default()
                        }
                    }),
            );
        } else {
            if let Some(icon) = helpers::symbol_icon(&instance.symbol, 16, theme.palette().text) {
                symbol_row = symbol_row.push(icon);
            }
            symbol_row = symbol_row.push(
                text(format!("Positioning Information ({display})"))
                    .size(13)
                    .color(theme.palette().text),
            );
        }
        if let Some(dex) = helpers::hip3_dex(&instance.symbol) {
            symbol_row = symbol_row.push(
                text(format!("({dex})"))
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        let (loading, last_fetch_ms) = match instance.page {
            PositioningInfoPage::Positions => (instance.loading, instance.last_fetch_ms),
            PositioningInfoPage::Change => (instance.change_loading, instance.change_last_fetch_ms),
        };

        let status: Element<'_, Message> = if loading {
            row![
                self.view_spinner(14),
                text("Refreshing")
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(5)
            .align_y(Alignment::Center)
            .into()
        } else {
            text(
                last_fetch_ms
                    .map(|last| {
                        format!(
                            "{} ago",
                            helpers::format_relative_time(last, TradingTerminal::now_ms())
                        )
                    })
                    .unwrap_or_else(|| "Not loaded".to_string()),
            )
            .size(10)
            .color(theme.extended_palette().background.weak.text)
            .into()
        };

        row![
            symbol_row.width(Fill),
            status,
            button(text("Refresh").size(10))
                .style(button::text)
                .on_press(Message::RefreshPositioningInfoPane(instance.id))
                .padding([2, 6]),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    pub(in crate::market_views::positioning_info) fn view_positioning_info_symbol_dropdown<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let search = text_input("Search perp ticker...", &instance.search_query)
            .id(Self::positioning_symbol_search_input_id(instance.id))
            .style(helpers::text_input_style)
            .on_input(move |q| Message::PositioningInfoSearchChanged(instance.id, q))
            .size(12)
            .padding([5, 8]);
        let autocomplete =
            self.view_positioning_info_autocomplete(instance.id, &instance.search_query, theme);

        let content = column![search, autocomplete].spacing(5).padding(6);

        container(content)
            .width(Fill)
            .style(|theme: &Theme| iced::widget::container::Style {
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

    fn positioning_info_symbol_display(&self, symbol: &str) -> String {
        self.exchange_symbols
            .iter()
            .find(|candidate| candidate.key == symbol)
            .map(|candidate| {
                candidate
                    .display_name
                    .as_deref()
                    .unwrap_or(&candidate.ticker)
                    .to_string()
            })
            .unwrap_or_else(|| symbol.to_string())
    }
}
