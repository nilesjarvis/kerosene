use super::countdown::next_candle_countdown_label;
use super::fisheye::ChartFisheye;
use super::model::CandlestickChart;
use super::order_cancel_hover::ease_out_cubic;
use super::state::{ChartState, HudMarketSide, HudOrderKind};
use super::tooltips::TooltipSurface;
use crate::chart::crosshair_style::{CrosshairStyleRender, draw_crosshair_style};
use crate::config::ChartCrosshairStyle;
use crate::helpers::format_price;
use iced::widget::canvas;
use iced::{Color, Point, Radians, Size, Theme, alignment};

mod measurement;
mod range;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Crosshair Overlay
// ---------------------------------------------------------------------------

const HUD_GREEN: Color = Color {
    r: 80.0 / 255.0,
    g: 250.0 / 255.0,
    b: 123.0 / 255.0,
    a: 0.88,
};
const HUD_SHADOW: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.58,
};
const HUD_WARNING_YELLOW: Color = Color {
    r: 1.0,
    g: 211.0 / 255.0,
    b: 67.0 / 255.0,
    a: 0.96,
};
const HUD_LINE_HEIGHT: f32 = 13.0;
const HUD_CHAR_WIDTH: f32 = 6.4;
const HUD_PANEL_PAD: f32 = 7.0;
const HUD_MARKET_TARGET_RADIUS: f32 = 11.5;
const HUD_MARKET_TARGET_LINE_GAP: f32 = HUD_MARKET_TARGET_RADIUS + 6.0;

pub(super) struct CrosshairOverlayContext<'a, PriceToY>
where
    PriceToY: Fn(f64) -> f32,
{
    pub(super) frame: &'a mut canvas::Frame,
    pub(super) state: &'a ChartState,
    pub(super) theme: &'a Theme,
    pub(super) chart_w: f32,
    pub(super) chart_h: f32,
    pub(super) funding_panel_h: f32,
    pub(super) price_h: f32,
    pub(super) price_hi: f64,
    pub(super) price_range: f64,
    pub(super) heatmap_stride: usize,
    pub(super) step: f32,
    pub(super) fisheye: ChartFisheye,
    pub(super) price_to_y: &'a PriceToY,
}

impl CandlestickChart {
    pub(super) fn draw_crosshair_overlay<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(data_pos) = ctx.state.cursor_position else {
            return;
        };
        let drawable_h = ctx.chart_h + ctx.funding_panel_h;
        if data_pos.x >= ctx.chart_w || data_pos.y >= drawable_h {
            return;
        }
        if hud_game_panels_visible(
            self.crosshair_style,
            ctx.state.cursor_position,
            ctx.chart_w,
            drawable_h,
        ) {
            self.draw_hud_game_panels(ctx);
        }
        let visual_pos = ctx.fisheye.project(data_pos);
        let hover_timestamp_ms = self.x_to_timestamp(data_pos.x, ctx.state, ctx.chart_w);
        let hud_accent = (self.crosshair_style.normalized() == ChartCrosshairStyle::Hud)
            .then(|| self.hud_accent_color(ctx.theme, ctx.state));
        let hud_cancel_hover_progress =
            hud_accent.map_or(0.0, |_| ease_out_cubic(self.order_cancel_hover_progress()));

        if let Some(accent) = hud_accent
            && hud_cancel_hover_progress > 0.01
        {
            draw_hud_cancel_collapsed_reticle(
                ctx.frame,
                visual_pos,
                ctx.fisheye,
                accent,
                hud_cancel_hover_progress,
            );
        } else if self.crosshair_style.normalized() == ChartCrosshairStyle::Hud
            && ctx.state.ctrl_down
        {
            draw_hud_size_scroller(
                ctx.frame,
                visual_pos,
                ctx.state,
                hud_accent.unwrap_or(HUD_GREEN),
            );
        } else {
            draw_crosshair_style(
                ctx.frame,
                ctx.theme,
                CrosshairStyleRender {
                    style: self.crosshair_style,
                    guide_lines_enabled: self.crosshair_guides_enabled,
                    crosshair_scale: self.crosshair_scale,
                    position: data_pos,
                    width: ctx.chart_w,
                    height: drawable_h,
                    fisheye: ctx.fisheye,
                    accent_color: hud_accent,
                },
            );
        }
        if let Some(accent) = hud_accent
            && hud_cancel_hover_progress <= 0.01
        {
            self.draw_hud_market_price_vector(ctx, visual_pos, accent);
            draw_hud_center_order_summary(ctx.frame, visual_pos, ctx.state, self.hud_armed, accent);
        }

