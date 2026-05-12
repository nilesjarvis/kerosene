use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::helpers;
use crate::message::Message;
use iced::widget::{Space, button, column, row, text};
use iced::{Color, Element, Theme};

impl TradingTerminal {
    pub(super) fn view_chart_placeholder_header<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        button(self.view_chart_symbol_title(instance, theme))
            .on_press(Message::ChartOpenEditor(chart_id))
            .padding([4, 6])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    pub(super) fn view_chart_symbol_button<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
        last_close: f64,
        price_color: Color,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let sym_col = column![
            self.view_chart_symbol_title(instance, theme),
            text(helpers::format_price(last_close))
                .size(16)
                .font(iced::Font::MONOSPACE)
                .color(price_color),
        ]
        .spacing(2);

        button(sym_col)
            .on_press(Message::ChartOpenEditor(chart_id))
            .padding([4, 6])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    fn view_chart_symbol_title<'a>(
        &self,
        instance: &'a ChartInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let mut title = row![];
        if let Some(icon) = helpers::symbol_icon(&instance.symbol, 18, theme.palette().text)
            .or_else(|| helpers::symbol_icon(&instance.symbol_display, 18, theme.palette().text))
        {
            title = title.push(icon).push(Space::new().width(6.0));
        }

        title = title.push(
            text(&instance.symbol_display)
                .size(14)
                .color(theme.palette().text),
        );

        if let Some(dex) = helpers::hip3_dex(&instance.symbol) {
            title = title.push(Space::new().width(4.0)).push(
                text(format!("({dex})"))
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        title.align_y(iced::Alignment::Center).into()
    }
}
