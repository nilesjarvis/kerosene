use crate::journal_views::analytics::JournalKpis;
use crate::journal_views::style::{journal_muted, journal_surface_sunken};
use crate::message::Message;
use iced::widget::{canvas, column, row, text};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

const DONUT_SIZE: f32 = 150.0;

pub(super) fn view_journal_winloss_body(
    kpis: &JournalKpis,
    denomination: &crate::denomination::DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let donut = canvas(JournalDonut {
        win_fraction: if kpis.scored > 0 {
            kpis.wins as f32 / kpis.scored as f32
        } else {
            0.0
        },
        win_rate: kpis.win_rate as f32,
        scored: kpis.scored,
    })
    .width(Length::Fixed(DONUT_SIZE))
    .height(Length::Fixed(DONUT_SIZE));

    let legend = column![
        winloss_metric(
            "WINS",
            kpis.wins.to_string(),
            theme.palette().success,
            theme
        ),
        winloss_metric(
            "LOSSES",
            kpis.losses.to_string(),
            theme.palette().danger,
            theme
        ),
        winloss_metric(
            "EXPECTANCY",
            kpis.expectancy
                .map(|value| denomination.format_signed_value(value, 2))
                .unwrap_or_else(|| "—".to_string()),
            kpis.expectancy
                .map(|value| crate::helpers::signed_number_color(value, theme))
                .unwrap_or(theme.palette().text),
            theme,
        ),
    ]
    .spacing(12);

    row![donut, legend]
        .spacing(18)
        .align_y(Alignment::Center)
        .into()
}

fn winloss_metric(
    label: &'static str,
    value: String,
    value_color: Color,
    theme: &Theme,
) -> Element<'static, Message> {
    column![
        text(label)
            .size(9)
            .font(crate::app_fonts::monospace_font())
            .color(journal_muted(theme)),
        text(value)
            .size(16)
            .font(crate::app_fonts::monospace_font())
            .color(value_color),
    ]
    .spacing(3)
    .into()
}

// ---- Win/Loss donut canvas ----

#[derive(Debug, Clone)]
struct JournalDonut {
    win_fraction: f32,
    win_rate: f32,
    scored: usize,
}

impl canvas::Program<Message> for JournalDonut {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        draw_donut(&mut frame, theme, bounds.size(), self);
        vec![frame.into_geometry()]
    }
}

fn draw_donut(frame: &mut canvas::Frame, theme: &Theme, size: Size, donut: &JournalDonut) {
    let dimension = size.width.min(size.height);
    if dimension <= 8.0 {
        return;
    }
    let center = Point::new(size.width / 2.0, size.height / 2.0);
    let thickness = dimension * 0.16;
    let radius = dimension / 2.0 - thickness / 2.0 - 2.0;
    let track_color = journal_surface_sunken(theme);
    let win_color = theme.palette().success;
    let loss_color = theme.palette().danger;

    // Background track.
    stroke_arc(
        frame,
        center,
        radius,
        0.0,
        std::f32::consts::TAU,
        thickness,
        track_color,
    );

    if donut.scored > 0 {
        let start = -std::f32::consts::FRAC_PI_2;
        let win_sweep = std::f32::consts::TAU * donut.win_fraction.clamp(0.0, 1.0);
        stroke_arc(
            frame,
            center,
            radius,
            start,
            start + win_sweep,
            thickness,
            win_color,
        );
        stroke_arc(
            frame,
            center,
            radius,
            start + win_sweep,
            start + std::f32::consts::TAU,
            thickness,
            loss_color,
        );
    }

    let label = if donut.scored > 0 {
        format!("{:.0}%", donut.win_rate)
    } else {
        "—".to_string()
    };
    frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(center.x, center.y - 8.0),
        color: theme.palette().text,
        size: iced::Pixels(dimension * 0.2),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
    frame.fill_text(canvas::Text {
        content: "WIN RATE".to_string(),
        position: Point::new(center.x, center.y + 12.0),
        color: journal_muted(theme),
        size: iced::Pixels(9.0),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn stroke_arc(
    frame: &mut canvas::Frame,
    center: Point,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    thickness: f32,
    color: Color,
) {
    if (end_angle - start_angle).abs() < 1e-4 {
        return;
    }
    let steps = (((end_angle - start_angle).abs() / std::f32::consts::TAU) * 96.0)
        .ceil()
        .max(2.0) as usize;
    let path = canvas::Path::new(|builder| {
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let angle = start_angle + (end_angle - start_angle) * t;
            let point = Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );
            if step == 0 {
                builder.move_to(point);
            } else {
                builder.line_to(point);
            }
        }
    });
    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_color(color)
            .with_width(thickness)
            .with_line_cap(canvas::LineCap::Butt),
    );
}