        self.draw_crosshair_time_label(ctx, data_pos, visual_pos, drawable_h);

        if ctx.funding_panel_h > 0.0 && data_pos.y >= ctx.chart_h {
            let mut tooltip_surface =
                TooltipSurface::new(ctx.frame, ctx.theme, visual_pos, ctx.chart_w, ctx.price_h);
            tooltip_surface.draw_funding_hover(
                &self.funding_rates,
                ctx.chart_h,
                ctx.funding_panel_h,
                self.funding_annualized,
                |point| {
                    self.timestamp_to_x(point.time_ms, ctx.state, ctx.chart_w)
                        .map(|x| ctx.fisheye.project(Point::new(x, data_pos.y)).x)
                },
            );
            return;
        }

        if let Some(idx) = self.x_to_candle_index(data_pos.x, ctx.state, ctx.chart_w) {
            let volume = self.candles[idx].volume;
            ctx.frame.fill_text(canvas::Text {
                content: format!("Vol: {}", format_volume_compact(volume)),
                position: Point::new(6.0, ctx.price_h + 2.0),
                color: ctx.theme.palette().text,
                size: iced::Pixels(11.0),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Top,
                font: crate::app_fonts::monospace_font(),
                ..canvas::Text::default()
            });
        }

        if data_pos.y > ctx.price_h || ctx.price_range <= 0.0 {
            return;
        }

        let hover_price =
            self.y_to_price_with(data_pos.y, ctx.price_hi, ctx.price_range, ctx.price_h);

        self.draw_range_measurement(ctx, data_pos, visual_pos, hover_price);
        if hud_cancel_hover_progress <= 0.01 {
            self.draw_hud_crosshair_readout(
                ctx,
                data_pos,
                visual_pos,
                hover_price,
                hover_timestamp_ms,
            );
        }

        ctx.frame.fill_text(canvas::Text {
            content: format_price(hover_price),
            position: Point::new(ctx.chart_w + 6.0, visual_pos.y),
            color: self.crosshair_accent_text_color(ctx.theme, ctx.state, Color::WHITE),
            size: iced::Pixels(11.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });

        let mut tooltip_surface =
            TooltipSurface::new(ctx.frame, ctx.theme, visual_pos, ctx.chart_w, ctx.price_h);
        let projected_price_to_y = |price| {
            ctx.fisheye
                .project(Point::new(data_pos.x, (ctx.price_to_y)(price)))
                .y
        };
        tooltip_surface.draw_liquidation_hover(
            hover_price,
            ctx.price_range,
            &self.liquidation_buckets,
            &projected_price_to_y,
        );
        tooltip_surface.draw_heatmap_hover(
            &self.heatmap_rects,
            ctx.heatmap_stride,
            self.heatmap_max_usd,
            |rect| {
                self.heatmap_x_bounds(rect, ctx.state, ctx.chart_w, ctx.step)
                    .map(|(left, right)| {
                        (
                            ctx.fisheye.project(Point::new(left, data_pos.y)).x,
                            ctx.fisheye.project(Point::new(right, data_pos.y)).x,
                        )
                    })
            },
            &projected_price_to_y,
        );
    }

