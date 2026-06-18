use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::helpers::{self, category_color};
use crate::message::Message;
use iced::widget::{Space, button, row, text};
use iced::{Color, Element, Fill, Theme, color};

#[derive(Debug, Clone, Copy)]
struct ChartEditorSymbolRowState {
    is_fav: bool,
    is_selected: bool,
    is_keyboard_selected: bool,
    secondary: bool,
}

impl TradingTerminal {
    pub(super) fn view_chart_editor_symbol_row<'a>(
        &'a self,
        chart_id: ChartId,
        sym: &'a ExchangeSymbol,
        is_fav: bool,
        is_selected: bool,
        is_keyboard_selected: bool,
        theme: &Theme,
    ) -> Element<'a, Message> {
        self.view_chart_editor_symbol_row_for(
            chart_id,
            sym,
            ChartEditorSymbolRowState {
                is_fav,
                is_selected,
                is_keyboard_selected,
                secondary: false,
            },
            theme,
        )
    }

    pub(super) fn view_chart_secondary_editor_symbol_row<'a>(
        &'a self,
        chart_id: ChartId,
        sym: &'a ExchangeSymbol,
        is_fav: bool,
        is_selected: bool,
        is_keyboard_selected: bool,
        theme: &Theme,
    ) -> Element<'a, Message> {
        self.view_chart_editor_symbol_row_for(
            chart_id,
            sym,
            ChartEditorSymbolRowState {
                is_fav,
                is_selected,
                is_keyboard_selected,
                secondary: true,
            },
            theme,
        )
    }

    fn view_chart_editor_symbol_row_for<'a>(
        &'a self,
        chart_id: ChartId,
        sym: &'a ExchangeSymbol,
        row_state: ChartEditorSymbolRowState,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let ChartEditorSymbolRowState {
            is_fav,
            is_selected,
            is_keyboard_selected,
            secondary,
        } = row_state;
        let display = sym.display_name.as_deref().unwrap_or(&sym.ticker);
        let prefix = sym.key.split(':').next().unwrap_or("");
        let cat_badge = text(sym.category.to_uppercase())
            .size(9)
            .color(category_color(&sym.category, theme));

        let star_key = sym.key.clone();
        let star_btn = button(text(if is_fav { "★" } else { "☆" }).size(12))
            .on_press(Message::ToggleFavourite(star_key))
            .padding([0, 4])
            .style(move |theme: &Theme, status| {
                let text_color = if is_fav {
                    theme.palette().primary
                } else {
                    match status {
                        button::Status::Hovered => theme.palette().primary,
                        _ => color!(0x666666),
                    }
                };
                button::Style {
                    background: None,
                    text_color,
                    ..Default::default()
                }
            });

        let key = sym.key.clone();
        let mut coin_content = row![];
        if let Some(icon) = helpers::symbol_icon(&sym.key, 14, theme.palette().text) {
            coin_content = coin_content.push(icon).push(Space::new().width(4.0));
        }
        coin_content = coin_content
            .push(text(display).size(12).width(Fill))
            .push(text(prefix).size(9).color(color!(0x666666)))
            .push(cat_badge)
            .spacing(6)
            .align_y(iced::Alignment::Center);

        let row_btn = button(coin_content)
            .on_press(if secondary {
                Message::ChartSecondarySymbolSelected(chart_id, key)
            } else {
                Message::ChartSymbolSelected(chart_id, key)
            })
            .padding([3, 6])
            .width(Fill)
            .style(move |theme: &Theme, status| {
                let bg = if is_selected || is_keyboard_selected {
                    match status {
                        button::Status::Hovered => theme.extended_palette().background.strong.color,
                        _ => color!(0x2a2a4a),
                    }
                } else {
                    match status {
                        button::Status::Hovered => theme.extended_palette().background.weak.color,
                        _ => theme.extended_palette().background.strong.color,
                    }
                };
                let text_color = theme.palette().text;
                let border_color = if is_selected || is_keyboard_selected {
                    Color {
                        a: 0.4,
                        ..theme.palette().primary
                    }
                } else {
                    Color::TRANSPARENT
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color,
                    border: iced::Border {
                        radius: 2.0.into(),
                        width: if is_selected || is_keyboard_selected {
                            1.0
                        } else {
                            0.0
                        },
                        color: border_color,
                    },
                    ..Default::default()
                }
            });

        row![star_btn, row_btn]
            .spacing(2)
            .align_y(iced::Alignment::Center)
            .into()
    }
}
