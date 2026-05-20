use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::format_decimal_with_commas;
use crate::journal::{AggregatedTrade, JournalFilter};
use crate::message::Message;

use chrono::{DateTime, Utc};
use iced::widget::container as container_style;
use iced::widget::{Space, canvas, checkbox, column, container, row, rule, text};
use iced::{Alignment, Color, Element, Fill, Font, Point, Rectangle, Renderer, Size, Theme};

// ---------------------------------------------------------------------------
// Journal Summary Chart
// ---------------------------------------------------------------------------

const FLAT_RANGE_EPSILON: f64 = 1e-9;
const DAY_MS: u64 = 24 * 60 * 60 * 1000;
const LEADING_ZERO_POINT_COUNT: usize = 4;
const MAX_LEADING_ZERO_WINDOW_MS: u64 = 7 * DAY_MS;
const Y_PADDING_RATIO: f64 = 0.10;
const RECENT_OUTCOME_TILE_LIMIT: usize = 56;
const OUTCOME_TILE_GAP: f32 = 4.0;
const OUTCOME_TILE_TOOLTIP_WIDTH: f32 = 128.0;
const OUTCOME_TILE_TOOLTIP_HEIGHT: f32 = 34.0;
const TOOLTIP_WIDTH: f32 = 210.0;
const TOOLTIP_HEIGHT: f32 = 46.0;

