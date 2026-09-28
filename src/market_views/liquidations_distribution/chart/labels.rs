use super::{PlotArea, nearest_distribution_point, point_x};
use crate::denomination::DisplayDenominationContext;
use crate::helpers;
use crate::liquidations_distribution_state::LiquidationDistributionData;
use iced::widget::canvas::{self, Frame, Stroke};
use iced::{Color, Point, Rectangle, Size, Theme, color};

pub(super) fn draw_axes(
    frame: &mut Frame,
    _data: &LiquidationDistributionData,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    plot: &PlotArea,
    max_bucket_usd: f64,
    max_cumulative_usd: f64,
) {
    let axis_color = Color {
        a: 0.42,
        ..theme.palette().text
    };
    let label_color = Color {
        a: 0.68,
        ..theme.palette().text
    };
    let base = canvas::Path::new(|builder| {
        builder.move_to(Point::new(plot.left, plot.top));
        builder.line_to(Point::new(plot.left, plot.bottom));
        builder.line_to(Point::new(plot.right, plot.bottom));
        builder.move_to(Point::new(plot.right, plot.top));
        builder.line_to(Point::new(plot.right, plot.bottom));
    });
    frame.stroke(
        &base,
        Stroke::default().with_color(axis_color).with_width(1.0),
    );

    for fraction in [0.0_f32, 0.5, 1.0] {
        let y = plot.bottom - plot.height * fraction;
        let bucket_value = max_bucket_usd * fraction as f64;
        let cumulative_value = max_cumulative_usd * fraction as f64;
        frame.fill_text(canvas::Text {
            content: compact_denomination_value(denomination, bucket_value),
            position: Point::new(plot.left - 6.0, y),
            color: label_color,
            size: iced::Pixels(10.0),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center,
            ..Default::default()
        });
        frame.fill_text(canvas::Text {
            content: compact_denomination_value(denomination, cumulative_value),
            position: Point::new(plot.right + 6.0, y),
            color: label_color,
            size: iced::Pixels(10.0),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center,
            ..Default::default()
        });
    }

    for fraction in [0.0_f64, 0.25, 0.5, 0.75, 1.0] {
        let price = plot.price_min + (plot.price_max - plot.price_min) * fraction;
        let x = plot.price_to_x(price);
        frame.fill_text(canvas::Text {
            content: denomination.format_price(price),
            position: Point::new(x, plot.bottom + 15.0),
            color: label_color,
            size: iced::Pixels(10.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center,
            ..Default::default()
        });
    }
}

pub(super) fn draw_current_mark(
    frame: &mut Frame,
    data: &LiquidationDistributionData,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    plot: &PlotArea,
) {
    if !plot.price_is_visible(data.request.mark) {
        return;
    }
    let x = plot.price_to_x(data.request.mark);
    let marker_color = theme.palette().primary;
    let mut stroke = Stroke::default().with_color(marker_color).with_width(1.5);
    stroke.line_dash = canvas::stroke::LineDash {
        segments: &[6.0, 5.0],
        offset: 0,
    };
    let line = canvas::Path::line(Point::new(x, plot.top), Point::new(x, plot.bottom));
    frame.stroke(&line, stroke);

    let price_label = denomination.format_price(data.request.mark);
    let compact = plot.width < 112.0;
    let label = if compact {
        price_label
    } else {
        format!("Current: {price_label}")
    };
    let estimated_width = label.chars().count() as f32 * 6.2 + 14.0;
    let width = estimated_width.min(plot.width.max(1.0)).max(1.0);
    let height = 20.0;
    let max_label_x = (plot.right - width).max(plot.left);
    let label_x = (x - width / 2.0).clamp(plot.left, max_label_x);
    let label_y = plot.bottom + 4.0;
    frame.fill_rectangle(
        Point::new(label_x, label_y),
        iced::Size::new(width, height),
        marker_color,
    );
    let label_border =
        canvas::Path::rectangle(Point::new(label_x, label_y), iced::Size::new(width, height));
    frame.stroke(
        &label_border,
        Stroke::default()
            .with_color(theme.extended_palette().background.strong.color)
            .with_width(1.0),
    );
    frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(label_x + width / 2.0, label_y + height / 2.0),
        color: theme.palette().background,
        size: iced::Pixels(10.0),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Center,
        ..Default::default()
    });
}

