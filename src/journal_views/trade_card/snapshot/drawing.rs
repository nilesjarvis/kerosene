use super::interaction::{JournalSnapshotCanvasState, loaded_time_range};
use super::markers::{MarkerGroup, draw_markers};
use super::plot::{SnapshotPlot, point_in_snapshot_plot};
use crate::api::Candle;
use crate::helpers::format_price;
use crate::journal::JournalTradeSnapshot;
use iced::widget::canvas;
use iced::{Color, Point, Size, Theme, alignment};

pub(super) fn draw_snapshot_chart(
    frame: &mut canvas::Frame,
    theme: &Theme,
    snapshot: &JournalTradeSnapshot,
    state: &JournalSnapshotCanvasState,
    cursor: Option<Point>,
) -> Option<MarkerGroup> {
    let size = frame.size();
    if snapshot.candles.is_empty() || size.width <= 20.0 || size.height <= 20.0 {
        return None;
    }

    let loaded_range = loaded_time_range(snapshot);
    let (view_start_ms, view_end_ms) = state.view_or_full_range(snapshot, loaded_range);
    let candles = &snapshot.candles;
    let visible_candles = &candles[candles.partition_point(|c| c.close_time < view_start_ms)
        ..candles.partition_point(|c| c.open_time <= view_end_ms)];
    let markers = &snapshot.markers;
    let visible_markers = &markers[markers.partition_point(|m| m.time_ms < view_start_ms)
        ..markers.partition_point(|m| m.time_ms <= view_end_ms)];
    let scale_candles = if visible_candles.is_empty() {
        candles.as_slice()
    } else {
        visible_candles
    };
    let extra_prices = visible_markers.iter().map(|marker| marker.price).chain(
        snapshot
            .live_position
            .then_some(snapshot.metrics.entry_price),
    );
    let plot = SnapshotPlot::new(
        size,
        view_start_ms,
        view_end_ms,
        scale_candles,
        extra_prices,
    );
    let cursor = cursor.filter(|_| !state.is_dragging());
    let crosshair = cursor.filter(|pos| point_in_snapshot_plot(size, *pos));
    draw_grid(frame, theme, plot, crosshair);
    draw_candles(frame, theme, plot, visible_candles);
    if snapshot.live_position {
        draw_entry_line(frame, theme, plot, snapshot.metrics.entry_price);
    } else {
        draw_guides(
            frame,
            theme,
            plot,
            snapshot.trade_start_ms,
            snapshot.trade_end_ms,
            snapshot.is_open,
        );
    }
    if let Some(pos) = crosshair {
        draw_crosshair(frame, theme, plot, pos);
    }
    draw_markers(frame, theme, plot, visible_candles, visible_markers, cursor)
}

const PRICE_LABEL_HEIGHT: f32 = 18.0;

fn draw_grid(
    frame: &mut canvas::Frame,
    theme: &Theme,
    plot: SnapshotPlot,
    crosshair: Option<Point>,
) {
    for fraction in [0.25_f32, 0.5, 0.75] {
        let y = plot.top + plot.height * fraction;
        let path = canvas::Path::line(
            Point::new(plot.left, y),
            Point::new(plot.left + plot.width, y),
        );
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.08,
                    ..theme.palette().text
                })
                .with_width(1.0),
        );
        if crosshair.is_none_or(|pos| (pos.y - y).abs() >= PRICE_LABEL_HEIGHT) {
            draw_price_label(frame, theme, plot, y, false);
        }
    }
}

fn draw_crosshair(frame: &mut canvas::Frame, theme: &Theme, plot: SnapshotPlot, pos: Point) {
    let guides = canvas::Path::new(|path| {
        path.move_to(Point::new(pos.x, plot.top));
        path.line_to(Point::new(pos.x, plot.top + plot.height));
        path.move_to(Point::new(plot.left, pos.y));
        path.line_to(Point::new(plot.left + plot.width, pos.y));
    });
    frame.stroke(
        &guides,
        canvas::Stroke {
            line_dash: canvas::stroke::LineDash {
                segments: &[4.0, 3.0],
                offset: 0,
            },
            ..canvas::Stroke::default().with_color(Color {
                a: 0.55,
                ..theme.palette().text
            })
        },
    );
    draw_price_label(frame, theme, plot, pos.y, true);
}