impl TradingTerminal {
    pub(super) fn view_journal_summary_chart(
        &self,
        filtered_trades: &[&AggregatedTrade],
        total_pnl: f64,
        total_fees: f64,
        win_rate: f64,
        total_closed: usize,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let pnl_points = journal_cumulative_pnl_points(filtered_trades);
        let account_value_points = self.journal_account_value_chart_points(&pnl_points);
        let show_account_value = self.journal.show_account_value_chart;
        let denomination = self.display_denomination_context();

        let value_color = signed_value_color(total_pnl, &theme);
        let muted = theme.extended_palette().background.weak.text;
        let filter_label = journal_filter_label(self.journal.filter);
        let win_rate_color = if total_closed == 0 {
            muted
        } else if win_rate >= 50.0 {
            theme.palette().success
        } else {
            theme.palette().danger
        };

        let chart_body: Element<'_, Message> = if pnl_points.len() >= 2 {
            canvas(JournalSummaryChart {
                pnl_points,
                account_value_points,
                show_account_value,
                denomination: denomination.clone(),
            })
            .width(Fill)
            .height(112)
            .into()
        } else {
            container(
                text("No PnL history available")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(112)
            .center(Fill)
            .into()
        };

        let account_toggle = checkbox(show_account_value)
            .label("Acct value")
            .on_toggle(Message::JournalToggleAccountValueChart)
            .size(10)
            .spacing(4)
            .text_size(10)
            .font(Font::MONOSPACE);

        let content = column![
            row![
                text("Performance").size(14).color(theme.palette().text),
                Space::new().width(Fill),
                text(filter_label).size(12).color(muted),
                account_toggle,
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            rule::horizontal(1),
            row![
                text(format_signed_display_full(total_pnl, &denomination))
                    .size(22)
                    .font(Font::MONOSPACE)
                    .color(value_color),
                text("PNL").size(13).font(Font::MONOSPACE).color(muted),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            chart_body,
            column![
                journal_outcome_strip(filtered_trades, denomination.clone()),
                column![
                    text(format!("{win_rate:.1}% Win Rate"))
                        .size(11)
                        .font(Font::MONOSPACE)
                        .color(win_rate_color),
                    text(trade_count_label(total_closed))
                        .size(11)
                        .font(Font::MONOSPACE)
                        .color(muted),
                    text(format!("Fees {}", denomination.format_value(total_fees, 2)))
                        .size(11)
                        .font(Font::MONOSPACE)
                        .color(theme.palette().danger),
                ]
                .spacing(2)
                .align_x(Alignment::Start),
            ]
            .spacing(3)
            .align_x(Alignment::Start),
        ]
        .spacing(7)
        .height(Fill);

        container(content)
            .padding([12, 16])
            .width(Fill)
            .height(320)
            .style(|theme: &Theme| summary_panel_style(theme))
            .into()
    }

    fn journal_account_value_chart_points(&self, pnl_points: &[(u64, f64)]) -> Vec<(u64, f64)> {
        if !self.journal.show_account_value_chart {
            return Vec::new();
        }

        let Some((start_ms, end_ms)) = chart_time_range(pnl_points) else {
            return Vec::new();
        };
        let bucket_key = match self.journal.filter {
            JournalFilter::Perp => "perpAllTime",
            JournalFilter::All | JournalFilter::Spot => "allTime",
        };

        self.portfolio_bucket_by_key(bucket_key)
            .or_else(|| self.portfolio_bucket_by_key("allTime"))
            .map(|bucket| {
                account_value_points_for_range(&bucket.account_value_history, start_ms, end_ms)
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JournalTradeOutcome {
    Win,
    Loss,
    Flat,
}

#[derive(Debug, Clone)]
struct JournalOutcomeTile {
    outcome: JournalTradeOutcome,
    pnl: f64,
    trade_type: String,
}

pub(super) fn journal_cumulative_pnl_points(trades: &[&AggregatedTrade]) -> Vec<(u64, f64)> {
    let mut trade_pnls = trades
        .iter()
        .filter_map(|trade| {
            let pnl = trade.pnl;
            pnl.is_finite()
                .then_some((trade.end_time.unwrap_or(trade.start_time), pnl))
        })
        .collect::<Vec<_>>();
    trade_pnls.sort_by_key(|(timestamp_ms, _)| *timestamp_ms);

    let Some(first_timestamp) = trade_pnls.first().map(|(timestamp_ms, _)| *timestamp_ms) else {
        return Vec::new();
    };
    let last_timestamp = trade_pnls
        .last()
        .map(|(timestamp_ms, _)| *timestamp_ms)
        .unwrap_or(first_timestamp);

    let mut points = journal_leading_zero_points(first_timestamp, last_timestamp);
    let mut cumulative_pnl = 0.0;
    let mut idx = 0;
    while idx < trade_pnls.len() {
        let timestamp_ms = trade_pnls[idx].0;
        while idx < trade_pnls.len() && trade_pnls[idx].0 == timestamp_ms {
            cumulative_pnl += trade_pnls[idx].1;
            idx += 1;
        }
        if let Some(last) = points.last_mut()
            && last.0 == timestamp_ms
        {
            last.1 = cumulative_pnl;
        } else {
            points.push((timestamp_ms, cumulative_pnl));
        }
    }

    points
}

fn journal_leading_zero_points(first_timestamp: u64, last_timestamp: u64) -> Vec<(u64, f64)> {
    let active_span = last_timestamp.saturating_sub(first_timestamp);
    let requested_span = if first_timestamp > DAY_MS {
        active_span
            .saturating_mul(2)
            .clamp(DAY_MS, MAX_LEADING_ZERO_WINDOW_MS)
    } else {
        active_span
            .saturating_mul(2)
            .max(LEADING_ZERO_POINT_COUNT as u64)
    };
    let baseline_span = requested_span.min(first_timestamp.saturating_sub(1));
    if baseline_span == 0 {
        return vec![(first_timestamp.saturating_sub(1), 0.0)];
    }

    let step = (baseline_span / LEADING_ZERO_POINT_COUNT as u64).max(1);
    let mut points = Vec::with_capacity(LEADING_ZERO_POINT_COUNT + 1);
    for idx in (1..=LEADING_ZERO_POINT_COUNT).rev() {
        let timestamp = first_timestamp.saturating_sub(step.saturating_mul(idx as u64));
        if timestamp < first_timestamp
            && points
                .last()
                .is_none_or(|(last_timestamp, _)| *last_timestamp < timestamp)
        {
            points.push((timestamp, 0.0));
        }
    }

    let anchor_timestamp = first_timestamp.saturating_sub(1);
    if points
        .last()
        .is_none_or(|(last_timestamp, _)| *last_timestamp < anchor_timestamp)
    {
        points.push((anchor_timestamp, 0.0));
    }

    points
}

pub(super) fn account_value_points_for_range(
    points: &[(u64, f64)],
    start_ms: u64,
    end_ms: u64,
) -> Vec<(u64, f64)> {
    if end_ms <= start_ms {
        return Vec::new();
    }

    let mut sorted = finite_sorted_points(points);
    if sorted.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    if let Some((_, value)) = sorted
        .iter()
        .rev()
        .find(|(timestamp_ms, _)| *timestamp_ms <= start_ms)
    {
        out.push((start_ms, *value));
    }

    out.extend(
        sorted
            .drain(..)
            .filter(|(timestamp_ms, _)| *timestamp_ms > start_ms && *timestamp_ms <= end_ms),
    );

    out
}

fn journal_outcome_strip(
    filtered_trades: &[&AggregatedTrade],
    denomination: DisplayDenominationContext,
) -> Element<'static, Message> {
    let tiles = journal_recent_trade_outcome_tiles(filtered_trades);
    if tiles.is_empty() {
        return container(text("No closed trades").size(10).font(Font::MONOSPACE))
            .height(42)
            .into();
    }

    canvas(JournalOutcomeStrip {
        tiles,
        denomination,
    })
    .width(Fill)
    .height(42)
    .into()
}

fn journal_recent_trade_outcome_tiles(trades: &[&AggregatedTrade]) -> Vec<JournalOutcomeTile> {
    let mut tiles = trades
        .iter()
        .filter(|trade| journal_trade_counts_toward_win_rate(trade))
        .map(|trade| {
            let outcome = if trade.pnl > 0.0 {
                JournalTradeOutcome::Win
            } else if trade.pnl < 0.0 {
                JournalTradeOutcome::Loss
            } else {
                JournalTradeOutcome::Flat
            };
            (
                trade.end_time.unwrap_or(trade.start_time),
                JournalOutcomeTile {
                    outcome,
                    pnl: trade.pnl,
                    trade_type: trade_type_label(trade),
                },
            )
        })
        .collect::<Vec<_>>();
    tiles.sort_by_key(|(timestamp_ms, _)| *timestamp_ms);

    let start = tiles.len().saturating_sub(RECENT_OUTCOME_TILE_LIMIT);
    tiles
        .into_iter()
        .skip(start)
        .map(|(_, tile)| tile)
        .collect()
}

fn journal_trade_counts_toward_win_rate(trade: &AggregatedTrade) -> bool {
    trade.status == "CLOSED"
        && !trade.coin.starts_with('@')
        && !trade.coin.starts_with('#')
        && trade.basis_complete
}

fn trade_type_label(trade: &AggregatedTrade) -> String {
    let side = if trade.is_long { "Long" } else { "Short" };
    format!("{side} {}", trade.coin)
}

fn journal_filter_label(filter: JournalFilter) -> &'static str {
    match filter {
        JournalFilter::All => "All",
        JournalFilter::Perp => "Perp",
        JournalFilter::Spot => "Spot",
    }
}

fn signed_value_color(value: f64, theme: &Theme) -> Color {
    if value > 0.0 {
        theme.palette().success
    } else if value < 0.0 {
        theme.palette().danger
    } else {
        theme.palette().text
    }
}

#[cfg(test)]
fn format_signed_usd_full(value: f64) -> String {
    format_signed_display_full(value, &DisplayDenominationContext::default())
}

fn format_signed_display_full(value: f64, denomination: &DisplayDenominationContext) -> String {
    let display_value = if value.abs() < 0.005 { 0.0 } else { value };
    if denomination.active_code() != "USD" {
        return denomination.format_signed_value(display_value, 2);
    }
    let sign = if display_value > 0.0 {
        "+"
    } else if display_value < 0.0 {
        "-"
    } else {
        ""
    };
    format!(
        "{sign}${}",
        format_decimal_with_commas(display_value.abs(), 2)
    )
}

fn trade_count_label(total_closed: usize) -> String {
    if total_closed == 1 {
        "1 Trade".to_string()
    } else {
        format!("{total_closed} Trades")
    }
}

fn summary_panel_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        border: iced::Border {
            color: theme.extended_palette().background.weak.color,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

#[derive(Debug, Clone)]
struct JournalSummaryChart {
    pnl_points: Vec<(u64, f64)>,
    account_value_points: Vec<(u64, f64)>,
    show_account_value: bool,
    denomination: DisplayDenominationContext,
}

#[derive(Debug, Clone)]
struct JournalOutcomeStrip {
    tiles: Vec<JournalOutcomeTile>,
    denomination: DisplayDenominationContext,
}

impl canvas::Program<Message> for JournalOutcomeStrip {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        draw_journal_outcome_strip(
            &self.tiles,
            &self.denomination,
            renderer,
            theme,
            bounds,
            cursor,
        )
    }
}

impl canvas::Program<Message> for JournalSummaryChart {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        draw_journal_summary_chart(self, renderer, theme, bounds, cursor)
    }
}

fn draw_journal_outcome_strip(
    tiles: &[JournalOutcomeTile],
    denomination: &DisplayDenominationContext,
    renderer: &Renderer,
    theme: &Theme,
    bounds: Rectangle,
    cursor: iced::mouse::Cursor,
) -> Vec<canvas::Geometry> {
    let mut frame = canvas::Frame::new(renderer, bounds.size());
    frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

    if tiles.is_empty() || bounds.width <= 0.0 || bounds.height <= 0.0 {
        return vec![frame.into_geometry()];
    }

    let tile_count = tiles.len() as f32;
    let gap = if tile_count > 1.0 {
        OUTCOME_TILE_GAP.min(((bounds.width / tile_count) * 0.3).max(1.0))
    } else {
        0.0
    };
    let total_gap = gap * (tile_count - 1.0).max(0.0);
    let tile_w = ((bounds.width - total_gap) / tile_count).clamp(5.0, 14.0);
    let tile_h = tile_w.clamp(5.0, 10.0);
    let start_x = 0.0;
    let tile_y = (bounds.height - tile_h - 2.0).max(0.0);
    let cursor_pos = cursor.position_in(bounds);
    let mut hovered_tile = None;
    let mut hovered_origin = Point::ORIGIN;

    for (idx, tile) in tiles.iter().enumerate() {
        let x = start_x + idx as f32 * (tile_w + gap);
        let tile_origin = Point::new(x, tile_y);
        let fill = outcome_color(tile.outcome, theme);
        frame.fill_rectangle(
            tile_origin,
            Size::new(tile_w, tile_h),
            Color { a: 0.86, ..fill },
        );
        let outline = canvas::Path::rectangle(tile_origin, Size::new(tile_w, tile_h));
        frame.stroke(
            &outline,
            canvas::Stroke::default()
                .with_color(Color { a: 0.40, ..fill })
                .with_width(1.0),
        );

        if let Some(pos) = cursor_pos
            && pos.x >= x
            && pos.x <= x + tile_w
            && pos.y >= tile_y
            && pos.y <= tile_y + tile_h
        {
            hovered_tile = Some(tile);
            hovered_origin = Point::new(x + tile_w / 2.0, tile_y + tile_h);
        }
    }

    if let Some(tile) = hovered_tile {
        let origin = tooltip_origin(
            hovered_origin,
            bounds.width,
            bounds.height,
            Size::new(OUTCOME_TILE_TOOLTIP_WIDTH, OUTCOME_TILE_TOOLTIP_HEIGHT),
        );
        frame.fill_rectangle(
            origin,
            Size::new(OUTCOME_TILE_TOOLTIP_WIDTH, OUTCOME_TILE_TOOLTIP_HEIGHT),
            Color {
                a: 0.94,
                ..theme.extended_palette().background.strong.color
            },
        );
        frame.fill_text(canvas::Text {
            content: format!(
                "{}\nPnL {}",
                tile.trade_type,
                denomination.format_signed_value(tile.pnl, 2)
            ),
            position: Point::new(origin.x + 7.0, origin.y + 9.0),
            color: theme.palette().text,
            size: iced::Pixels(9.0),
            font: Font::MONOSPACE,
            ..canvas::Text::default()
        });
    }

    vec![frame.into_geometry()]
}

fn outcome_color(outcome: JournalTradeOutcome, theme: &Theme) -> Color {
    match outcome {
        JournalTradeOutcome::Win => theme.palette().success,
        JournalTradeOutcome::Loss => theme.palette().danger,
        JournalTradeOutcome::Flat => theme.extended_palette().background.weak.text,
    }
}

#[derive(Debug, Clone, Copy)]
struct ChartPoint {
    point: Point,
    timestamp_ms: u64,
    value: f64,
}

#[derive(Debug, Clone)]
struct ChartLayout {
    pnl_points: Vec<ChartPoint>,
    account_value_points: Vec<ChartPoint>,
    zero_y: f32,
}

fn draw_journal_summary_chart(
    chart: &JournalSummaryChart,
    renderer: &Renderer,
    theme: &Theme,
    bounds: Rectangle,
    cursor: iced::mouse::Cursor,
) -> Vec<canvas::Geometry> {
    let mut frame = canvas::Frame::new(renderer, bounds.size());
    frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

    let Some(layout) = prepare_chart_layout(chart, bounds.width, bounds.height) else {
        return vec![frame.into_geometry()];
    };

    draw_grid(&mut frame, theme, bounds.size());
    draw_zero_line(&mut frame, theme, bounds.width, layout.zero_y);

    let pnl_color = match chart.pnl_points.last().map(|(_, value)| *value) {
        Some(value) if value < 0.0 => theme.palette().danger,
        _ => theme.palette().success,
    };
    draw_pnl_area(
        &mut frame,
        &layout.pnl_points,
        layout.zero_y,
        pnl_color,
        bounds.height,
    );
    draw_series(&mut frame, &layout.pnl_points, pnl_color, 2.0, &[]);

    if chart.show_account_value && !layout.account_value_points.is_empty() {
        draw_series(
            &mut frame,
            &layout.account_value_points,
            theme.palette().primary,
            1.5,
            &[5.0, 4.0],
        );
    }

    draw_hover_state(
        &mut frame,
        &layout,
        chart.show_account_value,
        &chart.denomination,
        theme,
        bounds,
        cursor,
    );

    vec![frame.into_geometry()]
}

fn prepare_chart_layout(
    chart: &JournalSummaryChart,
    width: f32,
    height: f32,
) -> Option<ChartLayout> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let pnl_points = finite_sorted_points(&chart.pnl_points);
    if pnl_points.len() < 2 {
        return None;
    }

    let (min_ts, max_ts) = chart_time_range(&pnl_points)?;
    if max_ts <= min_ts {
        return None;
    }

    let (pnl_lo, pnl_hi) = padded_value_range(&pnl_points, true)?;
    let pnl_plot_points =
        map_chart_points(&pnl_points, min_ts, max_ts, pnl_lo, pnl_hi, width, height);
    let zero_y = value_y(0.0, pnl_lo, pnl_hi, height).clamp(0.0, height);

    let account_value_points = if chart.show_account_value {
        let account_points = finite_sorted_points(&chart.account_value_points)
            .into_iter()
            .filter(|(timestamp_ms, _)| *timestamp_ms >= min_ts && *timestamp_ms <= max_ts)
            .collect::<Vec<_>>();
        if let Some((account_lo, account_hi)) = padded_value_range(&account_points, false) {
            map_chart_points(
                &account_points,
                min_ts,
                max_ts,
                account_lo,
                account_hi,
                width,
                height,
            )
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Some(ChartLayout {
        pnl_points: pnl_plot_points,
        account_value_points,
        zero_y,
    })
}

fn finite_sorted_points(points: &[(u64, f64)]) -> Vec<(u64, f64)> {
    let mut sorted = points
        .iter()
        .filter_map(|(timestamp_ms, value)| value.is_finite().then_some((*timestamp_ms, *value)))
        .collect::<Vec<_>>();
    sorted.sort_by_key(|(timestamp_ms, _)| *timestamp_ms);
    sorted
}

fn chart_time_range(points: &[(u64, f64)]) -> Option<(u64, u64)> {
    let start = points.first().map(|(timestamp_ms, _)| *timestamp_ms)?;
    let end = points.last().map(|(timestamp_ms, _)| *timestamp_ms)?;
    Some((start, end))
}

fn padded_value_range(points: &[(u64, f64)], include_zero: bool) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = points.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(lo, hi), (_, value)| (lo.min(*value), hi.max(*value)),
    );
    if !lo.is_finite() || !hi.is_finite() {
        return None;
    }

    if include_zero {
        lo = lo.min(0.0);
        hi = hi.max(0.0);
    }
    if (hi - lo).abs() < FLAT_RANGE_EPSILON {
        let pad = hi.abs().max(1.0) * 0.05;
        lo -= pad;
        hi += pad;
    }

    let pad = (hi - lo) * Y_PADDING_RATIO;
    Some((lo - pad, hi + pad))
}

fn map_chart_points(
    points: &[(u64, f64)],
    min_ts: u64,
    max_ts: u64,
    value_lo: f64,
    value_hi: f64,
    width: f32,
    height: f32,
) -> Vec<ChartPoint> {
    let ts_span = (max_ts - min_ts) as f64;
    points
        .iter()
        .map(|(timestamp_ms, value)| {
            let x = (((*timestamp_ms - min_ts) as f64 / ts_span) * f64::from(width)) as f32;
            ChartPoint {
                point: Point::new(x, value_y(*value, value_lo, value_hi, height)),
                timestamp_ms: *timestamp_ms,
                value: *value,
            }
        })
        .collect()
}

fn value_y(value: f64, value_lo: f64, value_hi: f64, height: f32) -> f32 {
    (((value_hi - value) / (value_hi - value_lo)) * f64::from(height)) as f32
}

fn draw_grid(frame: &mut canvas::Frame, theme: &Theme, size: Size) {
    for fraction in [0.25_f32, 0.5, 0.75] {
        let y = size.height * fraction;
        let path = canvas::Path::line(Point::new(0.0, y), Point::new(size.width, y));
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.08,
                    ..theme.palette().text
                })
                .with_width(1.0),
        );
    }
}

