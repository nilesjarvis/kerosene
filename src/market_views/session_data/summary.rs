use super::tooltips::tooltip_body;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::session_data_state::{
    SessionDataInstance, SessionStreak, SessionVerdict, average_abs_move_pct, current_streak,
    most_active_weekday, overall_win_rate_pct, total_return_pct,
};

use iced::widget::canvas::{self, Frame};
use iced::widget::{Space, canvas as canvas_widget, column, container, row, text, tooltip};
use iced::{
    Alignment, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme, mouse,
};

const KPI_WRAP_WIDTH: f32 = 520.0;
const VERDICT_TAIL_WIDTH: f32 = 300.0;

impl TradingTerminal {
    pub(super) fn view_session_data_summary(
        &self,
        instance: &SessionDataInstance,
        theme: &Theme,
        verdict: &SessionVerdict,
        available_width: f32,
    ) -> Element<'static, Message> {
        let verdict_line = view_verdict_line(verdict, theme, available_width);
        let kpis = view_kpi_strip(instance, theme, self.status_bar_now_ms, available_width);
        column![verdict_line, kpis].spacing(8).into()
    }
}

// ---------------------------------------------------------------------------
// Verdict line + KPI strip
// ---------------------------------------------------------------------------

fn view_verdict_line(
    verdict: &SessionVerdict,
    theme: &Theme,
    available_width: f32,
) -> Element<'static, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let accent = text("\u{258e}").size(14).color(theme.palette().primary);

    match verdict {
        SessionVerdict::Insufficient {
            total_samples,
            min_required,
        } => {
            let line = row![
                accent,
                text(format!(
                    "Not enough completed sessions to call a trend ({total_samples} so far, need {min_required}+ per bucket)"
                ))
                .size(11)
                .color(weak),
            ]
            .spacing(5)
            .align_y(Alignment::Center);
            wrap_verdict_tooltip(
                line,
                "A trend is only called once a weekday or session bucket has enough completed sessions to be meaningful. Widen the lookback to gather more.",
            )
        }
        SessionVerdict::Edge { strongest, weakest } => {
            let mut line = row![
                accent,
                text("Strongest").size(11).color(weak),
                text(strongest.label).size(11).color(theme.palette().text),
                text(helpers::format_signed_percent_value(
                    strongest.average_return_pct
                ))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(helpers::signed_number_color(
                    strongest.average_return_pct,
                    theme
                )),
                text("\u{00b7}").size(11).color(weak),
                text(format!("{:.0}% win", strongest.win_rate_pct))
                    .size(11)
                    .color(theme.palette().text),
                text("\u{00b7}").size(11).color(weak),
                text(format!("n{}", strongest.sample_count))
                    .size(10)
                    .color(weak),
            ]
            .spacing(5)
            .align_y(Alignment::Center);

            if available_width >= VERDICT_TAIL_WIDTH
                && let Some(weakest) = weakest
            {
                line = line
                    .push(Space::new().width(Fill))
                    .push(text("Weakest").size(11).color(weak))
                    .push(text(weakest.label).size(11).color(theme.palette().text))
                    .push(
                        text(helpers::format_signed_percent_value(
                            weakest.average_return_pct,
                        ))
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .color(helpers::signed_number_color(
                            weakest.average_return_pct,
                            theme,
                        )),
                    );
            }
            wrap_verdict_tooltip(
                line,
                "The strongest and weakest buckets by average open-to-close return, across both weekdays and market sessions. Only buckets with enough samples qualify; n is the sample count and win is the share that closed green.",
            )
        }
    }
}

fn wrap_verdict_tooltip<'a>(
    line: impl Into<Element<'a, Message>>,
    body: &'static str,
) -> Element<'a, Message> {
    container(tooltip(line, tooltip_body(body), tooltip::Position::Bottom))
        .width(Fill)
        .into()
}

fn view_kpi_strip(
    instance: &SessionDataInstance,
    theme: &Theme,
    now_ms: u64,
    available_width: f32,
) -> Element<'static, Message> {
    let weak = theme.extended_palette().background.weak.text;

    let win = overall_win_rate_pct(&instance.weekday_summaries);
    let avg_move = average_abs_move_pct(&instance.bars);
    let total = total_return_pct(&instance.bars);
    let streak = current_streak(&instance.bars);
    let active = most_active_weekday(&instance.bars);
    let freshness = instance
        .last_fetch_ms
        .map(|ms| format!("{} ago", helpers::format_relative_time(ms, now_ms)));

    let tiles: [Element<'static, Message>; 6] = [
        view_kpi_tile_winrate(
            win,
            theme,
            "Share of all completed sessions that closed in the green, over the selected lookback. The bar fills toward 100%, with a tick at 50%.",
        ),
        view_kpi_tile(
            "AVG MOVE",
            avg_move.map(|value| format!("{value:.2}%")),
            theme.palette().text,
            theme,
            "Average size of a session's open-to-close move, regardless of direction — a sense of typical volatility.",
        ),
        view_kpi_tile_signed(
            "TOTAL",
            total,
            theme,
            "Compounded return from holding through every completed session in the lookback.",
        ),
        view_kpi_tile_streak(
            streak,
            theme,
            "How many of the most recent sessions have run in the same direction (\u{25b2} up / \u{25bc} down).",
        ),
        view_kpi_tile(
            "BUSIEST",
            active.map(|weekday| weekday.label().to_string()),
            theme.palette().text,
            theme,
            "Weekday with the most traded volume over the lookback.",
        ),
        view_kpi_tile(
            "UPDATED",
            freshness,
            weak,
            theme,
            "How long ago this session history was last fetched.",
        ),
    ];

    if available_width >= KPI_WRAP_WIDTH {
        let mut strip = row![].spacing(6);
        for tile in tiles {
            strip = strip.push(tile);
        }
        strip.into()
    } else {
        let mut top = row![].spacing(6);
        let mut bottom = row![].spacing(6);
        for (idx, tile) in tiles.into_iter().enumerate() {
            if idx < 3 {
                top = top.push(tile);
            } else {
                bottom = bottom.push(tile);
            }
        }
        column![top, bottom].spacing(6).into()
    }
}

