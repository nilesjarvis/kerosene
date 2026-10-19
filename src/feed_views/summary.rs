use crate::message::Message;
use iced::widget::{Space, column, container, row, text};
use iced::{Color, Element, Fill, Length, Theme};

pub(crate) const FEED_SUMMARY_TIMEFRAMES: [(u64, &str); 4] =
    [(1, "1m"), (5, "5m"), (15, "15m"), (60, "1H")];

/// Shared timeframe totals, side labels, and proportion bars for feed footers.
pub(crate) fn view_feed_summary(
    theme: &Theme,
    timeframes: &[(u64, &'static str)],
    sides: [(&str, Color); 2],
    totals: impl Fn(u64) -> (f64, f64),
    format_value: impl Fn(f64) -> String,
) -> Element<'static, Message> {
    let muted = theme.extended_palette().background.weak.text;
    let mut summary = row![].spacing(16);
    for &(minutes, label) in timeframes {
        let (first, second) = totals(minutes);
        let total = first + second;
        let has_data = total > 0.0;
        let [(first_label, first_color), (second_label, second_color)] = sides;
        let (first_text, second_text, first_bar, second_bar) = if has_data {
            (first_color, second_color, first_color, second_color)
        } else {
            let gray_bar = theme.extended_palette().background.strong.color;
            (muted, muted, gray_bar, gray_bar)
        };
        let text_block = column![
            row![
                text(label).size(11).color(muted),
                Space::new().width(Fill),
                text(format_value(total)).size(11).color(if has_data {
                    theme.palette().text
                } else {
                    muted
                }),
            ]
            .width(Fill),
            row![
                text(format!("{first_label}: {}", format_value(first)))
                    .size(10)
                    .color(first_text),
                Space::new().width(Fill),
                text(format!("{second_label}: {}", format_value(second)))
                    .size(10)
                    .color(second_text),
            ]
            .width(Fill),
        ]
        .spacing(2)
        .width(Fill);

        let (first_ratio, second_ratio) = if has_data {
            ((first / total) as f32, (second / total) as f32)
        } else {
            (0.5, 0.5)
        };
        let bar = |ratio: f32, color: Color| {
            container(Space::new())
                .width(Length::FillPortion((ratio * 1000.0).max(1.0) as u16))
                .height(4)
                .style(move |_| container::Style {
                    background: Some(color.into()),
                    ..Default::default()
                })
        };
        let ratio_bar = row![bar(first_ratio, first_bar), bar(second_ratio, second_bar)]
            .width(Fill)
            .height(4);
        summary = summary.push(column![text_block, ratio_bar].spacing(4).width(Fill));
    }
    container(summary).width(Fill).into()
}