fn draw_zero_line(frame: &mut canvas::Frame, theme: &Theme, width: f32, zero_y: f32) {
    let path = canvas::Path::line(Point::new(0.0, zero_y), Point::new(width, zero_y));
    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_color(Color {
                a: 0.20,
                ..theme.palette().text
            })
            .with_width(1.0),
    );
}

fn draw_pnl_area(
    frame: &mut canvas::Frame,
    points: &[ChartPoint],
    zero_y: f32,
    color: Color,
    height: f32,
) {
    if points.len() < 2 {
        return;
    }

    for segment in points.windows(2) {
        let p1 = segment[0].point;
        let p2 = segment[1].point;
        let top = p1.y.min(p2.y).min(zero_y);
        let depth = (zero_y - top).abs();
        let alpha = (0.05 + (depth / height) * 0.14).clamp(0.05, 0.18);
        let fill = Color { a: alpha, ..color };
        let poly = canvas::Path::new(|builder| {
            builder.move_to(Point::new(p1.x, zero_y));
            builder.line_to(p1);
            builder.line_to(p2);
            builder.line_to(Point::new(p2.x, zero_y));
            builder.close();
        });
        frame.fill(&poly, fill);
    }
}

fn draw_series(
    frame: &mut canvas::Frame,
    points: &[ChartPoint],
    color: Color,
    width: f32,
    dash_segments: &'static [f32],
) {
    match points {
        [] => {}
        [only] => {
            let dot = canvas::Path::circle(only.point, 2.4);
            frame.fill(&dot, color);
        }
        points => {
            let mut path = canvas::path::Builder::new();
            for (idx, point) in points.iter().enumerate() {
                if idx == 0 {
                    path.move_to(point.point);
                } else {
                    path.line_to(point.point);
                }
            }

            let mut stroke = canvas::Stroke::default()
                .with_color(color)
                .with_width(width);
            if !dash_segments.is_empty() {
                stroke.line_dash = canvas::stroke::LineDash {
                    segments: dash_segments,
                    offset: 0,
                };
            }
            frame.stroke(&path.build(), stroke);
        }
    }
}

