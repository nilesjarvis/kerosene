use super::analytics::{
    JournalDirectionSplit, JournalKpis, JournalSegmentStats, journal_asset_pnls,
    journal_direction_split, journal_kpis, journal_time_of_day,
};
use crate::app_state::TradingTerminal;
use crate::journal::AggregatedTrade;
use crate::journal_views::style::{
    journal_accent_soft, journal_card_style, journal_dim, journal_muted, journal_rule_style,
    journal_segment_style, journal_surface_sunken,
};
use crate::message::Message;
use crate::portfolio_state::PortfolioWindow;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, button, canvas, column, container, row, rule, text};
use iced::{
    Alignment, Border, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme,
};

const COCKPIT_WINDOWS: [PortfolioWindow; 7] = [
    PortfolioWindow::Day,
    PortfolioWindow::Week,
    PortfolioWindow::Mtd,
    PortfolioWindow::Month,
    PortfolioWindow::Quarter,
    PortfolioWindow::Ytd,
    PortfolioWindow::AllTime,
];

const PANEL_TITLE_HEIGHT: f32 = 30.0;
const DONUT_SIZE: f32 = 150.0;
const HEAT_CELL: f32 = 26.0;
const MAX_ASSET_BARS: usize = 12;

impl TradingTerminal {
    pub(super) fn view_journal_cockpit<'a>(
        &'a self,
        filtered_trades: &[&'a AggregatedTrade],
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let include_fees = self.journal.include_fees_in_pnl;

        // The cockpit timeframe windows the analytics; the global KPI strip
        // above stays all-time.
        let cutoff = self
            .journal
            .portfolio_window
            .cutoff_ms(self.status_bar_now_ms);
        let windowed: Vec<&AggregatedTrade> = filtered_trades
            .iter()
            .copied()
            .filter(|trade| cutoff.is_none_or(|cutoff| trade.start_time >= cutoff))
            .collect();
        let kpis = journal_kpis(&windowed, include_fees);
        let split = journal_direction_split(&windowed, include_fees);
        let assets = journal_asset_pnls(&windowed, include_fees);
        let heatmap = journal_time_of_day(&windowed, include_fees);

        let header = row![
            text("Performance Overview")
                .size(20)
                .color(theme.palette().text),
            Space::new().width(16.0),
            cockpit_timeframe_row(self.journal.portfolio_window),
            Space::new().width(Fill),
            text("Select a trade for detail →")
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(journal_dim(&theme)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let equity = cockpit_panel(
            "EQUITY CURVE",
            self.view_journal_equity_panel_body(filtered_trades, &kpis, &theme),
            &theme,
        );

        let donut = cockpit_panel(
            "WIN / LOSS",
            self.view_journal_winloss_body(&kpis, &denomination, &theme),
            &theme,
        );

        let tiles = cockpit_panel(
            "KEY METRICS",
            view_journal_kpi_tiles(&kpis, &denomination, &theme),
            &theme,
        );

        let direction = cockpit_panel(
            "LONG vs SHORT vs SPOT",
            view_journal_direction_bars(&split, &denomination, &theme),
            &theme,
        );

        let heat = cockpit_panel(
            "EDGE BY TIME OF DAY (UTC)",
            view_journal_heatmap(&heatmap, &theme),
            &theme,
        );

        let asset_bars = cockpit_panel(
            "PNL BY ASSET",
            self.view_journal_asset_bars(&assets, &denomination, &theme),
            &theme,
        );

        let content = column![
            header,
            equity,
            row![donut, tiles].spacing(14),
            direction,
            heat,
            asset_bars,
        ]
        .spacing(14)
        .padding(16)
        .width(Fill);

        iced::widget::scrollable(content)
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .margin(0)
                    .scroller_width(4),
            ))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_journal_equity_panel_body<'a>(
        &'a self,
        filtered_trades: &[&'a AggregatedTrade],
        kpis: &JournalKpis,
        theme: &Theme,
    ) -> Element<'a, Message> {
        column![
            text("Cumulative realized PnL")
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(journal_muted(theme)),
            self.view_journal_summary_chart(
                filtered_trades,
                kpis.net_pnl,
                kpis.total_fees,
                kpis.win_rate,
                kpis.scored,
            ),
        ]
        .spacing(8)
        .into()
    }

    fn view_journal_winloss_body(
        &self,
        kpis: &JournalKpis,
        denomination: &crate::denomination::DisplayDenominationContext,
        theme: &Theme,
    ) -> Element<'_, Message> {
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

