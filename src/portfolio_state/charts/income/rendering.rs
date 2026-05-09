use crate::account_metrics::format_signed_usd_value;

use super::series::{hovered_income_bar, income_tooltip_layout, prepare_income_chart_layout};
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

pub(super) fn draw_income_projection_chart(
    bars: &[(String, f64)],
    renderer: &Renderer,
    theme: &Theme,
    bounds: Rectangle,
    cursor: iced::mouse::Cursor,
) -> Vec<canvas::Geometry> {
    let mut frame = canvas::Frame::new(renderer, bounds.size());
    frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

    let Some(layout) = prepare_income_chart_layout(bars, bounds.width, bounds.height) else {
        return vec![frame.into_geometry()];
    };

    let baseline = canvas::Path::line(
        Point::new(layout.left_pad, layout.zero_y),
        Point::new(layout.left_pad + layout.plot_width, layout.zero_y),
    );
    frame.stroke(
        &baseline,
        canvas::Stroke::default()
            .with_color(Color {
                a: 0.14,
                ..theme.palette().text
            })
            .with_width(1.0),
    );

    for bar in &layout.bars {
        let color = if bar.value >= 0.0 {
            theme.palette().success
        } else {
            theme.palette().danger
        };
        frame.fill_rectangle(
            Point::new(bar.x, bar.y),
            Size::new(bar.width, bar.height),
            color,
        );

        if bar.show_axis_label {
            frame.fill_text(canvas::Text {
                content: bar.label.clone(),
                position: Point::new(bar.center_x, bounds.height - layout.bottom_pad + 6.0),
                color: Color {
                    a: 0.55,
                    ..theme.palette().text
                },
                size: iced::Pixels(10.0),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Top,
                font: iced::Font::MONOSPACE,
                ..canvas::Text::default()
            });
        }
    }

    if let Some(pos) = cursor.position_in(bounds)
        && let Some(bar) = hovered_income_bar(&layout, pos)
    {
        let value_text = format_signed_usd_value(bar.value);
        let tooltip = income_tooltip_layout(bar, &value_text, bounds.width, bounds.height);

        frame.fill_rectangle(
            tooltip.origin,
            Size::new(tooltip.width, tooltip.height),
            Color {
                a: 0.93,
                ..theme.extended_palette().background.strong.color
            },
        );
        let border =
            canvas::Path::rectangle(tooltip.origin, Size::new(tooltip.width, tooltip.height));
        frame.stroke(
            &border,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.16,
                    ..theme.palette().text
                })
                .with_width(1.0),
        );

        frame.fill_text(canvas::Text {
            content: bar.label.clone(),
            position: Point::new(tooltip.origin.x + 8.0, tooltip.origin.y + 8.0),
            color: theme.palette().text,
            size: iced::Pixels(10.0),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Top,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
        frame.fill_text(canvas::Text {
            content: value_text,
            position: Point::new(tooltip.origin.x + 8.0, tooltip.origin.y + 22.0),
            color: if bar.value >= 0.0 {
                theme.palette().success
            } else {
                theme.palette().danger
            },
            size: iced::Pixels(11.0),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Top,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
    }

    vec![frame.into_geometry()]
}