fn draw_hover_state(
    frame: &mut canvas::Frame,
    layout: &ChartLayout,
    show_account_value: bool,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    bounds: Rectangle,
    cursor: iced::mouse::Cursor,
) {
    let Some(cursor_pos) = cursor.position_in(bounds) else {
        return;
    };
    if cursor_pos.x < 0.0
        || cursor_pos.x > bounds.width
        || cursor_pos.y < 0.0
        || cursor_pos.y > bounds.height
    {
        return;
    }

    let Some(nearest_pnl) = nearest_chart_point(&layout.pnl_points, cursor_pos.x) else {
        return;
    };

    let v_line = canvas::Path::line(
        Point::new(nearest_pnl.point.x, 0.0),
        Point::new(nearest_pnl.point.x, bounds.height),
    );
    frame.stroke(
        &v_line,
        canvas::Stroke::default()
            .with_color(Color {
                a: 0.20,
                ..theme.palette().text
            })
            .with_width(1.0),
    );
    frame.fill(&canvas::Path::circle(nearest_pnl.point, 2.8), Color::WHITE);

    let mut lines = vec![
        format_timestamp(nearest_pnl.timestamp_ms),
        format!(
            "PnL {}",
            denomination.format_signed_value(nearest_pnl.value, 2)
        ),
    ];

    if show_account_value
        && let Some(nearest_account) =
            nearest_chart_point(&layout.account_value_points, cursor_pos.x)
    {
        frame.fill(
            &canvas::Path::circle(nearest_account.point, 2.4),
            theme.palette().primary,
        );
        lines.push(format!(
            "Acct {}",
            denomination.format_value(nearest_account.value, 2)
        ));
    }

    let tooltip_height = if lines.len() > 2 {
        TOOLTIP_HEIGHT
    } else {
        TOOLTIP_HEIGHT - 12.0
    };
    let origin = tooltip_origin(
        nearest_pnl.point,
        bounds.width,
        bounds.height,
        Size::new(TOOLTIP_WIDTH, tooltip_height),
    );
    frame.fill_rectangle(
        origin,
        Size::new(TOOLTIP_WIDTH, tooltip_height),
        Color {
            a: 0.93,
            ..theme.extended_palette().background.strong.color
        },
    );
    frame.fill_text(canvas::Text {
        content: lines.join("\n"),
        position: Point::new(origin.x + 7.0, origin.y + 9.0),
        color: theme.palette().text,
        size: iced::Pixels(10.0),
        font: Font::MONOSPACE,
        ..canvas::Text::default()
    });
}

