use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use iced::widget::{button, row, svg, text, tooltip};
use iced::{Element, Length, Theme};

const DETACH_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M15 3h6v6"/>
  <path d="M10 14 21 3"/>
  <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
</svg>
"#;

impl TradingTerminal {
    pub(super) fn view_spaghetti_toolbar(
        &self,
        id: SpaghettiChartId,
        inst: &SpaghettiChartInstance,
    ) -> Element<'static, Message> {
        let theme = self.theme();
        let mut toolbar = row![].spacing(4).align_y(iced::Alignment::Center);

        for series in &inst.canvas.series {
            let sym = series.symbol.clone();
            let sid = id;
            let text_color = inst.canvas.series_render_color(&theme, series);
            let remove_btn = button(
                text(format!("{} x", series.display))
                    .size(10)
                    .color(text_color),
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

        toolbar = toolbar.push(edit_btn);

        let detach_icon: Element<'static, Message> =
            svg(iced::widget::svg::Handle::from_memory(DETACH_ICON_SVG))
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(|theme: &Theme, _status| iced::widget::svg::Style {
                    color: Some(theme.palette().text),
                })
                .into();

        let detach_btn = tooltip(
            button(detach_icon)
                .on_press(Message::OpenDetachedSpaghettiChart(id))
                .padding([2, 5])
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
                }),
            text("Open comparison chart in new window")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Bottom,
        );

        toolbar.push(detach_btn).into()
    }
}