    fn view_journal_asset_bars<'a>(
        &'a self,
        assets: &[super::analytics::JournalAssetPnl],
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

// ---- Panel chrome ----

fn cockpit_panel<'a>(
    title: &'static str,
    body: Element<'a, Message>,
    theme: &Theme,
) -> Element<'a, Message> {
    container(
        column![
            container(
                text(title)
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_muted(theme)),
            )
            .height(Length::Fixed(PANEL_TITLE_HEIGHT))
            .padding([0, 12])
            .align_y(iced::alignment::Vertical::Center)
            .width(Fill),
            rule::horizontal(1).style(journal_rule_style),
            container(body).padding(12).width(Fill),
        ]
        .width(Fill),
    )
    .width(Fill)
    .style(journal_card_style)
    .into()
}

fn cockpit_timeframe_row(selected: PortfolioWindow) -> Element<'static, Message> {
    let mut row = row![].spacing(4).align_y(Alignment::Center);
    for window in COCKPIT_WINDOWS {
        row = row.push(
            button(
                text(window.label())
                    .size(10)
                    .font(crate::app_fonts::monospace_font()),
            )
            .on_press(Message::JournalPortfolioWindowChanged(window))
            .padding([3, 9])
            .style(journal_segment_style(selected == window)),
        );
    }
    row.into()
}

fn empty_note(message: &'static str, theme: &Theme) -> Element<'static, Message> {
    text(message)
        .size(11)
        .font(crate::app_fonts::monospace_font())
        .color(journal_muted(theme))
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

// ---- KPI tiles ----

fn view_journal_kpi_tiles(
    kpis: &JournalKpis,
    denomination: &crate::denomination::DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    let signed = |value: f64| crate::helpers::signed_number_color(value, theme);
    let text_color = theme.palette().text;

    let ratio = match (kpis.avg_win, kpis.avg_loss) {
        (Some(win), Some(loss)) if loss.abs() > 0.0 => format!("{:.2} : 1", win / loss.abs()),
        _ => "—".to_string(),
    };

    column![
        row![
            kpi_tile(
                "Expectancy / Trade",
                kpis.expectancy
                    .map(|value| denomination.format_signed_value(value, 2))
                    .unwrap_or_else(|| "—".to_string()),
                kpis.expectancy.map(signed).unwrap_or(text_color),
                "per scored trade",
                theme,
            ),
            kpi_tile(
                "Avg Win : Avg Loss",
                ratio,
                text_color,
                "reward / risk",
                theme,
            ),
        ]
        .spacing(12),
        row![
            kpi_tile(
                "Avg R Multiple",
                kpis.avg_r
                    .map(|value| format!("{value:+.2}R"))
                    .unwrap_or_else(|| "—".to_string()),
                kpis.avg_r.map(signed).unwrap_or(text_color),
                "vs avg loss",
                theme,
            ),
            kpi_tile(
                "Total Fees",
                denomination.format_value(kpis.total_fees, 2),
                theme.palette().warning,
                "all trades",
                theme,
            ),
        ]
        .spacing(12),
    ]
    .spacing(12)
    .into()
}

fn kpi_tile(
    label: &'static str,
    value: String,
    value_color: Color,
    caption: &'static str,
    theme: &Theme,
) -> Element<'static, Message> {
    container(
        column![
            text(label)
                .size(9)
                .font(crate::app_fonts::monospace_font())
                .color(journal_muted(theme)),
            text(value)
                .size(18)
                .font(crate::app_fonts::monospace_font())
                .color(value_color),
            text(caption)
                .size(9)
                .font(crate::app_fonts::monospace_font())
                .color(journal_dim(theme)),
        ]
        .spacing(4),
    )
    .width(Fill)
    .padding(10)
    .style(move |theme: &Theme| container_style::Style {
        background: Some(journal_surface_sunken(theme).into()),
        border: Border {
            color: crate::journal_views::style::journal_hairline(theme),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .into()
}

// ---- Long / Short / Spot bars ----

fn view_journal_direction_bars(
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

// ---- Time-of-day heatmap ----

fn view_journal_heatmap(
    heatmap: &super::analytics::JournalTimeOfDay,
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

fn heatmap_cell(
    cell: super::analytics::JournalHeatCell,
    max_abs: f64,
    theme: &Theme,
) -> Element<'static, Message> {
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

// ---- Win/Loss donut canvas ----

#[derive(Clone)]
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
