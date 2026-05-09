use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::helpers;
use crate::message::Message;
use iced::widget::{button, row, text, text_input};
use iced::{Element, Theme, color};

impl TradingTerminal {
    pub(super) fn view_chart_editor_top_bar(
        &self,
        chart_id: ChartId,
        instance: &ChartInstance,
    ) -> Element<'_, Message> {
        let search_bar = text_input("Search symbols...", &instance.editor_search_query)
            .id(Self::chart_symbol_search_input_id(chart_id))
            .style(helpers::text_input_style)
            .on_input(move |q| Message::ChartEditorSearchChanged(chart_id, q))
            .on_submit(Message::ChartEditorSubmit(chart_id))
            .size(12)
            .padding([5, 8]);

        let close_btn = button(text("X").size(11).center())
            .on_press(Message::ChartCloseEditor(chart_id))
            .padding([3, 6])
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(color!(0x3a3a3a).into()),
                text_color: color!(0xaaaaaa),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        row![search_bar, close_btn]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .into()
    }
}
