use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use iced::widget::{button, row, text};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_spaghetti_editor_selected_chips(
        &self,
        id: SpaghettiChartId,
        inst: &SpaghettiChartInstance,
    ) -> Element<'_, Message> {
        inst.canvas
            .series
            .iter()
            .fold(
                row![].spacing(4).align_y(iced::Alignment::Center),
                |r, series| {
                    let sym = series.symbol.clone();
                    let sid = id;
                    r.push(
                        button(
                            text(format!("{} x", series.display))
                                .size(10)
                                .color(series.color),
                        )
                        .on_press(Message::SpaghettiRemoveSymbol(sid, sym))
                        .padding([1, 4])
                        .style(|theme: &Theme, _status| button::Style {
                            background: Some(theme.extended_palette().background.weak.color.into()),
                            border: iced::Border {
                                radius: 2.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    )
                },
            )
            .into()
    }
}