    fn draw_hud_game_panels<PriceToY>(&self, ctx: &mut CrosshairOverlayContext<'_, PriceToY>)
    where
        PriceToY: Fn(f64) -> f32,
    {
        let accent = self.hud_accent_color(ctx.theme, ctx.state);
        let size_panel = Size::new(172.0, 32.0);
        let size_origin = Point::new((ctx.chart_w - size_panel.width - 8.0).max(8.0), 8.0);
        let size_text = hud_size_panel_label(ctx.state);
        draw_hud_panel(
            ctx.frame,
            size_origin,
            size_panel,
            ctx.state.hud_size_editing,
            accent,
        );
        draw_hud_panel_text(
            ctx.frame,
            &size_text,
            Point::new(
                size_origin.x + HUD_PANEL_PAD,
                size_origin.y + size_panel.height * 0.5,
            ),
            ctx.state.hud_size_editing,
            accent,
        );

        let menu_size = Size::new(142.0, 138.0);
        let menu_origin = Point::new(
            (ctx.chart_w - menu_size.width - 8.0).max(8.0),
            (ctx.chart_h - menu_size.height - 8.0).max(46.0),
        );
        draw_hud_panel(ctx.frame, menu_origin, menu_size, true, accent);
        draw_hud_panel_text(
            ctx.frame,
            "GAME HUD",
            Point::new(menu_origin.x + HUD_PANEL_PAD, menu_origin.y + 13.0),
            true,
            accent,
        );
        draw_hud_order_row(
            ctx.frame,
            menu_origin,
            30.0,
            HudOrderKind::Limit,
            ctx.state.hud_order_kind,
            accent,
        );
        draw_hud_order_row(
            ctx.frame,
            menu_origin,
            47.0,
            HudOrderKind::Market,
            ctx.state.hud_order_kind,
            accent,
        );
        draw_hud_arm_row(ctx.frame, menu_origin, 64.0, self.hud_armed, accent);
        draw_hud_follow_row(
            ctx.frame,
            menu_origin,
            81.0,
            ctx.state.hud_follow_price,
            accent,
        );
        draw_hud_panel_text(
            ctx.frame,
            "MARKET SIDE",
            Point::new(menu_origin.x + HUD_PANEL_PAD, menu_origin.y + 97.0),
            ctx.state.hud_order_kind == HudOrderKind::Market,
            accent,
        );
        draw_hud_market_side_row(
            ctx.frame,
            menu_origin,
            111.0,
            HudMarketSide::Long,
            ctx.state.hud_market_side,
            ctx.state.hud_order_kind == HudOrderKind::Market,
            accent,
        );
        draw_hud_market_side_row(
            ctx.frame,
            menu_origin,
            125.0,
            HudMarketSide::Short,
            ctx.state.hud_market_side,
            ctx.state.hud_order_kind == HudOrderKind::Market,
            accent,
        );
    }

    fn draw_hud_crosshair_readout<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        data_pos: Point,
        visual_pos: Point,
        hover_price: f64,
        hover_timestamp_ms: Option<u64>,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        if self.crosshair_style.normalized() != ChartCrosshairStyle::Hud
            || ctx.chart_w < 180.0
            || ctx.price_h < 90.0
        {
            return;
        }

        let symbol = if self.symbol_label.trim().is_empty() {
            format!("CHART {}", self.id)
        } else {
            self.symbol_label.to_uppercase()
        };
        let candle_close_label = self
            .candles
            .last()
            .and_then(|candle| {
                next_candle_countdown_label(candle.open_time, self.timeframe, self.clock_now_ms)
            })
            .unwrap_or_else(|| "--".to_string());
        let hover_time_label = hover_timestamp_ms
            .map(format_hud_hover_time)
            .unwrap_or_else(|| "--".to_string());
        let accent = self.hud_accent_color(ctx.theme, ctx.state);

        let left_lines = [
            format!("{symbol} {}", self.timeframe.label()),
            format!("PX {}", format_price(hover_price)),
            format!("XY {:>5.1} {:>5.1}", data_pos.x, data_pos.y),
        ];
        let right_lines = [
            format!("T  {hover_time_label}"),
            format!("NOW {}", format_hud_clock_time(self.clock_now_ms)),
            format!("CLS {candle_close_label}"),
        ];

        let left_size = hud_text_block_size(&left_lines);
        let right_size = hud_text_block_size(&right_lines);
        let left_origin = hud_block_origin(
            visual_pos,
            -left_size.width - 42.0,
            -left_size.height - 26.0,
            left_size,
            ctx.chart_w,
            ctx.price_h,
        );
        let right_origin = hud_block_origin(
            visual_pos,
            42.0,
            -right_size.height - 26.0,
            right_size,
            ctx.chart_w,
            ctx.price_h,
        );

        draw_hud_connector(
            ctx.frame,
            visual_pos,
            Point::new(
                left_origin.x + left_size.width,
                left_origin.y + left_size.height,
            ),
            accent,
        );
        draw_hud_connector(
            ctx.frame,
            visual_pos,
            Point::new(right_origin.x, right_origin.y + right_size.height),
            accent,
        );
        draw_hud_text_block(ctx.frame, &left_lines, left_origin, accent);
        draw_hud_text_block(ctx.frame, &right_lines, right_origin, accent);
    }

