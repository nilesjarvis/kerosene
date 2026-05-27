use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{button, pane_grid, svg, text, tooltip};
use iced::{Element, Length, Theme};

const DETACH_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M15 3h6v6"/>
  <path d="M10 14 21 3"/>
  <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
</svg>
"#;

const COLLAPSE_HEADER_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4 5h16"/>
  <path d="m6 16 6-6 6 6"/>
</svg>
"#;

const EXPAND_HEADER_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4 5h16"/>
  <path d="m6 10 6 6 6-6"/>
</svg>
"#;

impl TradingTerminal {
    pub(crate) fn view_chart_header_collapse_button(
        &self,
        chart_id: ChartId,
        collapsed: bool,
    ) -> Element<'static, Message> {
        let icon_svg = if collapsed {
            EXPAND_HEADER_ICON_SVG
        } else {
            COLLAPSE_HEADER_ICON_SVG
        };
        let label = if collapsed {
            "Expand chart header"
        } else {
            "Collapse chart header"
        };

        let icon: Element<'static, Message> = svg(SvgHandle::from_memory(icon_svg))
            .width(Length::Fixed(12.0))
            .height(Length::Fixed(12.0))
            .style(|theme: &Theme, _status| svg::Style {
                color: Some(theme.palette().text),
            })
            .into();

        tooltip(
            button(icon)
                .on_press(Message::ToggleChartHeaderCollapsed(chart_id))
                .padding([2, 5])
                .style(move |theme: &Theme, status| {
                    let bg = match status {
                        button::Status::Hovered => theme.extended_palette().background.strong.color,
                        _ if collapsed => theme.extended_palette().background.strong.color,
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
            text(label)
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Bottom,
        )
        .into()
    }

    pub(crate) fn view_chart_add_button(&self, pane: pane_grid::Pane) -> Element<'static, Message> {
        tooltip(
            button(text("+").size(11).center())
                .on_press(Message::AddChart(pane))
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
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
            text("Add candlestick chart")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Bottom,
        )
        .into()
    }

    pub(crate) fn view_detach_chart_button(&self, chart_id: ChartId) -> Element<'static, Message> {
        let icon: Element<'static, Message> = svg(SvgHandle::from_memory(DETACH_ICON_SVG))
            .width(Length::Fixed(12.0))
            .height(Length::Fixed(12.0))
            .style(|theme: &Theme, _status| svg::Style {
                color: Some(theme.palette().text),
            })
            .into();

        tooltip(
            button(icon)
                .on_press(Message::OpenDetachedChart(chart_id))
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
            text("Open chart in new window")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Bottom,
        )
        .into()
    }
}
