use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use iced::widget::{button, pane_grid, text};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_chart_indicator_button(
        &self,
        chart_id: ChartId,
        macro_menu_open: bool,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let text_color = if macro_menu_open {
            theme.palette().success
        } else {
            theme.palette().text
        };

        button(text("Indicators").size(11).color(text_color))
            .on_press(Message::ToggleMacroMenu(chart_id))
            .padding([6, 8])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    pub(super) fn view_chart_add_button(&self, pane: pane_grid::Pane) -> Element<'static, Message> {
        button(text("+").size(11).center())
            .on_press(Message::AddChart(pane))
            .padding([6, 8])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().success,
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}