    fn draw_crosshair_time_label<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        data_pos: Point,
        visual_pos: Point,
        drawable_h: f32,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(timestamp_ms) = self.x_to_timestamp(data_pos.x, ctx.state, ctx.chart_w) else {
            return;
        };

        let label = format_crosshair_relative_time(timestamp_ms, self.clock_now_ms);
        let label_width = label.len() as f32 * 6.5 + 12.0;
        let label_height = 17.0;
        if label_width + 8.0 > ctx.chart_w {
            return;
        }

        let label_x = (visual_pos.x - label_width * 0.5)
            .max(4.0)
            .min(ctx.chart_w - label_width - 4.0);
        let label_y = drawable_h + 3.0;

        ctx.frame.fill_rectangle(
            Point::new(label_x, label_y),
            Size::new(label_width, label_height),
            Color {
                a: 0.92,
                ..ctx.theme.extended_palette().background.strong.color
            },
        );
        ctx.frame.fill_text(canvas::Text {
            content: label,
            position: Point::new(label_x + label_width * 0.5, label_y + label_height * 0.5),
            color: self.crosshair_accent_text_color(ctx.theme, ctx.state, ctx.theme.palette().text),
            size: iced::Pixels(11.0),
            align_x: alignment::Horizontal::Center.into(),
            align_y: alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });
    }

    fn crosshair_accent_text_color(
        &self,
        theme: &Theme,
        state: &ChartState,
        fallback: Color,
    ) -> Color {
        if self.crosshair_style.normalized() == ChartCrosshairStyle::Hud {
            self.hud_accent_color(theme, state)
        } else {
            fallback
        }
    }

    fn hud_accent_color(&self, theme: &Theme, state: &ChartState) -> Color {
        match state.hud_order_kind {
            HudOrderKind::Limit => Color {
                a: 1.0,
                ..theme.palette().text
            },
            HudOrderKind::Market => {
                let color = match state.hud_market_side {
                    HudMarketSide::Long => theme.palette().success,
                    HudMarketSide::Short => theme.palette().danger,
                };
                Color { a: 1.0, ..color }
            }
        }
    }

    fn draw_hud_market_price_vector<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        center: Point,
        accent: Color,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.state.hud_order_kind != HudOrderKind::Market {
            return;
        }

        let Some(latest) = self.candles.last() else {
            return;
        };
        let Some(target_x) = self.timestamp_to_x(latest.open_time, ctx.state, ctx.chart_w) else {
            return;
        };
        let Some(reference_price) = self.market_reference_price else {
            return;
        };
        let target_y = (ctx.price_to_y)(reference_price);
        if !target_x.is_finite() || !target_y.is_finite() {
            return;
        }

        let target = ctx.fisheye.project(Point::new(
            target_x.clamp(0.0, ctx.chart_w),
            target_y.clamp(0.0, ctx.price_h),
        ));
        draw_hud_market_price_line(ctx.frame, center, target, accent);
    }
}

fn hud_game_panels_visible(
    style: ChartCrosshairStyle,
    cursor_position: Option<Point>,
    chart_w: f32,
    drawable_h: f32,
) -> bool {
    style.normalized() == ChartCrosshairStyle::Hud
        && chart_w > 0.0
        && drawable_h > 0.0
        && cursor_position.is_some_and(|pos| {
            pos.x >= 0.0 && pos.y >= 0.0 && pos.x < chart_w && pos.y < drawable_h
        })
}

fn hud_text_block_size(lines: &[String]) -> Size {
    let width = lines
        .iter()
        .map(|line| line.chars().count() as f32 * HUD_CHAR_WIDTH)
        .fold(0.0, f32::max);
    Size::new(width, lines.len() as f32 * HUD_LINE_HEIGHT)
}

fn hud_size_panel_label(state: &ChartState) -> String {
    let size = hud_display_size(state);
    if state.hud_size_editing {
        format!("S SIZE {size}_ COIN")
    } else {
        format!("S SIZE {size} COIN")
    }
}

fn hud_display_size(state: &ChartState) -> &str {
    let size = state.hud_size_input.trim();
    if size.is_empty() { "0" } else { size }
}