fn view_kpi_tile(
    label: &'static str,
    value: Option<String>,
    value_color: Color,
    theme: &Theme,
    tip: &'static str,
) -> Element<'static, Message> {
    let (value, value_color) = match value {
        Some(value) => (value, value_color),
        None => (
            "\u{2014}".to_string(),
            theme.extended_palette().background.weak.text,
        ),
    };
    let body = column![
        text(label)
            .size(9)
            .color(theme.extended_palette().background.weak.text),
        text(value)
            .size(13)
            .font(crate::app_fonts::monospace_font())
            .color(value_color),
    ]
    .spacing(2);
    let tile = container(body)
        .padding([4, 7])
        .width(Fill)
        .style(kpi_tile_style);
    kpi_tooltip(tile, tip)
}

fn view_kpi_tile_signed(
    label: &'static str,
    value: Option<f64>,
    theme: &Theme,
    tip: &'static str,
) -> Element<'static, Message> {
    match value {
        Some(value) => view_kpi_tile(
            label,
            Some(helpers::format_signed_percent_value(value)),
            helpers::signed_number_color(value, theme),
            theme,
            tip,
        ),
        None => view_kpi_tile(label, None, theme.palette().text, theme, tip),
    }
}

fn view_kpi_tile_streak(
    streak: Option<SessionStreak>,
    theme: &Theme,
    tip: &'static str,
) -> Element<'static, Message> {
    match streak {
        Some(streak) if streak.positive => view_kpi_tile(
            "STREAK",
            Some(format!("\u{25b2} {}", streak.length)),
            theme.palette().success,
            theme,
            tip,
        ),
        Some(streak) => view_kpi_tile(
            "STREAK",
            Some(format!("\u{25bc} {}", streak.length)),
            theme.palette().danger,
            theme,
            tip,
        ),
        None => view_kpi_tile("STREAK", None, theme.palette().text, theme, tip),
    }
}

fn view_kpi_tile_winrate(
    rate: Option<f64>,
    theme: &Theme,
    tip: &'static str,
) -> Element<'static, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let label = text("WIN RATE").size(9).color(weak);
    let value: Element<'static, Message> = match rate {
        Some(rate) => row![
            text(format!("{rate:.0}%"))
                .size(13)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().text),
            canvas_widget(WinRateBullet {
                ratio: (rate / 100.0) as f32,
            })
            .width(Length::Fixed(34.0))
            .height(Length::Fixed(8.0)),
        ]
        .spacing(5)
        .align_y(Alignment::Center)
        .into(),
        None => text("\u{2014}")
            .size(13)
            .font(crate::app_fonts::monospace_font())
            .color(weak)
            .into(),
    };
    let tile = container(column![label, value].spacing(2))
        .padding([4, 7])
        .width(Fill)
        .style(kpi_tile_style);
    kpi_tooltip(tile, tip)
}

fn kpi_tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    tip: &'static str,
) -> Element<'a, Message> {
    container(tooltip(content, tooltip_body(tip), tooltip::Position::Top))
        .width(Fill)
        .into()
}

fn kpi_tile_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            radius: 3.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Win-rate bullet (KPI tile)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct WinRateBullet {
    /// Win rate in 0..1.
    pub(super) ratio: f32,
}

impl canvas::Program<Message> for WinRateBullet {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }
        let track_h = (bounds.height * 0.5).clamp(3.0, 5.0);
        let y = (bounds.height - track_h) * 0.5;
        frame.fill_rectangle(
            Point::new(0.0, y),
            Size::new(bounds.width, track_h),
            Color {
                a: 0.9,
                ..theme.extended_palette().background.strong.color
            },
        );
        let fill_w = (self.ratio.clamp(0.0, 1.0) * bounds.width).max(0.0);
        frame.fill_rectangle(
            Point::new(0.0, y),
            Size::new(fill_w, track_h),
            theme.palette().primary,
        );
        let tick_x = bounds.width * 0.5;
        frame.fill_rectangle(
            Point::new(tick_x - 0.5, y - 1.0),
            Size::new(1.0, track_h + 2.0),
            Color {
                a: 0.6,
                ..theme.palette().text
            },
        );
        vec![frame.into_geometry()]
    }
}
