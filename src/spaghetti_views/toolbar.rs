use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use iced::widget::{button, row, text};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_spaghetti_toolbar(
        &self,
        id: SpaghettiChartId,
        inst: &SpaghettiChartInstance,
    ) -> Element<'static, Message> {
        let mut toolbar = row![].spacing(4).align_y(iced::Alignment::Center);

        for series in &inst.canvas.series {
            let sym = series.symbol.clone();
            let sid = id;
            let remove_btn = button(
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
            });
            toolbar = toolbar.push(remove_btn);
        }

        let edit_btn = button(text("+").size(12).center())
            .on_press(Message::SpaghettiOpenEditor(id))
            .padding([2, 6])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().success,
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });

        toolbar.push(edit_btn).into()
    }
}