fn draw_hud_center_order_summary(
    frame: &mut canvas::Frame,
    center: Point,
    state: &ChartState,
    armed: bool,
    accent: Color,
) {
    let size = hud_display_size(state);
    let order_label = match state.hud_order_kind {
        HudOrderKind::Limit => "LIMIT".to_string(),
        HudOrderKind::Market => format!("MKT {}", state.hud_market_side.label()),
    };
    let amount_label = format!("{size} COIN");

    if armed {
        draw_hud_armed_warning_triangle(frame, Point::new(center.x, center.y - 52.0));
    }

    draw_hud_text(
        frame,
        &order_label,
        Point::new(center.x, center.y - 30.0),
        accent,
        alignment::Horizontal::Center,
    );
    draw_hud_text(
        frame,
        &amount_label,
        Point::new(center.x, center.y - 17.0),
        accent,
        alignment::Horizontal::Center,
    );
}

fn draw_hud_armed_warning_triangle(frame: &mut canvas::Frame, center: Point) {
    let half_width = 9.0;
    let height = 17.0;
    let top = Point::new(center.x, center.y - height * 0.5);
    let left = Point::new(center.x - half_width, center.y + height * 0.5);
    let right = Point::new(center.x + half_width, center.y + height * 0.5);

    let shadow = canvas::Path::new(|path| {
        path.move_to(Point::new(top.x + 1.0, top.y + 1.0));
        path.line_to(Point::new(right.x + 1.0, right.y + 1.0));
        path.line_to(Point::new(left.x + 1.0, left.y + 1.0));
        path.close();
    });
    frame.fill(&shadow, HUD_SHADOW);

    let triangle = canvas::Path::new(|path| {
        path.move_to(top);
        path.line_to(right);
        path.line_to(left);
        path.close();
    });
    frame.fill(&triangle, HUD_WARNING_YELLOW);
    frame.stroke(
        &triangle,
        canvas::Stroke::default()
            .with_color(Color {
                a: 0.92,
                ..HUD_WARNING_YELLOW
            })
            .with_width(1.0),
    );

    let stem = canvas::Path::line(
        Point::new(center.x, center.y - 1.8),
        Point::new(center.x, center.y + 4.0),
    );
    frame.stroke(
        &stem,
        canvas::Stroke::default()
            .with_color(Color::BLACK)
            .with_width(1.4)
            .with_line_cap(canvas::LineCap::Round),
    );
    let dot = canvas::Path::circle(Point::new(center.x, center.y + 7.0), 1.0);
    frame.fill(&dot, Color::BLACK);
}

fn draw_hud_cancel_collapsed_reticle(
    frame: &mut canvas::Frame,
    center: Point,
    fisheye: ChartFisheye,
    accent: Color,
    progress: f32,
) {
    let progress = progress.clamp(0.0, 1.0);
    let alpha = hud_lerp(0.78, 0.34, progress);
    let radius = hud_lerp(8.0, 2.8, progress);
    let gap = hud_lerp(13.0, 3.5, progress);
    let arm = hud_lerp(34.0, 6.0, progress);
    let stroke = canvas::Stroke::default()
        .with_color(Color { a: alpha, ..accent })
        .with_width(hud_lerp(1.15, 0.8, progress))
        .with_line_cap(canvas::LineCap::Round);

    fisheye.stroke_projected_circle(frame, center, radius, stroke);
    for (start, end) in [
        (
            Point::new(center.x - arm, center.y),
            Point::new(center.x - gap, center.y),
        ),
        (
            Point::new(center.x + gap, center.y),
            Point::new(center.x + arm, center.y),
        ),
        (
            Point::new(center.x, center.y - arm * 0.7),
            Point::new(center.x, center.y - gap),
        ),
        (
            Point::new(center.x, center.y + gap),
            Point::new(center.x, center.y + arm * 0.55),
        ),
    ] {
        fisheye.stroke_projected_line(frame, start, end, stroke);
    }

    fisheye.fill_projected_circle(
        frame,
        center,
        hud_lerp(1.45, 1.0, progress),
        Color { a: alpha, ..accent },
    );
}