fn draw_price_label(
    frame: &mut canvas::Frame,
    theme: &Theme,
    plot: SnapshotPlot,
    y: f32,
    hovered: bool,
) {
    let content = plot.price_label_at_y(y);
    let right = frame.width();
    let color = if hovered {
        let accent = theme.extended_palette().primary.base;
        let width = (right - (plot.left + plot.width))
            .max(content.len() as f32 * 6.6 + 10.0)
            .min(right);
        let badge = canvas::Path::rounded_rectangle(
            Point::new(right - width, y - PRICE_LABEL_HEIGHT * 0.5),
            Size::new(width, PRICE_LABEL_HEIGHT),
            3.0.into(),
        );
        frame.fill(&badge, accent.color);
        accent.text
    } else {
        Color {
            a: 0.65,
            ..theme.palette().text
        }
    };
    frame.fill_text(canvas::Text {
        content,
        position: Point::new(right - 5.0, y),
        color,
        size: iced::Pixels(11.0),
        align_x: alignment::Horizontal::Right.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

/// Horizontal entry-level guide for a live position (no opening fills, so the
/// vertical OPEN/CLOSE boundaries don't apply).
fn draw_entry_line(frame: &mut canvas::Frame, theme: &Theme, plot: SnapshotPlot, entry_price: f64) {
    if !entry_price.is_finite() || entry_price <= 0.0 {
        return;
    }

    let color = theme.palette().primary;
    let y = plot.y_for_price(entry_price);
    let path = canvas::Path::line(
        Point::new(plot.left, y),
        Point::new(plot.left + plot.width, y),
    );
    let mut stroke = canvas::Stroke::default()
        .with_color(Color { a: 0.7, ..color })
        .with_width(1.2);
    stroke.line_dash = canvas::stroke::LineDash {
        segments: &[5.0, 3.0],
        offset: 0,
    };
    frame.stroke(&path, stroke);

    let label = format!("ENTRY {}", format_price(entry_price));
    let label_width = (label.len() as f32 * 5.6 + 8.0).min(plot.width);
    let label_top = (y - 13.0).max(plot.top);
    frame.fill_rectangle(
        Point::new(plot.left, label_top),
        Size::new(label_width, 12.0),
        Color {
            a: 0.88,
            ..theme.extended_palette().background.strong.color
        },
    );
    frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(plot.left + 4.0, label_top + 1.0),
        color,
        size: iced::Pixels(9.0),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Top,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn draw_guides(
    frame: &mut canvas::Frame,
    theme: &Theme,
    plot: SnapshotPlot,
    trade_start_ms: u64,
    trade_end_ms: u64,
    is_open: bool,
) {
    draw_boundary_marker(
        frame,
        theme,
        plot,
        trade_start_ms,
        "OPEN",
        theme.palette().primary,
        &[5.0, 3.0],
    );
    draw_boundary_marker(
        frame,
        theme,
        plot,
        trade_end_ms,
        if is_open { "NOW" } else { "CLOSE" },
        theme.extended_palette().background.weak.text,
        &[2.0, 3.0],
    );
}

fn draw_boundary_marker(
    frame: &mut canvas::Frame,
    theme: &Theme,
    plot: SnapshotPlot,
    time_ms: u64,
    label: &'static str,
    color: Color,
    dash_segments: &'static [f32],
) {
    if time_ms < plot.start_ms || time_ms > plot.end_ms {
        return;
    }

    let x = plot.x_for_time(time_ms);
    let path = canvas::Path::line(
        Point::new(x, plot.top - 2.0),
        Point::new(x, plot.top + plot.height),
    );
    let mut stroke = canvas::Stroke::default()
        .with_color(Color { a: 0.62, ..color })
        .with_width(1.2);
    stroke.line_dash = canvas::stroke::LineDash {
        segments: dash_segments,
        offset: 0,
    };
    frame.stroke(&path, stroke);

    let label_width = if label == "CLOSE" { 38.0 } else { 30.0 };
    let label_x = (x - label_width / 2.0)
        .max(plot.left)
        .min(plot.left + plot.width - label_width);
    frame.fill_rectangle(
        Point::new(label_x, 1.0),
        Size::new(label_width, 13.0),
        Color {
            a: 0.88,
            ..theme.extended_palette().background.strong.color
        },
    );
    frame.fill_text(canvas::Text {
        content: label.to_string(),
        position: Point::new(label_x + label_width / 2.0, 3.0),
        color,
        size: iced::Pixels(9.0),
        align_x: alignment::Horizontal::Center.into(),
        align_y: alignment::Vertical::Top,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn draw_candles(frame: &mut canvas::Frame, theme: &Theme, plot: SnapshotPlot, candles: &[Candle]) {
    let candle_width = (plot.width / candles.len().max(1) as f32 * 0.58).clamp(2.0, 8.0);
    for candle in candles {
        let x = plot.x_for_time(candle.open_time);
        let open_y = plot.y_for_price(candle.open);
        let close_y = plot.y_for_price(candle.close);
        let high_y = plot.y_for_price(candle.high);
        let low_y = plot.y_for_price(candle.low);
        let color = Color {
            a: 0.82,
            ..if candle.close >= candle.open {
                theme.palette().success
            } else {
                theme.palette().danger
            }
        };

        let wick = canvas::Path::line(Point::new(x, high_y), Point::new(x, low_y));
        frame.stroke(
            &wick,
            canvas::Stroke::default().with_color(color).with_width(1.0),
        );

        frame.fill_rectangle(
            Point::new(x - candle_width / 2.0, open_y.min(close_y)),
            Size::new(candle_width, (open_y - close_y).abs().max(1.0)),
            color,
        );
    }
}
