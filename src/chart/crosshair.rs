use super::candle_layer::{EARNINGS_DOT_RADIUS, earnings_marker_dot_y};
use super::drawing::{AxisBadgeStyle, fill_right_axis_badge};
use super::fisheye::ChartFisheye;
use super::model::{CandlestickChart, EarningsMarker};
use super::price_badges::RIGHT_AXIS_PRIMARY_BADGE_HEIGHT;
use super::state::{ChartState, HudMarketSide, HudOrderKind};
use super::tooltips::{TooltipLine, TooltipSurface};
use crate::chart::crosshair_style::{CrosshairStyleRender, RacingHudMetrics, draw_crosshair_style};
use crate::config::{ChartCrosshairStyle, ChartHudReadoutConfig};
use crate::helpers::{ease_out_cubic, format_price};
use iced::widget::canvas;
use iced::{Color, Point, Size, Theme, alignment};

mod game_hud;
mod measurement;
mod range;

use game_hud::{draw_hud_text_sized, fill_chevron_right, fill_triangle, hud_pulse_wave};
pub(in crate::chart) use game_hud::{hud_selector_bounds, hud_station_metrics};

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
const HUD_MARKET_TARGET_RADIUS: f32 = 11.5;
const HUD_MARKET_TARGET_LINE_GAP: f32 = HUD_MARKET_TARGET_RADIUS + 6.0;
const HUD_JET_TAPE_GAP: f32 = 46.0;
const HUD_JET_TAPE_TEXT_SIZE: f32 = 11.0;
const EARNINGS_HOVER_RADIUS: f32 = 12.5;
const EARNINGS_TOOLTIP_SUMMARY_MAX_CHARS: usize = 54;

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
    pub(super) session_panel_h: f32,
    pub(super) price_h: f32,
    pub(super) price_hi: f64,
    pub(super) price_range: f64,
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
        // The armed combat frame is safety chrome: it stays on screen with or
        // without a cursor so a hot chart is never mistaken for a safe one.
        if self.crosshair_style.is_game_hud() && self.hud_armed {
            self.draw_hud_armed_frame(ctx.frame, ctx.chart_w, ctx.price_h);
        }

        let Some(data_pos) = ctx.state.cursor_position else {
            return;
        };
        let drawable_h = ctx.chart_h + ctx.funding_panel_h + ctx.session_panel_h;
        if data_pos.x >= ctx.chart_w || data_pos.y >= drawable_h {
            return;
        }
        if self.earnings_marker_hover_overlay_active(ctx.state, ctx.chart_w) {
            self.draw_earnings_marker_hover(ctx);
            return;
        }
        if hud_game_panels_visible(
            self.crosshair_style,
            ctx.state.cursor_position,
            ctx.chart_w,
            drawable_h,
        ) {
            let accent = self.hud_accent_color(ctx.theme, ctx.state);
            self.draw_hud_game_chrome(ctx, accent);
        }
        let visual_pos = ctx.fisheye.project(data_pos);
        let hover_timestamp_ms = self.x_to_timestamp(data_pos.x, ctx.state, ctx.chart_w);
        let hover_price = (data_pos.y <= ctx.price_h && ctx.price_range > 0.0)
            .then(|| self.y_to_price_with(data_pos.y, ctx.price_hi, ctx.price_range, ctx.price_h))
            .filter(|price| price.is_finite() && *price > 0.0);
        let hud_accent = self
            .crosshair_style
            .is_game_hud()
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
        } else if self.crosshair_style.is_game_hud() && ctx.state.ctrl_down {
            draw_hud_size_scroller(
                ctx.frame,
                visual_pos,
                ctx.state,
                hud_accent.unwrap_or(HUD_GREEN),
            );
        } else {
            // The reticle breathes on the shared armed pulse so the cursor and
            // the combat frame read as one instrument.
            let mut crosshair_scale = self.crosshair_scale;
            if self.hud_armed && self.crosshair_style.normalized() == ChartCrosshairStyle::Hud {
                crosshair_scale *= 1.0 + 0.05 * hud_pulse_wave(self.hud_pulse_phase);
            }
            draw_crosshair_style(
                ctx.frame,
                ctx.theme,
                CrosshairStyleRender {
                    style: self.crosshair_style,
                    guide_lines_enabled: self.crosshair_guides_enabled,
                    crosshair_scale,
                    position: data_pos,
                    width: ctx.chart_w,
                    height: drawable_h,
                    fisheye: ctx.fisheye,
                    accent_color: hud_accent,
                    racing_hud_metrics: self.racing_hud_metrics(ctx.state, hover_price),
                },
            );
        }
        if let Some(accent) = hud_accent
            && hud_cancel_hover_progress <= 0.01
        {
            self.draw_hud_market_price_vector(ctx, visual_pos, accent);
            self.draw_hud_order_summary(ctx, visual_pos, accent, hover_price);
        }

        self.draw_crosshair_time_label(ctx, data_pos, visual_pos, drawable_h);

        if ctx.funding_panel_h > 0.0
            && data_pos.y >= ctx.chart_h
            && data_pos.y < ctx.chart_h + ctx.funding_panel_h
        {
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

        if data_pos.y >= ctx.chart_h {
            return;
        }

        if let Some(idx) = self.x_to_candle_index(data_pos.x, ctx.state, ctx.chart_w) {
            let volume = self.candles[idx].volume;
            ctx.frame.fill_text(canvas::Text {
                content: format!(
                    "Vol: {}",
                    format_volume_readout(volume, self.whole_unit_volume)
                ),
                position: Point::new(6.0, ctx.price_h + 2.0),
                color: ctx.theme.palette().text,
                size: iced::Pixels(11.0),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Top,
                font: crate::app_fonts::monospace_font(),
                ..canvas::Text::default()
            });
        }

        let Some(hover_price) = hover_price else {
            self.draw_earnings_marker_hover(ctx);
            return;
        };

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

        fill_right_axis_badge(
            ctx.frame,
            ctx.chart_w,
            visual_pos.y,
            format_price(hover_price),
            ctx.theme.extended_palette().background.strong.color,
            AxisBadgeStyle {
                char_width: 6.5,
                padding_width: 8.0,
                height: RIGHT_AXIS_PRIMARY_BADGE_HEIGHT,
                text_size: 11.0,
                text_color: self.crosshair_accent_text_color(
                    ctx.theme,
                    ctx.state,
                    ctx.theme.palette().text,
                ),
            },
        );

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
        self.draw_earnings_marker_hover(ctx);
    }

    fn earnings_marker_hover_overlay_active(&self, state: &ChartState, chart_w: f32) -> bool {
        let Some(time_ms) = self.hover_earnings_marker_time_ms else {
            return false;
        };
        self.earnings_markers
            .iter()
            .any(|marker| marker.time_ms == time_ms)
            && self
                .timestamp_to_x(time_ms, state, chart_w)
                .is_some_and(|x| x.is_finite() && x >= 0.0 && x <= chart_w)
    }

    fn draw_earnings_marker_hover<PriceToY>(&self, ctx: &mut CrosshairOverlayContext<'_, PriceToY>)
    where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(time_ms) = self.hover_earnings_marker_time_ms else {
            return;
        };
        let hover = self.earnings_marker_hover_progress_for(time_ms);
        if hover <= 0.01 || ctx.chart_w <= 0.0 || ctx.price_h <= 0.0 {
            return;
        }
        let Some(marker) = self
            .earnings_markers
            .iter()
            .find(|marker| marker.time_ms == time_ms)
        else {
            return;
        };
        let Some(x) = self.timestamp_to_x(time_ms, ctx.state, ctx.chart_w) else {
            return;
        };
        if !x.is_finite() || x < 0.0 || x > ctx.chart_w {
            return;
        }

        let source_center = Point::new(x, earnings_marker_dot_y(ctx.price_h));
        let visual_center = ctx.fisheye.project(source_center);
        let accent = ctx.theme.palette().primary;
        let radius = hud_lerp(EARNINGS_DOT_RADIUS, EARNINGS_HOVER_RADIUS, hover);
        ctx.fisheye.fill_projected_circle(
            ctx.frame,
            source_center,
            radius + 3.5 * hover,
            Color {
                a: 0.14 * hover,
                ..accent
            },
        );
        ctx.fisheye.fill_projected_circle(
            ctx.frame,
            source_center,
            radius,
            Color {
                a: hud_lerp(0.72, 0.9, hover),
                ..accent
            },
        );
        ctx.fisheye.stroke_projected_circle(
            ctx.frame,
            source_center,
            radius,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.32 + 0.38 * hover,
                    ..Color::WHITE
                })
                .with_width(0.75 + 0.35 * hover),
        );

        let mut lines = Vec::with_capacity(9);
        lines.push(TooltipLine {
            content: format!(
                "EARN {}",
                marker.quarter_label.as_deref().unwrap_or("Quarter unknown")
            ),
            color: Color {
                a: 0.74,
                ..ctx.theme.palette().text
            },
        });
        if !marker.accession_number.is_empty() {
            lines.push(TooltipLine {
                content: format!("Filing # {}", marker.accession_number),
                color: accent,
            });
        }
        if !marker.form.is_empty() {
            lines.push(TooltipLine {
                content: format!("Form {}", marker.form),
                color: Color {
                    a: 0.68,
                    ..ctx.theme.palette().text
                },
            });
        }
        if !marker.filing_date.is_empty() {
            lines.push(TooltipLine {
                content: format!("Filed {}", marker.filing_date),
                color: Color {
                    a: 0.62,
                    ..ctx.theme.palette().text
                },
            });
        }
        append_earnings_summary_lines(
            marker,
            &mut lines,
            accent,
            Color {
                a: 0.7,
                ..ctx.theme.palette().text
            },
        );
        if marker.cik != 0
            && !marker.accession_number.is_empty()
            && !marker.primary_document.is_empty()
        {
            lines.push(TooltipLine {
                content: "Click to open filing".to_string(),
                color: Color { a: 0.78, ..accent },
            });
        }

        let card_size = TooltipSurface::card_size_for_lines(&lines, 220.0);
        let card_x = (visual_center.x + 12.0)
            .min(ctx.chart_w - card_size.width - 4.0)
            .max(4.0);
        let max_card_y = (ctx.price_h - card_size.height).max(0.0);
        let card_y = (visual_center.y - card_size.height - 10.0).clamp(0.0, max_card_y);
        let mut tooltip_surface = TooltipSurface::new(
            ctx.frame,
            ctx.theme,
            visual_center,
            ctx.chart_w,
            ctx.price_h,
        );
        tooltip_surface.draw_card(Point::new(card_x, card_y), card_size, &lines);
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
        if !self.crosshair_style.is_game_hud() || ctx.chart_w < 180.0 || ctx.price_h < 90.0 {
            return;
        }

        let symbol = if self.symbol_label.trim().is_empty() {
            format!("CHART {}", self.id)
        } else {
            self.symbol_label.to_uppercase()
        };
        let accent = self.hud_accent_color(ctx.theme, ctx.state);

        // Spotting block left of the reticle: instrument identity + ghost
        // telemetry. Clock and candle countdown live in the mission strip.
        let left_lines =
            hud_left_block_lines(self.hud_readout, &symbol, self.timeframe.label(), data_pos);
        if !left_lines.is_empty() {
            let left_size = hud_text_block_size(&left_lines);
            let left_origin = hud_block_origin(
                visual_pos,
                -left_size.width - 42.0,
                -left_size.height - 26.0,
                left_size,
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
            for (index, line) in left_lines.iter().enumerate() {
                let position = Point::new(
                    left_origin.x,
                    left_origin.y + index as f32 * HUD_LINE_HEIGHT + HUD_LINE_HEIGHT * 0.5,
                );
                // Coordinates are debug-class telemetry: ghosted, never accented.
                let color = if line.starts_with("XY ") {
                    Color { a: 0.40, ..accent }
                } else {
                    accent
                };
                draw_hud_text(
                    ctx.frame,
                    line,
                    position,
                    color,
                    alignment::Horizontal::Left,
                );
            }
        }

        // Jet-HUD tapes on the cursor line: price right (altitude), time left
        // (airspeed). Zero vertical eye travel from the aim point.
        let tape_y_offset = if self.crosshair_style.normalized() == ChartCrosshairStyle::RacingHud {
            -50.0
        } else {
            0.0
        };
        let tape_y = (visual_pos.y + tape_y_offset).clamp(10.0, ctx.price_h - 10.0);
        let price_label = self.hud_readout.price.then(|| format_price(hover_price));
        let time_label = if self.hud_readout.hover_time {
            hover_timestamp_ms.map(format_hud_hover_time)
        } else {
            None
        };
        let price_side = price_label.as_deref().and_then(|label| {
            hud_jet_tape_side(
                visual_pos.x,
                hud_text_width(label, HUD_JET_TAPE_TEXT_SIZE),
                1.0,
                ctx.chart_w,
            )
        });
        let time_side = time_label.as_deref().and_then(|label| {
            hud_jet_tape_side(
                visual_pos.x,
                hud_text_width(label, HUD_JET_TAPE_TEXT_SIZE),
                -1.0,
                ctx.chart_w,
            )
        });
        // Near a plot edge both tapes can resolve to the same side; stack the
        // time tape one slot off the cursor line instead of double-drawing.
        let time_y = if price_side.is_some() && price_side == time_side {
            if tape_y + 16.0 > ctx.price_h - 10.0 {
                tape_y - 16.0
            } else {
                tape_y + 16.0
            }
        } else {
            tape_y
        };
        if let (Some(label), Some(side)) = (price_label.as_deref(), price_side) {
            draw_hud_jet_tape(
                ctx.frame,
                visual_pos.x,
                tape_y,
                label,
                Color { a: 0.95, ..accent },
                side,
            );
        }
        if let (Some(label), Some(side)) = (time_label.as_deref(), time_side) {
            draw_hud_jet_tape(
                ctx.frame,
                visual_pos.x,
                time_y,
                label,
                Color { a: 0.75, ..accent },
                side,
            );
        }
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
        if self.crosshair_style.is_game_hud() {
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

    fn racing_hud_metrics(
        &self,
        state: &ChartState,
        hover_price: Option<f64>,
    ) -> Option<RacingHudMetrics> {
        if self.crosshair_style.normalized() != ChartCrosshairStyle::RacingHud {
            return None;
        }

        let current_size = parse_hud_size_value(hud_display_size(state));
        let price_for_max_size = self
            .market_reference_price
            .or_else(|| self.candles.last().map(|candle| candle.close))
            .or(hover_price)
            .and_then(positive_finite_value);
        let max_size = self
            .hud_max_notional
            .zip(price_for_max_size)
            .and_then(|(max_notional, price)| positive_finite_value(max_notional / price));

        Some(RacingHudMetrics::new(
            current_size,
            max_size,
            self.current_spread,
            self.spread_history_bounds(),
        ))
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
        let pulse_wave = self.hud_armed.then(|| hud_pulse_wave(self.hud_pulse_phase));
        draw_hud_market_price_line(ctx.frame, center, target, accent, pulse_wave);
    }
}

fn hud_game_panels_visible(
    style: ChartCrosshairStyle,
    cursor_position: Option<Point>,
    chart_w: f32,
    drawable_h: f32,
) -> bool {
    style.is_game_hud()
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

/// Lines for the cursor-attached spotting block: instrument identity first,
/// ghost cursor telemetry last. Price/time ride the cursor line as jet tapes
/// and clock/candle-close live in the top-center mission strip.
fn hud_left_block_lines(
    config: ChartHudReadoutConfig,
    symbol: &str,
    timeframe: &str,
    data_pos: Point,
) -> Vec<String> {
    let mut lines = Vec::new();
    if config.symbol {
        lines.push(format!("{symbol} {timeframe}"));
    }
    if config.coordinates {
        lines.push(format!("XY {:>5.1} {:>5.1}", data_pos.x, data_pos.y));
    }
    lines
}

/// Monospace width estimate scaled to the rendered pixel size.
fn hud_text_width(label: &str, size: f32) -> f32 {
    label.chars().count() as f32 * size * 0.61
}

/// Picks the side of the reticle (+1.0 right, -1.0 left) where a jet tape's
/// full extent fits inside the plot: the preferred side first, its mirror as
/// the edge-flip fallback, `None` when neither fits.
fn hud_jet_tape_side(cursor_x: f32, label_width: f32, preferred: f32, chart_w: f32) -> Option<f32> {
    let fits = |side: f32| {
        let bracket_x = cursor_x + side * HUD_JET_TAPE_GAP;
        let outer_x = cursor_x + side * (HUD_JET_TAPE_GAP + 6.0 + label_width);
        (4.0..=chart_w - 4.0).contains(&bracket_x) && (4.0..=chart_w - 4.0).contains(&outer_x)
    };
    if fits(preferred) {
        Some(preferred)
    } else if fits(-preferred) {
        Some(-preferred)
    } else {
        None
    }
}

/// Half-bracket readout on the cursor line, jet-HUD style: `side` +1.0 draws
/// right of the reticle (price/altitude slot), -1.0 left (time/airspeed
/// slot). The side must come from `hud_jet_tape_side`.
fn draw_hud_jet_tape(
    frame: &mut canvas::Frame,
    cursor_x: f32,
    y: f32,
    label: &str,
    color: Color,
    side: f32,
) {
    let bracket_x = cursor_x + side * HUD_JET_TAPE_GAP;

    let bracket = canvas::Path::new(|path| {
        path.move_to(Point::new(bracket_x - side * 5.0, y - 7.0));
        path.line_to(Point::new(bracket_x, y - 7.0));
        path.line_to(Point::new(bracket_x, y + 7.0));
        path.line_to(Point::new(bracket_x - side * 5.0, y + 7.0));
    });
    frame.stroke(
        &bracket,
        canvas::Stroke::default()
            .with_color(Color {
                a: color.a * 0.8,
                ..color
            })
            .with_width(1.0)
            .with_line_cap(canvas::LineCap::Round),
    );

    let (text_x, align_x) = if side > 0.0 {
        (bracket_x + 6.0, alignment::Horizontal::Left)
    } else {
        (bracket_x - 6.0, alignment::Horizontal::Right)
    };
    draw_hud_text_sized(
        frame,
        label,
        Point::new(text_x, y),
        color,
        align_x,
        HUD_JET_TAPE_TEXT_SIZE,
    );
}

fn hud_display_size(state: &ChartState) -> &str {
    let size = state.hud_size_input.trim();
    if size.is_empty() { "0" } else { size }
}

fn parse_hud_size_value(input: &str) -> Option<f64> {
    let value = input.trim().parse::<f64>().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value)
}

fn positive_finite_value(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}

/// One text segment of the single-line firing summary, with an optional
/// up/down triangle glyph drawn after it.
struct HudSummarySegment {
    content: String,
    color: Color,
    triangle_up: Option<bool>,
}

impl CandlestickChart {
    /// Single-line "what fires on click" readout above the reticle. Limit
    /// mode shows the live inferred side from the same reference price the
    /// click handler uses; armed state flanks the line with pulsing amber
    /// chevrons and a caret instead of the old hazard triangle.
    fn draw_hud_order_summary<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        center: Point,
        accent: Color,
        hover_price: Option<f64>,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let state = ctx.state;
        let size = hud_display_size(state);
        let success = ctx.theme.palette().success;
        let danger = ctx.theme.palette().danger;
        let text = ctx.theme.palette().text;

        let mut segments: Vec<HudSummarySegment> = Vec::new();
        if !self.hud_armed {
            segments.push(HudSummarySegment {
                content: "SAFE  ".to_string(),
                color: Color { a: 0.45, ..text },
                triangle_up: None,
            });
        }
        match state.hud_order_kind {
            HudOrderKind::Market => {
                let is_long = state.hud_market_side == HudMarketSide::Long;
                let side_color = if is_long { success } else { danger };
                segments.push(HudSummarySegment {
                    content: "MKT ".to_string(),
                    color: accent,
                    triangle_up: None,
                });
                segments.push(HudSummarySegment {
                    content: state.hud_market_side.label().to_string(),
                    color: Color {
                        a: 0.95,
                        ..side_color
                    },
                    triangle_up: Some(is_long),
                });
                segments.push(HudSummarySegment {
                    content: format!(" {size} COIN"),
                    color: accent,
                    triangle_up: None,
                });
            }
            HudOrderKind::Limit => {
                match hover_price.and_then(|price| self.hud_limit_click_is_buy(price)) {
                    Some(is_buy) => {
                        let (side_label, side_color) = if is_buy {
                            ("BUY", success)
                        } else {
                            ("SELL", danger)
                        };
                        segments.push(HudSummarySegment {
                            content: "LIMIT ".to_string(),
                            color: accent,
                            triangle_up: None,
                        });
                        segments.push(HudSummarySegment {
                            content: side_label.to_string(),
                            color: Color {
                                a: 0.95,
                                ..side_color
                            },
                            triangle_up: Some(is_buy),
                        });
                        segments.push(HudSummarySegment {
                            content: format!(" {size} COIN"),
                            color: accent,
                            triangle_up: None,
                        });
                    }
                    None => segments.push(HudSummarySegment {
                        content: format!("LIMIT {size} COIN"),
                        color: accent,
                        triangle_up: None,
                    }),
                }
            }
        }

        const TRIANGLE_W: f32 = 10.0;
        let total_width: f32 = segments
            .iter()
            .map(|segment| {
                segment.content.chars().count() as f32 * HUD_CHAR_WIDTH
                    + if segment.triangle_up.is_some() {
                        TRIANGLE_W
                    } else {
                        0.0
                    }
            })
            .sum();
        let y = center.y - 36.0;
        let mut x = center.x - total_width * 0.5;
        for segment in &segments {
            draw_hud_text(
                ctx.frame,
                &segment.content,
                Point::new(x, y),
                segment.color,
                alignment::Horizontal::Left,
            );
            x += segment.content.chars().count() as f32 * HUD_CHAR_WIDTH;
            if let Some(up) = segment.triangle_up {
                fill_triangle(
                    ctx.frame,
                    Point::new(x + 4.0, y),
                    3.5,
                    6.0,
                    up,
                    segment.color,
                );
                x += TRIANGLE_W;
            }
        }

        if self.hud_armed {
            let wave = hud_pulse_wave(self.hud_pulse_phase);
            let amber = Color {
                a: 0.45 + 0.45 * wave,
                ..HUD_WARNING_YELLOW
            };
            fill_chevron_right(
                ctx.frame,
                Point::new(center.x - total_width * 0.5 - 12.0, y),
                3.5,
                amber,
            );
            game_hud::fill_chevron_left(
                ctx.frame,
                Point::new(center.x + total_width * 0.5 + 12.0, y),
                3.5,
                amber,
            );
            let caret = canvas::Path::new(|path| {
                path.move_to(Point::new(center.x - 4.0, y - 11.0));
                path.line_to(Point::new(center.x, y - 16.0));
                path.line_to(Point::new(center.x + 4.0, y - 11.0));
            });
            ctx.frame.stroke(
                &caret,
                canvas::Stroke::default()
                    .with_color(amber)
                    .with_width(1.5)
                    .with_line_cap(canvas::LineCap::Round)
                    .with_line_join(canvas::LineJoin::Round),
            );
        }
    }
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

/// Dashed bearing line from the reticle to the live market price, ending in
/// a lock-on diamond — the "range to target" instrument for market orders.
fn draw_hud_market_price_line(
    frame: &mut canvas::Frame,
    center: Point,
    target: Point,
    accent: Color,
    pulse_wave: Option<f32>,
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

    let line = canvas::Path::line(center, line_end);
    frame.stroke(
        &line,
        canvas::Stroke {
            line_dash: canvas::LineDash {
                segments: &[4.0, 3.0],
                offset: 0,
            },
            ..canvas::Stroke::default()
                .with_color(Color { a: 0.72, ..accent })
                .with_width(1.0)
                .with_line_cap(canvas::LineCap::Round)
        },
    );

    draw_hud_lock_diamond(frame, target, accent, pulse_wave);
}

fn draw_hud_lock_diamond(
    frame: &mut canvas::Frame,
    center: Point,
    accent: Color,
    pulse_wave: Option<f32>,
) {
    let radius = HUD_MARKET_TARGET_RADIUS * 0.62;
    let diamond_points = |offset: f32| {
        canvas::Path::new(|path| {
            path.move_to(Point::new(center.x + offset, center.y - radius + offset));
            path.line_to(Point::new(center.x + radius + offset, center.y + offset));
            path.line_to(Point::new(center.x + offset, center.y + radius + offset));
            path.line_to(Point::new(center.x - radius + offset, center.y + offset));
            path.close();
        })
    };
    frame.stroke(
        &diamond_points(1.0),
        canvas::Stroke::default()
            .with_color(HUD_SHADOW)
            .with_width(2.2)
            .with_line_join(canvas::LineJoin::Round),
    );
    frame.stroke(
        &diamond_points(0.0),
        canvas::Stroke::default()
            .with_color(Color { a: 0.9, ..accent })
            .with_width(1.25)
            .with_line_join(canvas::LineJoin::Round),
    );

    // Four diagonal lock ticks; they breathe with the armed pulse.
    let tick_alpha = pulse_wave.map_or(0.55, |wave| 0.40 + 0.45 * wave);
    let tick_stroke = canvas::Stroke::default()
        .with_color(Color {
            a: tick_alpha,
            ..accent
        })
        .with_width(1.1)
        .with_line_cap(canvas::LineCap::Round);
    let inner = radius + 2.0;
    let outer = radius + 6.0;
    let diagonal = std::f32::consts::FRAC_1_SQRT_2;
    for x_sign in [-1.0, 1.0] {
        for y_sign in [-1.0, 1.0] {
            let tick = canvas::Path::line(
                Point::new(
                    center.x + x_sign * inner * diagonal,
                    center.y + y_sign * inner * diagonal,
                ),
                Point::new(
                    center.x + x_sign * outer * diagonal,
                    center.y + y_sign * outer * diagonal,
                ),
            );
            frame.stroke(&tick, tick_stroke);
        }
    }
}

fn hud_lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t.clamp(0.0, 1.0)
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
    draw_hud_text_sized(frame, content, position, color, align_x, 10.5);
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

fn append_earnings_summary_lines(
    marker: &EarningsMarker,
    lines: &mut Vec<TooltipLine>,
    accent: Color,
    muted: Color,
) {
    if marker.filing_summary_loading {
        lines.push(TooltipLine {
            content: "Summary loading".to_string(),
            color: muted,
        });
        return;
    }

    if let Some(summary) = &marker.filing_summary {
        if let Some(headline) = summary
            .headline
            .as_deref()
            .map(str::trim)
            .filter(|headline| !headline.is_empty())
        {
            lines.push(TooltipLine {
                content: compact_earnings_tooltip_text(
                    headline,
                    EARNINGS_TOOLTIP_SUMMARY_MAX_CHARS,
                ),
                color: accent,
            });
        }

        for highlight in summary
            .highlights
            .iter()
            .filter(|item| !item.trim().is_empty())
            .take(2)
        {
            lines.push(TooltipLine {
                content: format!(
                    "- {}",
                    compact_earnings_tooltip_text(highlight, EARNINGS_TOOLTIP_SUMMARY_MAX_CHARS)
                ),
                color: muted,
            });
        }
        return;
    }

    if let Some(status) = marker
        .filing_summary_status
        .as_deref()
        .map(str::trim)
        .filter(|status| !status.is_empty())
    {
        lines.push(TooltipLine {
            content: status.to_string(),
            color: muted,
        });
    }
}

fn compact_earnings_tooltip_text(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }

    let mut out = normalized
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    while out.chars().last().is_some_and(char::is_whitespace) {
        out.pop();
    }
    out.push_str("...");
    out
}

/// Whole-unit markets (outcome contracts) read as whole counts below the
/// compaction threshold; everything else uses the fractional compact form.
pub(super) fn format_volume_readout(volume: f64, whole_units: bool) -> String {
    if whole_units && volume.is_finite() && (0.0..1_000.0).contains(&volume) {
        return format!("{volume:.0}");
    }
    format_volume_compact(volume)
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