fn draw_hud_market_price_line(
    frame: &mut canvas::Frame,
    center: Point,
    target: Point,
    accent: Color,
) {
    let dx = target.x - center.x;
    let dy = target.y - center.y;
    let distance = (dx * dx + dy * dy).sqrt();
    if !distance.is_finite() || distance < HUD_MARKET_TARGET_LINE_GAP {
        return;
    }
    let unit_x = dx / distance;
    let unit_y = dy / distance;
    let line_end = Point::new(
        target.x - unit_x * HUD_MARKET_TARGET_LINE_GAP,
        target.y - unit_y * HUD_MARKET_TARGET_LINE_GAP,
    );

    let shadow = canvas::Path::line(
        Point::new(center.x + 1.0, center.y + 1.0),
        Point::new(line_end.x + 1.0, line_end.y + 1.0),
    );
    frame.stroke(
        &shadow,
        canvas::Stroke::default()
            .with_color(HUD_SHADOW)
            .with_width(1.5)
            .with_line_cap(canvas::LineCap::Round),
    );

    let line = canvas::Path::line(center, line_end);
    frame.stroke(
        &line,
        canvas::Stroke::default()
            .with_color(Color { a: 0.72, ..accent })
            .with_width(1.0)
            .with_line_cap(canvas::LineCap::Round),
    );

    draw_hud_market_target_ring(frame, target, accent);
}

fn draw_hud_market_target_ring(frame: &mut canvas::Frame, center: Point, accent: Color) {
    let shadow_stroke = canvas::Stroke::default()
        .with_color(HUD_SHADOW)
        .with_width(2.4)
        .with_line_cap(canvas::LineCap::Round);
    let stroke = canvas::Stroke::default()
        .with_color(Color { a: 0.9, ..accent })
        .with_width(1.25)
        .with_line_cap(canvas::LineCap::Round);

    for (start, end) in [
        (-0.74, 0.74),
        (std::f32::consts::PI - 0.74, std::f32::consts::PI + 0.74),
    ] {
        stroke_hud_market_target_arc(
            frame,
            Point::new(center.x + 1.0, center.y + 1.0),
            HUD_MARKET_TARGET_RADIUS,
            start,
            end,
            shadow_stroke,
        );
        stroke_hud_market_target_arc(frame, center, HUD_MARKET_TARGET_RADIUS, start, end, stroke);
    }
}

fn stroke_hud_market_target_arc(
    frame: &mut canvas::Frame,
    center: Point,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    stroke: canvas::Stroke<'static>,
) {
    let arc = canvas::Path::new(|path| {
        path.arc(canvas::path::Arc {
            center,
            radius,
            start_angle: Radians(start_angle),
            end_angle: Radians(end_angle),
        });
    });
    frame.stroke(&arc, stroke);
}

fn hud_lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t.clamp(0.0, 1.0)
}

fn draw_hud_panel(
    frame: &mut canvas::Frame,
    origin: Point,
    size: Size,
    active: bool,
    accent: Color,
) {
    frame.fill_rectangle(
        origin,
        size,
        Color {
            a: if active { 0.46 } else { 0.34 },
            ..Color::BLACK
        },
    );
    let border = canvas::Path::rectangle(origin, size);
    frame.stroke(
        &border,
        canvas::Stroke::default()
            .with_color(Color {
                a: if active { 0.64 } else { 0.34 },
                ..accent
            })
            .with_width(if active { 1.0 } else { 0.7 }),
    );
}

fn draw_hud_panel_text(
    frame: &mut canvas::Frame,
    content: &str,
    position: Point,
    active: bool,
    accent: Color,
) {
    let color = Color {
        a: if active { 0.94 } else { 0.62 },
        ..accent
    };
    draw_hud_text(frame, content, position, color, alignment::Horizontal::Left);
}

fn draw_hud_order_row(
    frame: &mut canvas::Frame,
    menu_origin: Point,
    y_offset: f32,
    kind: HudOrderKind,
    active_kind: HudOrderKind,
    accent: Color,
) {
    let active = kind == active_kind;
    let row_origin = Point::new(menu_origin.x + 5.0, menu_origin.y + y_offset - 7.0);
    let row_size = Size::new(122.0, 14.0);
    if active {
        frame.fill_rectangle(row_origin, row_size, Color { a: 0.17, ..accent });
    }

    let hotkey = match kind {
        HudOrderKind::Limit => "L",
        HudOrderKind::Market => "M",
    };
    let marker = if active { ">" } else { " " };
    let label = format!("{marker} {hotkey} {}", kind.label());
    draw_hud_panel_text(
        frame,
        &label,
        Point::new(menu_origin.x + HUD_PANEL_PAD, menu_origin.y + y_offset),
        active,
        accent,
    );
}

