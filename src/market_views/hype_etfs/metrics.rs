use crate::message::Message;

use iced::widget::{Column, column, container, row, text};
use iced::{Color, Element, Fill, Theme, color};

// ---------------------------------------------------------------------------
// HYPE ETF Metric Cards
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(super) struct Metric {
    label: &'static str,
    value: String,
    color: Option<Color>,
}

pub(super) fn metric(label: &'static str, value: String, color: Option<Color>) -> Metric {
    Metric {
        label,
        value,
        color,
    }
}

pub(super) fn metric_grid(metrics: Vec<Metric>, available_width: f32) -> Element<'static, Message> {
    let columns = if available_width >= 560.0 {
        3
    } else if available_width >= 360.0 {
        2
    } else {
        1
    };

    let mut grid = Column::new().spacing(6);
    for chunk in metrics.chunks(columns) {
        let mut line = row![].spacing(6).width(Fill);
        for item in chunk {
            line = line.push(metric_card(item.clone()));
        }
        grid = grid.push(line);
    }
    grid.into()
}

fn metric_card(metric: Metric) -> Element<'static, Message> {
    let value_color = metric.color;
    container(
        column![
            text(metric.label)
                .size(10)
                .color(color!(0x888888))
                .width(Fill),
            text(metric.value)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .style(move |theme: &Theme| text::Style {
                    color: Some(value_color.unwrap_or(theme.palette().text)),
                })
                .width(Fill),
        ]
        .spacing(2),
    )
    .width(Fill)
    .padding([6, 8])
    .style(move |theme: &Theme| container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: iced::Border {
            radius: 3.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    })
    .into()
}