fn nearest_chart_point(points: &[ChartPoint], cursor_x: f32) -> Option<ChartPoint> {
    points.iter().copied().min_by(|left, right| {
        let left_dist = (left.point.x - cursor_x).abs();
        let right_dist = (right.point.x - cursor_x).abs();
        left_dist.total_cmp(&right_dist)
    })
}

fn tooltip_origin(point: Point, width: f32, height: f32, tooltip_size: Size) -> Point {
    let x = if point.x + tooltip_size.width + 10.0 > width {
        point.x - tooltip_size.width - 8.0
    } else {
        point.x + 8.0
    }
    .clamp(0.0, (width - tooltip_size.width).max(0.0));

    let y = if point.y + tooltip_size.height + 10.0 > height {
        point.y - tooltip_size.height - 8.0
    } else {
        point.y + 8.0
    }
    .clamp(0.0, (height - tooltip_size.height).max(0.0));

    Point::new(x, y)
}

fn format_timestamp(timestamp_ms: u64) -> String {
    i64::try_from(timestamp_ms)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "UTC time unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        JournalTradeOutcome, account_value_points_for_range, format_signed_usd_full,
        journal_cumulative_pnl_points, journal_recent_trade_outcome_tiles,
    };
    use crate::journal::AggregatedTrade;

    fn trade(start_time: u64, end_time: Option<u64>, pnl: f64) -> AggregatedTrade {
        AggregatedTrade {
            id: format!("trade-{start_time}-{pnl}"),
            legacy_note_ids: Vec::new(),
            coin: "BTC".to_string(),
            start_time,
            end_time,
            max_position: 1.0,
            volume: 100.0,
            fee: 1.0,
            pnl,
            status: "CLOSED".to_string(),
            fill_count: 2,
            avg_entry_price: 100.0,
            total_entry_notional: 100.0,
            total_entry_size: 1.0,
            is_long: true,
            basis_complete: true,
        }
    }

    #[test]
    fn cumulative_pnl_points_sort_and_coalesce_trade_times() {
        let first = trade(1_000, Some(3_000), 10.0);
        let second = trade(2_000, Some(2_000), -4.0);
        let third = trade(4_000, Some(3_000), 2.5);
        let trades = vec![&first, &second, &third];

        let points = journal_cumulative_pnl_points(&trades);
        assert!(points.len() > 3);
        assert!(points.windows(2).all(|window| window[0].0 < window[1].0));
        assert!(
            points[..points.len() - 2]
                .iter()
                .all(|(_, pnl)| *pnl == 0.0)
        );
        assert!(points.ends_with(&[(2_000, -4.0), (3_000, 8.5)]));
    }

    #[test]
    fn account_value_points_are_clamped_to_pnl_time_range() {
        let points = vec![
            (1_000, 90.0),
            (2_000, 100.0),
            (3_000, 110.0),
            (4_000, 120.0),
        ];

        assert_eq!(
            account_value_points_for_range(&points, 2_500, 3_500),
            vec![(2_500, 100.0), (3_000, 110.0)]
        );
    }

    #[test]
    fn signed_usd_full_keeps_large_values_expanded() {
        assert_eq!(format_signed_usd_full(29_425_659.43), "+$29,425,659.43");
        assert_eq!(format_signed_usd_full(-42.5), "-$42.50");
        assert_eq!(format_signed_usd_full(0.001), "$0.00");
    }

    #[test]
    fn recent_trade_outcomes_sort_filter_and_cap_results() {
        let ignored_open = AggregatedTrade {
            status: "OPEN".to_string(),
            ..trade(500, Some(500), 50.0)
        };
        let first = trade(1_000, Some(1_000), 1.0);
        let second = trade(2_000, Some(2_000), -1.0);
        let third = trade(3_000, Some(3_000), 0.0);
        let trades = vec![&third, &ignored_open, &second, &first];

        let outcomes = journal_recent_trade_outcome_tiles(&trades)
            .into_iter()
            .map(|tile| (tile.outcome, tile.trade_type, tile.pnl))
            .collect::<Vec<_>>();

        assert_eq!(
            outcomes,
            vec![
                (JournalTradeOutcome::Win, "Long BTC".to_string(), 1.0),
                (JournalTradeOutcome::Loss, "Long BTC".to_string(), -1.0),
                (JournalTradeOutcome::Flat, "Long BTC".to_string(), 0.0),
            ]
        );
    }
}
