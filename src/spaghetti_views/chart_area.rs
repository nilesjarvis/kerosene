use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartInstance;
use iced::widget::{canvas, container, stack, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_spaghetti_chart_area<'a>(
        &self,
        inst: &'a SpaghettiChartInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let chart_canvas = canvas(&inst.canvas).width(Fill).height(Fill);

        if inst.canvas.series.is_empty() {
            let placeholder = container(
                text("Click + to add symbols to compare")
                    .size(14)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill);
            stack![chart_canvas, placeholder]
                .width(Fill)
                .height(Fill)
                .into()
        } else {
            stack![chart_canvas].width(Fill).height(Fill).into()
        }
    }
}
