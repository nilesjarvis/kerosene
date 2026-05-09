use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use iced::widget::{button, row, text, text_input};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_spaghetti_editor_top_bar(
        &self,
        id: SpaghettiChartId,
        inst: &SpaghettiChartInstance,
    ) -> Element<'_, Message> {
        let search_bar = text_input("Search symbols to compare...", &inst.editor_search_query)
            .style(helpers::text_input_style)
            .on_input(move |q| Message::SpaghettiEditorSearchChanged(id, q))
            .size(12)
            .padding([5, 8]);

        let close_btn = button(text("Done").size(11).center())
            .on_press(Message::SpaghettiCloseEditor(id))
            .padding([3, 8])
            .style(|theme: &Theme, _status| button::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
                text_color: theme.palette().success,
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
