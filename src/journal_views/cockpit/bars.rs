use crate::app_state::TradingTerminal;
use crate::journal_views::analytics::{
    JournalAssetPnl, JournalDirectionSplit, JournalSegmentStats,
};
use crate::journal_views::style::{
    journal_accent_soft, journal_dim, journal_muted, journal_rule_style,
};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, column, container, row, rule, text};
use iced::{Alignment, Border, Color, Element, Fill, Length, Theme};

const MAX_ASSET_BARS: usize = 12;

impl TradingTerminal {
    pub(super) fn view_journal_asset_bars<'a>(
        &'a self,
        assets: &[JournalAssetPnl],
        denomination: &crate::denomination::DisplayDenominationContext,
        theme: &Theme,
    ) -> Element<'a, Message> {
        if assets.is_empty() {
            return empty_note("No asset PnL yet.", theme);
        }
        let max_abs = assets
            .iter()
            .map(|asset| asset.pnl.abs())
            .fold(0.0_f64, f64::max)
            .max(f64::EPSILON);

        let show_all = self.journal.show_all_assets;
        let limit = if show_all {
            assets.len()
        } else {
            MAX_ASSET_BARS
        };

        let mut list = Column::new().spacing(6);
        for asset in assets.iter().take(limit) {
            let positive = asset.pnl >= 0.0;
            let color = if positive {
                theme.palette().success
            } else {
                theme.palette().danger
            };
            let fraction = (asset.pnl.abs() / max_abs).clamp(0.0, 1.0) as f32;

            let left = if positive {
                empty_track()
            } else {
                row![spacer(1.0 - fraction), bar(fraction, color)]
                    .width(Fill)
                    .into()
            };
            let right = if positive {
                row![bar(fraction, color), spacer(1.0 - fraction)]
                    .width(Fill)
                    .into()
            } else {
                empty_track()
            };

            let label = self.display_coin_for_journal(&asset.coin);
            list = list.push(
                row![
                    text(label)
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .color(theme.palette().text)
                        .width(Length::Fixed(96.0)),
                    left,
                    container(rule::vertical(1).style(journal_rule_style))
                        .width(Length::Fixed(1.0))
                        .height(Length::Fixed(12.0)),
                    right,
                    text(denomination.format_signed_value(asset.pnl, 2))
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .color(color)
                        .width(Length::Fixed(96.0))
                        .align_x(iced::alignment::Horizontal::Right),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }

        if assets.len() > MAX_ASSET_BARS {
            let label = if show_all {
                format!("Show top {MAX_ASSET_BARS}")
            } else {
                format!("Show all {}", assets.len())
            };
            list = list.push(
                iced::widget::button(
                    text(label)
                        .size(10)
                        .font(crate::app_fonts::monospace_font()),
                )
                .on_press(Message::JournalToggleAllAssets)
                .padding([4, 10])
                .style(crate::journal_views::style::journal_ghost_button_style),
            );
        }

        list.into()
    }
}

// ---- Long / Short / Spot bars ----

pub(super) fn view_journal_direction_bars(
    split: &JournalDirectionSplit,
    denomination: &crate::denomination::DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let max_abs = [&split.long, &split.short, &split.spot]
        .iter()
        .map(|segment| segment.pnl.abs())
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);

    column![
        direction_row(
            "Long",
            &split.long,
            max_abs,
            theme.palette().success,
            denomination,
            theme
        ),
        direction_row(
            "Short",
            &split.short,
            max_abs,
            theme.palette().danger,
            denomination,
            theme
        ),
        direction_row(
            "Spot",
            &split.spot,
            max_abs,
            journal_accent_soft(theme),
            denomination,
            theme
        ),
    ]
    .spacing(12)
    .into()
}

fn direction_row(
    label: &'static str,
    segment: &JournalSegmentStats,
    max_abs: f64,
    bar_color: Color,
    denomination: &crate::denomination::DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let color = if segment.pnl >= 0.0 {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    let fraction = (segment.pnl.abs() / max_abs).clamp(0.0, 1.0) as f32;
    let win_rate = segment
        .win_rate()
        .map(|rate| format!("{rate:.0}% win"))
        .unwrap_or_else(|| "—".to_string());

    column![
        row![
            text(label)
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().text)
                .width(Length::Fixed(56.0)),
            row![bar(fraction, bar_color), spacer(1.0 - fraction)].width(Fill),
            text(denomination.format_signed_value(segment.pnl, 2))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(color)
                .width(Length::Fixed(96.0))
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(format!("{} trades · {}", segment.count, win_rate))
            .size(9)
            .font(crate::app_fonts::monospace_font())
            .color(journal_dim(theme)),
    ]
    .spacing(3)
    .into()
}

fn bar(fraction: f32, color: Color) -> Element<'static, Message> {
    let weight = (fraction * 1000.0).round().max(1.0) as u16;
    container(Space::new().height(Length::Fixed(8.0)))
        .width(Length::FillPortion(weight))
        .height(Length::Fixed(8.0))
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(color.into()),
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn spacer(fraction: f32) -> Element<'static, Message> {
    let weight = (fraction * 1000.0).round().max(1.0) as u16;
    Space::new().width(Length::FillPortion(weight)).into()
}

fn empty_track() -> Element<'static, Message> {
    Space::new().width(Fill).into()
}

fn empty_note(message: &'static str, theme: &Theme) -> Element<'static, Message> {
    text(message)
        .size(11)
        .font(crate::app_fonts::monospace_font())
        .color(journal_muted(theme))
        .into()
}