fn draw_hud_arm_row(
    frame: &mut canvas::Frame,
    menu_origin: Point,
    y_offset: f32,
    armed: bool,
    accent: Color,
) {
    let row_origin = Point::new(menu_origin.x + 5.0, menu_origin.y + y_offset - 7.0);
    let row_size = Size::new(122.0, 14.0);
    if armed {
        frame.fill_rectangle(row_origin, row_size, Color { a: 0.17, ..accent });
    }

    let marker = if armed { ">" } else { " " };
    let label = if armed {
        format!("{marker} A ARMED")
    } else {
        format!("{marker} A SAFE")
    };
    draw_hud_panel_text(
        frame,
        &label,
        Point::new(menu_origin.x + HUD_PANEL_PAD, menu_origin.y + y_offset),
        armed,
        accent,
    );
}

fn draw_hud_follow_row(
    frame: &mut canvas::Frame,
    menu_origin: Point,
    y_offset: f32,
    following: bool,
    accent: Color,
) {
    let row_origin = Point::new(menu_origin.x + 5.0, menu_origin.y + y_offset - 7.0);
    let row_size = Size::new(122.0, 14.0);
    if following {
        frame.fill_rectangle(row_origin, row_size, Color { a: 0.17, ..accent });
    }

    let marker = if following { ">" } else { " " };
    let label = if following {
        format!("{marker} C FOLLOW")
    } else {
        format!("{marker} C MANUAL")
    };
    draw_hud_panel_text(
        frame,
        &label,
        Point::new(menu_origin.x + HUD_PANEL_PAD, menu_origin.y + y_offset),
        following,
        accent,
    );
}

fn draw_hud_market_side_row(
    frame: &mut canvas::Frame,
    menu_origin: Point,
    y_offset: f32,
    side: HudMarketSide,
    active_side: HudMarketSide,
    market_active: bool,
    accent: Color,
) {
    let active = market_active && side == active_side;
    let row_origin = Point::new(menu_origin.x + 5.0, menu_origin.y + y_offset - 7.0);
    let row_size = Size::new(132.0, 14.0);
    if active {
        frame.fill_rectangle(row_origin, row_size, Color { a: 0.17, ..accent });
    }

    let hotkey = match side {
        HudMarketSide::Long => "Y",
        HudMarketSide::Short => "X",
    };
    let marker = if active { ">" } else { " " };
    let label = format!("{marker} {hotkey} {}", side.label());
    draw_hud_panel_text(
        frame,
        &label,
        Point::new(menu_origin.x + HUD_PANEL_PAD, menu_origin.y + y_offset),
        market_active,
        accent,
    );
}

fn draw_hud_size_scroller(
    frame: &mut canvas::Frame,
    center: Point,
    state: &ChartState,
    accent: Color,
) {
    let radius = 35.0;
    let outer = canvas::Path::circle(center, radius);
    frame.stroke(
        &outer,
        canvas::Stroke::default()
            .with_color(Color { a: 0.84, ..accent })
            .with_width(1.25),
    );
    let inner = canvas::Path::circle(center, 7.0);
    frame.stroke(
        &inner,
        canvas::Stroke::default()
            .with_color(Color { a: 0.72, ..accent })
            .with_width(1.0),
    );

    for index in 0..12 {
        let angle = std::f32::consts::TAU * index as f32 / 12.0;
        let start = Point::new(center.x + angle.cos() * 26.0, center.y + angle.sin() * 26.0);
        let end = Point::new(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        );
        let tick = canvas::Path::line(start, end);
        frame.stroke(
            &tick,
            canvas::Stroke::default()
                .with_color(Color { a: 0.45, ..accent })
                .with_width(0.8),
        );
    }

    let bias = state.hud_size_scroll_bias.clamp(-1.0, 1.0);
    let needle_angle = -std::f32::consts::FRAC_PI_2 + bias * std::f32::consts::FRAC_PI_4;
    let needle_end = Point::new(
        center.x + needle_angle.cos() * 24.0,
        center.y + needle_angle.sin() * 24.0,
    );
    let needle = canvas::Path::line(center, needle_end);
    frame.stroke(
        &needle,
        canvas::Stroke::default()
            .with_color(accent)
            .with_width(1.3)
            .with_line_cap(canvas::LineCap::Round),
    );

    draw_hud_text(
        frame,
        "SIZE",
        Point::new(center.x, center.y - 52.0),
        accent,
        alignment::Horizontal::Center,
    );
    draw_hud_text(
        frame,
        hud_display_size(state),
        Point::new(center.x, center.y + 52.0),
        accent,
        alignment::Horizontal::Center,
    );
}

