use super::super::metrics::format_positioning_timestamp;
use crate::app_state::TradingTerminal;
use crate::config::effective_radius;
use crate::helpers;
use crate::helpers::design::{UiColors, compact_button_style, compact_input_style};
use crate::helpers::icons::{self, IconColor};
use crate::message::Message;
use crate::positioning_state::{PositioningInfoInstance, PositioningInfoPage};

use iced::widget::{Row, button, column, container, row, text, text_input, tooltip};
use iced::{Alignment, Element, Fill, Theme};

mod autocomplete;

// ---------------------------------------------------------------------------
// Positioning Symbol Picker
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::market_views::positioning_info) fn view_positioning_info_title<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        now_ms: u64,
        theme: &Theme,
        compact_symbol_picker: bool,
    ) -> Element<'a, Message> {
        let pane_corner_radius = self.pane_corner_radius;
        let display = self.positioning_info_symbol_display(&instance.symbol);
        let mut symbol_row = Row::new().spacing(6).align_y(Alignment::Center);
        if compact_symbol_picker {
            // Identity order: logo, symbol, then picker affordance.
            let mut picker_content = Row::new().spacing(5).align_y(Alignment::Center);
            if let Some(icon) = helpers::symbol_icon(&instance.symbol, 14, theme.palette().text) {
                picker_content = picker_content.push(icon);
            }
            picker_content = picker_content
                .push(
                    text(display)
                        .size(12)
                        .style(move |theme: &Theme| text::Style {
                            color: Some(UiColors::new(theme).text),
                        }),
                )
                .push(icons::icon(
                    if instance.symbol_picker_open {
                        icons::CHEVRON_UP
                    } else {
                        icons::CHEVRON_DOWN
                    },
                    10.0,
                    IconColor::Muted,
                ));

            symbol_row = symbol_row.push(
                button(picker_content)
                    .on_press(Message::TogglePositioningInfoSymbolPicker(instance.id))
                    .height(24)
                    .padding([0, 8])
                    .style(move |theme: &Theme, status| {
                        compact_button_style(
                            theme,
                            status,
                            instance.symbol_picker_open,
                            pane_corner_radius,
                        )
                    }),
            );
        } else {
            if let Some(icon) = helpers::symbol_icon(&instance.symbol, 16, theme.palette().text) {
                symbol_row = symbol_row.push(icon);
            }
            symbol_row = symbol_row.push(
                text(format!("Positioning Information ({display})"))
                    .size(13)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).text),
                    }),
            );
        }
        if let Some(dex) = helpers::hip3_dex(&instance.symbol) {
            symbol_row = symbol_row.push(text(format!("({dex})")).size(11).style(
                move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).muted),
                },
            ));
        }

        let (loading, last_fetch_ms) = match instance.page {
            PositioningInfoPage::Positions => (instance.loading, instance.last_fetch_ms),
            PositioningInfoPage::Change => (instance.change_loading, instance.change_last_fetch_ms),
        };

        let tooltip_label = if loading {
            "Refreshing…".to_string()
        } else {
            last_fetch_ms
                .map(|last| {
                    format!(
                        "Refresh • updated {} ago",
                        helpers::format_relative_time(last, now_ms)
                    )
                })
                .unwrap_or_else(|| "Refresh • not loaded".to_string())
        };

        // Vector refresh icon on a compact 24px target; a spinner replaces it
        // while refreshing without changing the button frame.
        let icon: Element<'_, Message> = if loading {
            self.view_spinner(14)
        } else {
            icons::icon(icons::REFRESH, 14.0, IconColor::Muted)
        };

        let mut refresh_button =
            button(icon)
                .width(24)
                .height(24)
                .padding(0)
                .style(move |theme: &Theme, status| {
                    compact_button_style(theme, status, false, pane_corner_radius)
                });
        if !loading {
            refresh_button =
                refresh_button.on_press(Message::RefreshPositioningInfoPane(instance.id));
        }

        let refresh = tooltip(
            refresh_button,
            container(text(tooltip_label).size(11))
                .padding([6, 8])
                .max_width(280)
                .style(move |theme: &Theme| UiColors::new(theme).overlay(pane_corner_radius)),
            tooltip::Position::Top,
        );

        let fetched = last_fetch_ms
            .map(|last| {
                format!(
                    "Fetched {} ago",
                    helpers::format_relative_time(last, now_ms)
                )
            })
            .unwrap_or_else(|| "Not fetched".to_string());
        let fetched = text(fetched)
            .size(11)
            .style(move |theme: &Theme| text::Style {
                color: Some(UiColors::new(theme).muted),
            });
        let fetched: Element<'_, Message> = if instance.page == PositioningInfoPage::Positions
            && let Some(data) = &instance.data
        {
            tooltip(
                fetched,
                container(
                    text(format!(
                        "Data updated {}",
                        format_positioning_timestamp(&data.timestamp)
                    ))
                    .size(11),
                )
                .padding([6, 8])
                .max_width(280)
                .style(move |theme: &Theme| UiColors::new(theme).overlay(pane_corner_radius)),
                tooltip::Position::Top,
            )
            .into()
        } else {
            fetched.into()
        };

        row![symbol_row.width(Fill), fetched, refresh]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
    }

    pub(in crate::market_views::positioning_info) fn view_positioning_info_symbol_dropdown<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let pane_corner_radius = self.pane_corner_radius;
        let search = text_input("Search perp ticker...", &instance.search_query)
            .id(Self::positioning_symbol_search_input_id(instance.id))
            .on_input(move |q| Message::PositioningInfoSearchChanged(instance.id, q))
            .size(12)
            .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(16.0)))
            .padding([4, 8])
            .style(move |theme, status| {
                compact_input_style(theme, status, false, pane_corner_radius)
            });
        let autocomplete =
            self.view_positioning_info_autocomplete(instance.id, &instance.search_query, theme);

        let content = column![search, autocomplete].spacing(4).padding(4);

        container(content)
            .width(Fill)
            .style(move |theme: &Theme| container::Style {
                background: Some(UiColors::new(theme).inset.into()),
                text_color: Some(UiColors::new(theme).text),
                border: iced::Border {
                    radius: effective_radius(pane_corner_radius, 6.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    fn positioning_info_symbol_display<'a>(&'a self, symbol: &'a str) -> &'a str {
        self.exchange_symbols
            .iter()
            .find(|candidate| candidate.key == symbol)
            .map(|candidate| {
                candidate
                    .display_name
                    .as_deref()
                    .unwrap_or(&candidate.ticker)
            })
            .unwrap_or(symbol)
    }
}