pub(super) struct HoverStateRenderContext<'a> {
    pub(super) frame: &'a mut Frame,
    pub(super) data: &'a LiquidationDistributionData,
    pub(super) denomination: &'a DisplayDenominationContext,
    pub(super) theme: &'a Theme,
    pub(super) bounds: Rectangle,
    pub(super) plot: &'a PlotArea,
    pub(super) cursor: iced::mouse::Cursor,
    pub(super) max_cumulative_usd: f64,
}

pub(super) fn draw_hover_state(ctx: HoverStateRenderContext<'_>) {
    let Some(cursor_pos) = ctx.cursor.position_in(ctx.bounds) else {
        return;
    };
    if cursor_pos.x < ctx.plot.left
        || cursor_pos.x > ctx.plot.right
        || cursor_pos.y < ctx.plot.top
        || cursor_pos.y > ctx.plot.bottom
    {
        return;
    }
    let Some(point) = nearest_distribution_point(ctx.data, ctx.plot, cursor_pos.x) else {
        return;
    };

    let x = point_x(point, ctx.plot);
    let guide = canvas::Path::line(Point::new(x, ctx.plot.top), Point::new(x, ctx.plot.bottom));
    ctx.frame.stroke(
        &guide,
        Stroke::default()
            .with_color(Color {
                a: 0.22,
                ..ctx.theme.palette().text
            })
            .with_width(1.0),
    );

    for (value, max_value, color) in [
        (
            point.cumulative_long_usd,
            ctx.max_cumulative_usd,
            color!(0xff7777),
        ),
        (
            point.cumulative_short_usd,
            ctx.max_cumulative_usd,
            color!(0x66d9a8),
        ),
    ] {
        if value > 0.0 {
            let marker =
                canvas::Path::circle(Point::new(x, ctx.plot.value_to_y(value, max_value)), 2.8);
            ctx.frame.fill(&marker, color);
        }
    }

    let tooltip_width = 170.0_f32.min((ctx.plot.width - 8.0).max(126.0));
    let tooltip_height = 68.0_f32;
    let max_x = (ctx.plot.right - tooltip_width).max(ctx.plot.left);
    let max_y = (ctx.plot.bottom - tooltip_height).max(ctx.plot.top);
    let tooltip_x = if cursor_pos.x + tooltip_width + 12.0 <= ctx.plot.right {
        cursor_pos.x + 10.0
    } else {
        cursor_pos.x - tooltip_width - 10.0
    }
    .clamp(ctx.plot.left, max_x);
    let tooltip_y = (cursor_pos.y - tooltip_height / 2.0).clamp(ctx.plot.top, max_y);
    let tooltip_origin = Point::new(tooltip_x, tooltip_y);

    ctx.frame.fill_rectangle(
        tooltip_origin,
        Size::new(tooltip_width, tooltip_height),
        Color {
            a: 0.94,
            ..ctx.theme.extended_palette().background.strong.color
        },
    );
    let border = canvas::Path::rectangle(tooltip_origin, Size::new(tooltip_width, tooltip_height));
    ctx.frame.stroke(
        &border,
        Stroke::default()
            .with_color(Color {
                a: 0.18,
                ..ctx.theme.palette().text
            })
            .with_width(1.0),
    );

    let tooltip_text = format!(
        "{}\nL {}  S {}\nCum L {}\nCum S {}",
        ctx.denomination.format_price(point.price),
        compact_denomination_value(ctx.denomination, point.long_usd),
        compact_denomination_value(ctx.denomination, point.short_usd),
        compact_denomination_value(ctx.denomination, point.cumulative_long_usd),
        compact_denomination_value(ctx.denomination, point.cumulative_short_usd),
    );
    ctx.frame.fill_text(canvas::Text {
        content: tooltip_text,
        position: Point::new(tooltip_x + 8.0, tooltip_y + 8.0),
        color: ctx.theme.palette().text,
        size: iced::Pixels(10.0),
        font: crate::app_fonts::monospace_font(),
        align_x: iced::alignment::Horizontal::Left.into(),
        align_y: iced::alignment::Vertical::Top,
        ..Default::default()
    });
}

fn compact_denomination_value(denomination: &DisplayDenominationContext, usd_value: f64) -> String {
    let Some(value) = denomination.convert_usd_value(usd_value) else {
        return helpers::invalid_data_placeholder();
    };
    let sign = if value < 0.0 { "-" } else { "" };
    let number = compact_number(value.abs());
    match denomination.active_code() {
        "USD" | "EUR" => format!("{sign}{}{number}", denomination.active_symbol()),
        code => format!("{sign}{number} {code}"),
    }
}

fn compact_number(value: f64) -> String {
    if value >= 1_000_000_000.0 {
        format!("{:.1}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.0}K", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}
