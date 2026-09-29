use crate::journal_views::analytics::{JournalHeatCell, JournalTimeOfDay};
use crate::journal_views::style::{journal_muted, journal_surface_sunken};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Border, Color, Element, Length, Theme};

const HEAT_CELL: f32 = 26.0;

pub(super) fn view_journal_heatmap(
    heatmap: &JournalTimeOfDay,
    theme: &Theme,
) -> Element<'static, Message> {
    const DAYS: [&str; 5] = ["MON", "TUE", "WED", "THU", "FRI"];
    const HOURS: [&str; 6] = ["00", "04", "08", "12", "16", "20"];

    let header = {
        let mut row = row![container(Space::new()).width(Length::Fixed(34.0))].spacing(4);
        for hour in HOURS {
            row = row.push(
                container(
                    text(hour)
                        .size(9)
                        .font(crate::app_fonts::monospace_font())
                        .color(journal_muted(theme)),
                )
                .width(Length::Fixed(HEAT_CELL))
                .align_x(iced::alignment::Horizontal::Center),
            );
        }
        row.align_y(Alignment::Center)
    };

    let mut grid = column![header].spacing(4);
    for (day_index, day) in DAYS.iter().enumerate() {
        let mut day_row = row![
            container(
                text(*day)
                    .size(9)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_muted(theme)),
            )
            .width(Length::Fixed(34.0))
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        for bucket in 0..6 {
            let cell = heatmap.cells[day_index][bucket];
            day_row = day_row.push(heatmap_cell(cell, heatmap.max_abs_pnl, theme));
        }
        grid = grid.push(day_row);
    }

    grid.into()
}

fn heatmap_cell(cell: JournalHeatCell, max_abs: f64, theme: &Theme) -> Element<'static, Message> {
    let base = if cell.count == 0 {
        journal_surface_sunken(theme)
    } else if cell.pnl >= 0.0 {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    let intensity = if cell.count == 0 || max_abs <= 0.0 {
        0.0
    } else {
        (cell.pnl.abs() / max_abs).clamp(0.0, 1.0) as f32
    };
    let fill = if cell.count == 0 {
        base
    } else {
        Color {
            a: 0.18 + 0.72 * intensity,
            ..base
        }
    };

    container(Space::new())
        .width(Length::Fixed(HEAT_CELL))
        .height(Length::Fixed(HEAT_CELL))
        .style(move |theme: &Theme| container_style::Style {
            background: Some(fill.into()),
            border: Border {
                color: crate::journal_views::style::journal_hairline(theme),
                width: 1.0,
                radius: 2.0.into(),
            },
            ..Default::default()
        })
        .into()
}