fn hud_block_origin(
    anchor: Point,
    dx: f32,
    dy: f32,
    size: Size,
    chart_w: f32,
    price_h: f32,
) -> Point {
    let max_x = (chart_w - size.width - 4.0).max(4.0);
    let max_y = (price_h - size.height - 4.0).max(4.0);
    Point::new(
        (anchor.x + dx).clamp(4.0, max_x),
        (anchor.y + dy).clamp(4.0, max_y),
    )
}

fn draw_hud_text(
    frame: &mut canvas::Frame,
    content: &str,
    position: Point,
    color: Color,
    align_x: alignment::Horizontal,
) {
    frame.fill_text(canvas::Text {
        content: content.to_string(),
        position: Point::new(position.x + 1.0, position.y + 1.0),
        color: HUD_SHADOW,
        size: iced::Pixels(10.5),
        align_x: align_x.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
    frame.fill_text(canvas::Text {
        content: content.to_string(),
        position,
        color,
        size: iced::Pixels(10.5),
        align_x: align_x.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn draw_hud_connector(frame: &mut canvas::Frame, from: Point, to: Point, accent: Color) {
    let path = canvas::Path::line(from, to);
    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_color(Color { a: 0.32, ..accent })
            .with_width(0.65),
    );
}

fn draw_hud_text_block(frame: &mut canvas::Frame, lines: &[String], origin: Point, accent: Color) {
    for (index, line) in lines.iter().enumerate() {
        let position = Point::new(
            origin.x,
            origin.y + index as f32 * HUD_LINE_HEIGHT + HUD_LINE_HEIGHT * 0.5,
        );
        draw_hud_text(frame, line, position, accent, alignment::Horizontal::Left);
    }
}

fn format_hud_hover_time(unix_ms: u64) -> String {
    format_hud_time(unix_ms, "%m/%d %H:%M:%S", "--")
}

fn format_hud_clock_time(unix_ms: u64) -> String {
    format_hud_time(unix_ms, "%H:%M:%S", "--:--:--")
}

fn format_hud_time(unix_ms: u64, format: &str, fallback: &str) -> String {
    let Ok(unix_ms) = i64::try_from(unix_ms) else {
        return fallback.to_string();
    };
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(unix_ms)
        .map(|dt| dt.with_timezone(&chrono::Local).format(format).to_string())
        .unwrap_or_else(|| fallback.to_string())
}

pub(super) fn format_crosshair_relative_time(timestamp_ms: u64, now_ms: u64) -> String {
    let (is_future, diff_ms) = if timestamp_ms > now_ms {
        (true, timestamp_ms - now_ms)
    } else {
        (false, now_ms - timestamp_ms)
    };

    let seconds = diff_ms / 1_000;
    if seconds < 5 {
        return "now".to_string();
    }

    let (value, unit) = if seconds < 60 {
        (seconds, "second")
    } else {
        let minutes = seconds / 60;
        if minutes < 60 {
            (minutes, "minute")
        } else {
            let hours = minutes / 60;
            if hours < 24 {
                (hours, "hour")
            } else {
                let days = hours / 24;
                if days < 14 {
                    (days, "day")
                } else {
                    let weeks = days / 7;
                    if weeks < 8 {
                        (weeks, "week")
                    } else {
                        let months = days / 30;
                        if months < 12 {
                            (months.max(1), "month")
                        } else {
                            ((days / 365).max(1), "year")
                        }
                    }
                }
            }
        }
    };

    let suffix = if value == 1 { "" } else { "s" };
    if is_future {
        format!("in {value} {unit}{suffix}")
    } else {
        format!("{value} {unit}{suffix} ago")
    }
}

pub(super) fn format_volume_compact(volume: f64) -> String {
    if !volume.is_finite() || volume <= 0.0 {
        return "0".to_string();
    }
    if volume >= 1_000_000_000.0 {
        format!("{:.2}B", volume / 1_000_000_000.0)
    } else if volume >= 1_000_000.0 {
        format!("{:.2}M", volume / 1_000_000.0)
    } else if volume >= 1_000.0 {
        format!("{:.1}K", volume / 1_000.0)
    } else if volume >= 1.0 {
        format!("{volume:.2}")
    } else {
        format!("{volume:.4}")
    }
}
