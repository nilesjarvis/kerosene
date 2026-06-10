use super::super::countdown::{next_candle_countdown_label, remaining_ms_until_next_candle};
use super::super::hud_order_animation::{HUD_FEED_MAX_ROWS, HUD_FEED_TTL_MS};
use super::super::hud_safety::HUD_ARM_IDLE_TIMEOUT_MS;
use super::super::model::{CandlestickChart, HudSelectorKind};
use super::super::state::{ChartState, HudMarketSide, HudOrderKind};
use super::{
    CrosshairOverlayContext, HUD_CHAR_WIDTH, HUD_SHADOW, HUD_WARNING_YELLOW, draw_hud_text,
    format_hud_clock_time, hud_display_size,
};
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Size, alignment};

// ---------------------------------------------------------------------------
// Battlefield Game HUD Chrome
// ---------------------------------------------------------------------------
//
// Screen regions follow shooter-HUD grammar: armed combat frame at the plot
// corners, mission strip (clock + candle timer) top-center, battle feed of
// recent shots top-right, nav chip (camera follow) bottom-left, weapon
// station (mode / side / size / safety) bottom-right, and a ghost context
// prompt bottom-center. Cursor-attached pieces live in `crosshair.rs`.

const FRAME_INSET: f32 = 8.0;
const FRAME_CHAMFER: f32 = 6.0;

const STATION_MARGIN: f32 = 8.0;
const STATION_WIDTH: f32 = 158.0;
const STATION_PAD: f32 = 7.0;
const STATION_ROW_H: f32 = 13.0;
const STATION_COMPACT_WIDTH: f32 = 140.0;
const STATION_MIN_W: f32 = 200.0;
const STATION_MIN_H: f32 = 120.0;
const STATION_FULL_MIN_W: f32 = 380.0;
const STATION_FULL_MIN_H: f32 = 250.0;

const FEED_ROW_H: f32 = 12.0;
/// Glyph advance for the 9.5px battle feed text.
const FEED_CHAR_WIDTH: f32 = 5.9;

const STRIP_MIN_W: f32 = 360.0;
// Below this width the centered prompt would reach into the weapon station.
const PROMPT_MIN_W: f32 = 640.0;
const PROMPT_CURSOR_CLEARANCE: f32 = 60.0;

const TAPE_WIDTH: f32 = 96.0;
const TAPE_TICKS: usize = 17;

const SELECTOR_SLOT_H: f32 = 22.0;
const SELECTOR_SLOT_GAP: f32 = 4.0;
const SELECTOR_PAD: f32 = 5.0;
/// Vertical gap between the popup and the weapon station beneath it.
const SELECTOR_GAP: f32 = 6.0;

pub(in crate::chart) fn hud_pulse_wave(phase: f32) -> f32 {
    ((phase.clamp(0.0, 1.0) * std::f32::consts::TAU).sin() + 1.0) * 0.5
}

// ---------------------------------------------------------------------------
// Armed Combat Frame
// ---------------------------------------------------------------------------

impl CandlestickChart {
    /// Amber corner brackets + stencil tag shown whenever the chart is hot,
    /// cursor on the plot or not. Disarming removes them instantly.
    pub(super) fn draw_hud_armed_frame(
        &self,
        frame: &mut canvas::Frame,
        chart_w: f32,
        price_h: f32,
    ) {
        if chart_w < 120.0 || price_h < 90.0 {
            return;
        }

        let wave = hud_pulse_wave(self.hud_pulse_phase);
        let stroke = canvas::Stroke::default()
            .with_color(Color {
                a: 0.30 + 0.22 * wave,
                ..HUD_WARNING_YELLOW
            })
            .with_width(1.5)
            .with_line_cap(canvas::LineCap::Round)
            .with_line_join(canvas::LineJoin::Round);
        let leg = if chart_w < 360.0 { 10.0 } else { 18.0 };

        for x_sign in [1.0, -1.0] {
            for y_sign in [1.0, -1.0] {
                let corner = Point::new(
                    if x_sign > 0.0 {
                        FRAME_INSET
                    } else {
                        chart_w - FRAME_INSET
                    },
                    if y_sign > 0.0 {
                        FRAME_INSET
                    } else {
                        price_h - FRAME_INSET
                    },
                );
                draw_chamfer_bracket(frame, corner, x_sign, y_sign, leg, stroke);
            }
        }

        draw_hud_text_sized(
            frame,
            "[ A R M E D ]",
            Point::new(chart_w * 0.5, FRAME_INSET + 5.0),
            Color {
                a: 0.55 + 0.30 * wave,
                ..HUD_WARNING_YELLOW
            },
            alignment::Horizontal::Center,
            10.5,
        );
    }
}

/// Two legs meeting at a 45 degree chamfer elbow instead of a sharp corner.
fn draw_chamfer_bracket(
    frame: &mut canvas::Frame,
    corner: Point,
    x_sign: f32,
    y_sign: f32,
    leg: f32,
    stroke: canvas::Stroke<'static>,
) {
    let path = canvas::Path::new(|path| {
        path.move_to(Point::new(
            corner.x + x_sign * (FRAME_CHAMFER + leg),
            corner.y,
        ));
        path.line_to(Point::new(corner.x + x_sign * FRAME_CHAMFER, corner.y));
        path.line_to(Point::new(corner.x, corner.y + y_sign * FRAME_CHAMFER));
        path.line_to(Point::new(
            corner.x,
            corner.y + y_sign * (FRAME_CHAMFER + leg),
        ));
    });
    frame.stroke(&path, stroke);
}

// ---------------------------------------------------------------------------
// Chrome Orchestration
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_hud_game_chrome<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        accent: Color,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        self.draw_hud_mission_strip(ctx, accent);
        self.draw_hud_battle_feed(ctx);
        self.draw_hud_weapon_station(ctx, accent);
        self.draw_hud_weapon_selector(ctx, accent);
        draw_hud_nav_chip(ctx, accent);
        self.draw_hud_context_prompt(ctx, accent);
    }
}

// ---------------------------------------------------------------------------
// Mission Strip (top-center)
// ---------------------------------------------------------------------------

impl CandlestickChart {
    fn draw_hud_mission_strip<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        accent: Color,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        if !self.hud_mission_strip_visible(ctx.chart_w, ctx.price_h) {
            return;
        }
        let show_clock = self.hud_readout.clock && ctx.chart_w >= 420.0;
        let show_cls = self.hud_readout.candle_close;

        let center_x = ctx.chart_w * 0.5;
        let y = if self.hud_armed {
            FRAME_INSET + 18.0
        } else {
            FRAME_INSET + 5.0
        };
        let ghost = Color {
            a: 0.55,
            ..ctx.theme.palette().text
        };

        let clock_label =
            show_clock.then(|| format!("NOW {}", format_hud_clock_time(self.clock_now_ms)));
        let cls_label = show_cls.then(|| {
            let countdown = self
                .candles
                .last()
                .and_then(|candle| {
                    next_candle_countdown_label(candle.open_time, self.timeframe, self.clock_now_ms)
                })
                .unwrap_or_else(|| "--".to_string());
            format!("CLS {countdown}")
        });

        let cls_anchor_x = match (&clock_label, &cls_label) {
            (Some(clock), Some(cls)) => {
                draw_hud_text(
                    ctx.frame,
                    clock,
                    Point::new(center_x - 10.0, y),
                    ghost,
                    alignment::Horizontal::Right,
                );
                ctx.frame.fill(
                    &canvas::Path::circle(Point::new(center_x, y), 1.0),
                    Color { a: 0.45, ..accent },
                );
                draw_hud_text(
                    ctx.frame,
                    cls,
                    Point::new(center_x + 10.0, y),
                    Color { a: 0.92, ..accent },
                    alignment::Horizontal::Left,
                );
                center_x + 10.0 + cls.chars().count() as f32 * HUD_CHAR_WIDTH * 0.5
            }
            (Some(clock), None) => {
                draw_hud_text(
                    ctx.frame,
                    clock,
                    Point::new(center_x, y),
                    ghost,
                    alignment::Horizontal::Center,
                );
                center_x
            }
            (None, Some(cls)) => {
                draw_hud_text(
                    ctx.frame,
                    cls,
                    Point::new(center_x, y),
                    Color { a: 0.92, ..accent },
                    alignment::Horizontal::Center,
                );
                center_x
            }
            (None, None) => return,
        };

        if show_cls {
            self.draw_hud_candle_progress_tape(ctx, Point::new(cls_anchor_x, y + 9.0), accent);
        }
    }

    /// Tick tape under the CLS timer showing how much of the current candle
    /// has elapsed; advances in wall-clock steps, no animation driver needed.
    fn draw_hud_candle_progress_tape<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        center: Point,
        accent: Color,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(progress) = self.hud_candle_progress() else {
            return;
        };

        let left = center.x - TAPE_WIDTH * 0.5;
        let step = TAPE_WIDTH / (TAPE_TICKS - 1) as f32;
        let boundary = TAPE_TICKS as f32 * progress;
        for index in 0..TAPE_TICKS {
            let elapsed = (index as f32) < boundary;
            let x = left + index as f32 * step;
            let tick = canvas::Path::line(Point::new(x, center.y), Point::new(x, center.y + 4.0));
            ctx.frame.stroke(
                &tick,
                canvas::Stroke::default()
                    .with_color(Color {
                        a: if elapsed { 0.65 } else { 0.25 },
                        ..accent
                    })
                    .with_width(1.0),
            );
        }

        let caret_x = left + TAPE_WIDTH * progress;
        fill_triangle(
            ctx.frame,
            Point::new(caret_x, center.y - 2.0),
            3.0,
            4.0,
            false,
            Color { a: 0.9, ..accent },
        );
    }

    fn hud_mission_strip_visible(&self, chart_w: f32, price_h: f32) -> bool {
        if chart_w < STRIP_MIN_W || price_h < 140.0 {
            return false;
        }
        (self.hud_readout.clock && chart_w >= 420.0) || self.hud_readout.candle_close
    }

    fn hud_candle_progress(&self) -> Option<f32> {
        let last_open_ms = self.candles.last().map(|candle| candle.open_time)?;
        let interval_ms = self.timeframe.duration_ms();
        let remaining_ms =
            remaining_ms_until_next_candle(last_open_ms, interval_ms, self.clock_now_ms)?;
        if interval_ms == 0 {
            return None;
        }
        Some((1.0 - remaining_ms as f32 / interval_ms as f32).clamp(0.0, 1.0))
    }
}

// ---------------------------------------------------------------------------
// Battle Feed (top-right)
// ---------------------------------------------------------------------------

impl CandlestickChart {
    fn draw_hud_battle_feed<PriceToY>(&self, ctx: &mut CrosshairOverlayContext<'_, PriceToY>)
    where
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.chart_w < STRIP_MIN_W || self.hud_feed.is_empty() {
            return;
        }

        let bull = self.chart_bull_color.unwrap_or(ctx.theme.palette().success);
        let bear = self.chart_bear_color.unwrap_or(ctx.theme.palette().danger);
        let right = ctx.chart_w - 10.0;
        // Start below whatever occupies the top band (ARMED tag, mission
        // strip + progress tape) so long rows never smear across them.
        let strip_visible = self.hud_mission_strip_visible(ctx.chart_w, ctx.price_h);
        let feed_top = if self.hud_armed && strip_visible {
            44.0
        } else if self.hud_armed || strip_visible {
            30.0
        } else {
            FRAME_INSET + 10.0
        };
        let mut row = 0usize;
        for entry in self.hud_feed.iter().rev() {
            if row >= HUD_FEED_MAX_ROWS {
                break;
            }
            let Some(alpha) = hud_feed_alpha(entry.added_at_ms, self.clock_now_ms) else {
                continue;
            };
            let y = feed_top + row as f32 * FEED_ROW_H;
            let color = if entry.is_buy { bull } else { bear };
            let label_width = entry.label.chars().count() as f32 * FEED_CHAR_WIDTH;
            draw_hud_text_sized(
                ctx.frame,
                &entry.label,
                Point::new(right, y),
                Color { a: alpha, ..color },
                alignment::Horizontal::Right,
                9.5,
            );
            fill_chevron_right(
                ctx.frame,
                Point::new(right - label_width - 8.0, y),
                3.0,
                Color { a: alpha, ..color },
            );
            row += 1;
        }
    }
}

/// Stepwise 1s fade over the feed lifetime; `None` once expired.
fn hud_feed_alpha(added_at_ms: u64, now_ms: u64) -> Option<f32> {
    let age_ms = now_ms.saturating_sub(added_at_ms);
    if age_ms >= HUD_FEED_TTL_MS {
        return None;
    }
    let steps_left = (HUD_FEED_TTL_MS - age_ms).div_ceil(1_000);
    let total_steps = HUD_FEED_TTL_MS / 1_000;
    Some(0.85 * steps_left as f32 / total_steps as f32)
}

// ---------------------------------------------------------------------------
// Weapon Station (bottom-right)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::chart) struct HudStationMetrics {
    pub(in crate::chart) bounds: Rectangle,
    pub(in crate::chart) compact: bool,
}

/// Shared by drawing and the armed-click deadzone in the press handler.
pub(in crate::chart) fn hud_station_metrics(
    chart_w: f32,
    price_h: f32,
) -> Option<HudStationMetrics> {
    if chart_w < STATION_MIN_W || price_h < STATION_MIN_H {
        return None;
    }

    let compact = chart_w < STATION_FULL_MIN_W || price_h < STATION_FULL_MIN_H;
    let (width, height) = if compact {
        (STATION_COMPACT_WIDTH, 2.0 * STATION_ROW_H + 10.0)
    } else {
        // pad + size row + rule + equipped row + hint row + rule + safety + pad
        let height = STATION_PAD * 2.0 + 18.0 + 11.0 + STATION_ROW_H + 11.0 + 11.0 + 14.0;
        (STATION_WIDTH, height)
    };

    let x = (chart_w - STATION_MARGIN - width).max(0.0);
    let y = (price_h - STATION_MARGIN - height).max(0.0);
    Some(HudStationMetrics {
        bounds: Rectangle {
            x,
            y,
            width,
            height,
        },
        compact,
    })
}

/// Bounds of the transient weapon-selector popup above the station; `None`
/// when there is no room for it (or no station to anchor to).
pub(in crate::chart) fn hud_selector_bounds(chart_w: f32, price_h: f32) -> Option<Rectangle> {
    let station = hud_station_metrics(chart_w, price_h)?;
    let width = station.bounds.width;
    let height = 2.0 * SELECTOR_SLOT_H + SELECTOR_SLOT_GAP + 2.0 * SELECTOR_PAD;
    let y = station.bounds.y - SELECTOR_GAP - height;
    (y >= 4.0).then_some(Rectangle {
        x: station.bounds.x,
        y,
        width,
        height,
    })
}

impl CandlestickChart {
    fn draw_hud_weapon_station<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        accent: Color,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(metrics) = hud_station_metrics(ctx.chart_w, ctx.price_h) else {
            return;
        };
        let bounds = metrics.bounds;

        ctx.frame.fill_rectangle(
            Point::new(bounds.x, bounds.y),
            Size::new(bounds.width, bounds.height),
            Color {
                a: 0.30,
                ..Color::BLACK
            },
        );
        draw_station_corner_ticks(ctx.frame, bounds, accent);

        if metrics.compact {
            self.draw_hud_station_compact(ctx, bounds, accent);
        } else {
            self.draw_hud_station_full(ctx, bounds, accent);
        }
    }

    fn draw_hud_station_full<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        bounds: Rectangle,
        accent: Color,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let text = ctx.theme.palette().text;
        let left = bounds.x + STATION_PAD;
        let right = bounds.x + bounds.width - STATION_PAD;
        let mut y = bounds.y + STATION_PAD + 9.0;

        // Size ("ammo") row.
        let editing = ctx.state.hud_size_editing;
        let size_color = if editing {
            HUD_WARNING_YELLOW
        } else {
            Color { a: 0.95, ..accent }
        };
        draw_hud_text_sized(
            ctx.frame,
            "SIZE [S]",
            Point::new(left, y),
            Color { a: 0.55, ..text },
            alignment::Horizontal::Left,
            9.0,
        );
        let mut size_value = hud_display_size(ctx.state).to_string();
        if editing && self.hud_size_caret_visible() {
            size_value.push('_');
        }
        let (value_size, show_unit) = hud_station_size_text(size_value.chars().count());
        if show_unit {
            draw_hud_text_sized(
                ctx.frame,
                "COIN",
                Point::new(right, y + 2.0),
                Color { a: 0.50, ..text },
                alignment::Horizontal::Right,
                9.0,
            );
        }
        let value_right = if show_unit { right - 32.0 } else { right };
        draw_hud_text_sized(
            ctx.frame,
            &size_value,
            Point::new(value_right, y),
            size_color,
            alignment::Horizontal::Right,
            value_size,
        );
        y += 14.0;
        draw_station_rule(ctx.frame, left, right, y, accent);
        y += 11.0;

        // Equipped weapon line: only what is loaded fires here. The full
        // loadout list lives in the transient selector popup (L/M/Y/X).
        self.draw_hud_equipped_weapon(ctx, Point::new(left, y), accent);
        y += STATION_ROW_H;
        draw_hud_text_sized(
            ctx.frame,
            "[L][M] SWAP \u{b7} [Y][X] SIDE",
            Point::new(left + 9.0, y),
            Color { a: 0.38, ..text },
            alignment::Horizontal::Left,
            8.5,
        );
        y += 11.0;

        draw_station_rule(ctx.frame, left, right, y, accent);
        y += 11.0;
        self.draw_hud_station_safety_row(ctx.frame, left, right, y, text);
    }

    /// The persistent bottom-right readout of the loaded order type: name,
    /// fire pips (1 = limit/single, 3 = market/auto), and the side word.
    fn draw_hud_equipped_weapon<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        origin: Point,
        accent: Color,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let text = ctx.theme.palette().text;
        let market_mode = ctx.state.hud_order_kind == HudOrderKind::Market;
        let bright = Color { a: 0.95, ..accent };

        fill_chevron_right(ctx.frame, Point::new(origin.x + 2.0, origin.y), 3.0, bright);
        let name = if market_mode { "MARKET" } else { "LIMIT" };
        draw_hud_text_sized(
            ctx.frame,
            name,
            Point::new(origin.x + 9.0, origin.y),
            bright,
            alignment::Horizontal::Left,
            11.0,
        );
        let after_name = origin.x + 9.0 + name.chars().count() as f32 * 11.0 * 0.61 + 7.0;
        draw_fire_pips(
            ctx.frame,
            Point::new(after_name, origin.y),
            if market_mode { 3 } else { 1 },
            bright,
        );
        let after_pips = after_name + 3.0 * 4.5 + 7.0;

        if market_mode {
            let is_long = ctx.state.hud_market_side == HudMarketSide::Long;
            let side_color = if is_long {
                ctx.theme.palette().success
            } else {
                ctx.theme.palette().danger
            };
            let side = ctx.state.hud_market_side.label();
            draw_hud_text_sized(
                ctx.frame,
                side,
                Point::new(after_pips, origin.y),
                Color {
                    a: 0.95,
                    ..side_color
                },
                alignment::Horizontal::Left,
                11.0,
            );
            fill_triangle(
                ctx.frame,
                Point::new(
                    after_pips + side.chars().count() as f32 * 11.0 * 0.61 + 7.0,
                    origin.y,
                ),
                3.0,
                5.0,
                is_long,
                Color {
                    a: 0.95,
                    ..side_color
                },
            );
        } else {
            // Teaches the click-above/below-market side inference.
            draw_hud_text_sized(
                ctx.frame,
                "AUTO",
                Point::new(after_pips, origin.y),
                Color { a: 0.45, ..text },
                alignment::Horizontal::Left,
                9.5,
            );
            let glyph_x = after_pips + 4.0 * 9.5 * 0.61 + 8.0;
            fill_triangle(
                ctx.frame,
                Point::new(glyph_x, origin.y - 2.0),
                2.6,
                4.0,
                true,
                Color {
                    a: 0.45,
                    ..ctx.theme.palette().success
                },
            );
            fill_triangle(
                ctx.frame,
                Point::new(glyph_x + 8.0, origin.y - 2.0),
                2.6,
                4.0,
                false,
                Color {
                    a: 0.45,
                    ..ctx.theme.palette().danger
                },
            );
        }
    }

    fn draw_hud_station_compact<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        bounds: Rectangle,
        accent: Color,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let text = ctx.theme.palette().text;
        let left = bounds.x + STATION_PAD;
        let right = bounds.x + bounds.width - STATION_PAD;
        let summary = hud_station_compact_summary(ctx.state);
        let y = bounds.y + 5.0 + 6.0;
        draw_hud_text(
            ctx.frame,
            &summary,
            Point::new(left, y),
            Color { a: 0.92, ..accent },
            alignment::Horizontal::Left,
        );
        self.draw_hud_station_safety_row(ctx.frame, left, right, y + STATION_ROW_H, text);
    }

    fn draw_hud_station_safety_row(
        &self,
        frame: &mut canvas::Frame,
        left: f32,
        right: f32,
        y: f32,
        text: Color,
    ) {
        if self.hud_armed {
            let wave = hud_pulse_wave(self.hud_pulse_phase);
            draw_hud_text_sized(
                frame,
                "ARMED",
                Point::new(right, y),
                Color {
                    a: 0.80 + 0.20 * wave,
                    ..HUD_WARNING_YELLOW
                },
                alignment::Horizontal::Right,
                11.0,
            );
            self.draw_hud_idle_fuse_bar(frame, Point::new(left, y), right - left - 44.0, wave);
        } else {
            draw_hud_text_sized(
                frame,
                "SAFE [A]",
                Point::new(right, y),
                Color { a: 0.50, ..text },
                alignment::Horizontal::Right,
                11.0,
            );
        }
    }

    /// Drain bar for the idle auto-disarm fuse; full while the cursor keeps
    /// the chart active, draining once attention leaves the plot.
    fn draw_hud_idle_fuse_bar(
        &self,
        frame: &mut canvas::Frame,
        origin: Point,
        width: f32,
        wave: f32,
    ) {
        if width < 24.0 {
            return;
        }
        let fraction = self.hud_idle_fuse_fraction();
        let bar_y = origin.y - 1.5;
        frame.fill_rectangle(
            Point::new(origin.x, bar_y),
            Size::new(width, 3.0),
            Color {
                a: 0.18,
                ..HUD_WARNING_YELLOW
            },
        );
        frame.fill_rectangle(
            Point::new(origin.x, bar_y),
            Size::new(width * fraction, 3.0),
            Color {
                a: 0.55 + 0.25 * wave,
                ..HUD_WARNING_YELLOW
            },
        );
    }

    fn hud_idle_fuse_fraction(&self) -> f32 {
        if self.hud_hovering {
            return 1.0;
        }
        let Some(last_activity_ms) = self.hud_last_activity_ms else {
            return 0.0;
        };
        let elapsed = self.clock_now_ms.saturating_sub(last_activity_ms);
        (1.0 - elapsed as f32 / HUD_ARM_IDLE_TIMEOUT_MS as f32).clamp(0.0, 1.0)
    }

    /// Caret blinks on the armed pulse; falls back to wall-clock parity when
    /// safe since the animation tick only runs while armed.
    fn hud_size_caret_visible(&self) -> bool {
        if self.hud_armed {
            hud_pulse_wave(self.hud_pulse_phase) > 0.5
        } else {
            (self.clock_now_ms / 1_000).is_multiple_of(2)
        }
    }
}

/// Shrinks the ammo value (and drops the COIN unit) so long sizes never run
/// into the "SIZE [S]" caption on the station's most safety-relevant row.
fn hud_station_size_text(char_count: usize) -> (f32, bool) {
    if char_count <= 7 {
        (15.0, true)
    } else if char_count <= 10 {
        (11.0, true)
    } else {
        (11.0, false)
    }
}

fn hud_station_compact_summary(state: &ChartState) -> String {
    let size = hud_display_size(state);
    match state.hud_order_kind {
        HudOrderKind::Limit => format!("L>LIMIT {size}"),
        HudOrderKind::Market => format!("M>{} {size}", state.hud_market_side.label()),
    }
}

fn draw_station_corner_ticks(frame: &mut canvas::Frame, bounds: Rectangle, accent: Color) {
    let stroke = canvas::Stroke::default()
        .with_color(Color { a: 0.50, ..accent })
        .with_width(1.0);
    let leg = 6.0;
    for (x, x_sign) in [(bounds.x, 1.0), (bounds.x + bounds.width, -1.0)] {
        for (y, y_sign) in [(bounds.y, 1.0), (bounds.y + bounds.height, -1.0)] {
            let path = canvas::Path::new(|path| {
                path.move_to(Point::new(x + x_sign * leg, y));
                path.line_to(Point::new(x, y));
                path.line_to(Point::new(x, y + y_sign * leg));
            });
            frame.stroke(&path, stroke);
        }
    }
}

fn draw_station_rule(frame: &mut canvas::Frame, left: f32, right: f32, y: f32, accent: Color) {
    let rule = canvas::Path::line(Point::new(left, y), Point::new(right, y));
    frame.stroke(
        &rule,
        canvas::Stroke::default()
            .with_color(Color { a: 0.25, ..accent })
            .with_width(1.0),
    );
}

/// Classic fire-selector pips: one rectangle for single-shot (limit), three
/// for full-auto (market).
fn draw_fire_pips(frame: &mut canvas::Frame, origin: Point, count: usize, color: Color) {
    for index in 0..count {
        frame.fill_rectangle(
            Point::new(origin.x + index as f32 * 4.5, origin.y - 3.0),
            Size::new(2.4, 6.0),
            color,
        );
    }
}

// ---------------------------------------------------------------------------
// Weapon Selector Popup
// ---------------------------------------------------------------------------

/// Alpha envelope over the selector's normalized lifetime: quick pop-in,
/// hold, then fade back out — the Battlefield weapon-switch rhythm.
fn hud_selector_alpha(age: f32) -> f32 {
    if age < 0.08 {
        (age / 0.08).clamp(0.0, 1.0)
    } else if age > 0.72 {
        ((1.0 - age) / 0.28).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

struct HudSelectorSlot<'a> {
    key: &'a str,
    name: &'a str,
    /// Fire pips when `Some(count)`; side triangle when `None`.
    pips: Option<usize>,
    triangle_up: Option<bool>,
    color: Color,
    selected: bool,
}

impl CandlestickChart {
    /// Transient loadout list above the station: pops open on a selector
    /// keypress with the equipped slot bracketed, then fades out.
    fn draw_hud_weapon_selector<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        accent: Color,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(selector) = self.hud_weapon_selector else {
            return;
        };
        let Some(bounds) = hud_selector_bounds(ctx.chart_w, ctx.price_h) else {
            return;
        };
        let alpha = hud_selector_alpha(selector.age);
        if alpha <= 0.01 {
            return;
        }

        let text = ctx.theme.palette().text;
        let success = ctx.theme.palette().success;
        let danger = ctx.theme.palette().danger;
        let market_mode = ctx.state.hud_order_kind == HudOrderKind::Market;
        let long_side = ctx.state.hud_market_side == HudMarketSide::Long;

        let slots: [HudSelectorSlot<'_>; 2] = match selector.kind {
            HudSelectorKind::Mode => [
                HudSelectorSlot {
                    key: "[L]",
                    name: "LIMIT",
                    pips: Some(1),
                    triangle_up: None,
                    color: if market_mode { text } else { accent },
                    selected: !market_mode,
                },
                HudSelectorSlot {
                    key: "[M]",
                    name: "MARKET",
                    pips: Some(3),
                    triangle_up: None,
                    color: if market_mode { accent } else { text },
                    selected: market_mode,
                },
            ],
            HudSelectorKind::Side => [
                HudSelectorSlot {
                    key: "[Y]",
                    name: "LONG",
                    pips: None,
                    triangle_up: Some(true),
                    color: success,
                    selected: long_side,
                },
                HudSelectorSlot {
                    key: "[X]",
                    name: "SHORT",
                    pips: None,
                    triangle_up: Some(false),
                    color: danger,
                    selected: !long_side,
                },
            ],
        };

        ctx.frame.fill_rectangle(
            Point::new(bounds.x, bounds.y),
            Size::new(bounds.width, bounds.height),
            Color {
                a: 0.38 * alpha,
                ..Color::BLACK
            },
        );
        for (index, slot) in slots.iter().enumerate() {
            let slot_bounds = Rectangle {
                x: bounds.x + SELECTOR_PAD,
                y: bounds.y + SELECTOR_PAD + index as f32 * (SELECTOR_SLOT_H + SELECTOR_SLOT_GAP),
                width: bounds.width - 2.0 * SELECTOR_PAD,
                height: SELECTOR_SLOT_H,
            };
            draw_hud_selector_slot(ctx.frame, slot_bounds, slot, alpha, selector.pop, text);
        }
    }
}

fn draw_hud_selector_slot(
    frame: &mut canvas::Frame,
    bounds: Rectangle,
    slot: &HudSelectorSlot<'_>,
    alpha: f32,
    pop: f32,
    text: Color,
) {
    let center_y = bounds.y + bounds.height * 0.5;
    let (color, name_size) = if slot.selected {
        // The just-equipped slot lands with a brightness pop that settles.
        let boost = (1.0 - pop) * 0.6;
        (
            Color {
                a: ((0.95 + boost) * alpha).min(1.0),
                ..slot.color
            },
            11.5,
        )
    } else {
        (
            Color {
                a: 0.42 * alpha,
                ..text
            },
            10.5,
        )
    };

    if slot.selected {
        frame.fill_rectangle(
            Point::new(bounds.x, bounds.y),
            Size::new(bounds.width, bounds.height),
            Color {
                a: 0.16 * alpha,
                ..slot.color
            },
        );
        draw_station_corner_ticks(frame, bounds, color);
        fill_chevron_right(frame, Point::new(bounds.x + 7.0, center_y), 3.5, color);
    }

    draw_hud_text_sized(
        frame,
        slot.key,
        Point::new(bounds.x + 15.0, center_y),
        Color {
            a: (if slot.selected { 0.65 } else { 0.40 }) * alpha,
            ..text
        },
        alignment::Horizontal::Left,
        9.0,
    );
    let glyph_x = bounds.x + 44.0;
    if let Some(count) = slot.pips {
        draw_fire_pips(frame, Point::new(glyph_x, center_y), count, color);
    }
    if let Some(up) = slot.triangle_up {
        fill_triangle(
            frame,
            Point::new(glyph_x + 5.0, center_y),
            3.5,
            6.0,
            up,
            color,
        );
    }
    draw_hud_text_sized(
        frame,
        slot.name,
        Point::new(glyph_x + 20.0, center_y),
        color,
        alignment::Horizontal::Left,
        name_size,
    );
}

// ---------------------------------------------------------------------------
// Nav Chip (bottom-left)
// ---------------------------------------------------------------------------

fn draw_hud_nav_chip<PriceToY>(ctx: &mut CrosshairOverlayContext<'_, PriceToY>, accent: Color)
where
    PriceToY: Fn(f64) -> f32,
{
    if ctx.price_h < STATION_MIN_H {
        return;
    }
    let text = ctx.theme.palette().text;
    let y = ctx.price_h - 16.0;
    let compact = ctx.chart_w < STRIP_MIN_W;
    let following = ctx.state.hud_follow_price;

    // Inset past the armed corner bracket's vertical leg at FRAME_INSET.
    let chip_x = FRAME_INSET + 8.0;
    draw_hud_text(
        ctx.frame,
        "[C] CAM",
        Point::new(chip_x, y),
        Color { a: 0.55, ..text },
        alignment::Horizontal::Left,
    );
    if compact {
        return;
    }

    let chevrons_x = chip_x + 7.0 * HUD_CHAR_WIDTH + 8.0;
    let state_color = if following {
        Color { a: 0.85, ..accent }
    } else {
        Color { a: 0.45, ..text }
    };
    fill_chevron_right(ctx.frame, Point::new(chevrons_x, y), 3.0, state_color);
    if following {
        fill_chevron_right(ctx.frame, Point::new(chevrons_x + 5.0, y), 3.0, state_color);
    }
    draw_hud_text(
        ctx.frame,
        if following { "FOLLOW" } else { "FREE" },
        Point::new(chevrons_x + 13.0, y),
        state_color,
        alignment::Horizontal::Left,
    );
}

// ---------------------------------------------------------------------------
// Context Prompt (bottom-center)
// ---------------------------------------------------------------------------

impl CandlestickChart {
    fn draw_hud_context_prompt<PriceToY>(
        &self,
        ctx: &mut CrosshairOverlayContext<'_, PriceToY>,
        accent: Color,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.chart_w < PROMPT_MIN_W || ctx.price_h < 160.0 {
            return;
        }
        let center = Point::new(ctx.chart_w * 0.5, ctx.price_h - 16.0);
        if ctx.state.cursor_position.is_some_and(|pos| {
            let dx = pos.x - center.x;
            let dy = pos.y - center.y;
            (dx * dx + dy * dy).sqrt() < PROMPT_CURSOR_CLEARANCE
        }) {
            return;
        }

        let label = hud_context_prompt_label(self.hud_armed, ctx.state);
        let color = if self.hud_armed {
            Color { a: 0.45, ..accent }
        } else {
            Color {
                a: 0.40,
                ..ctx.theme.palette().text
            }
        };
        draw_hud_text_sized(
            ctx.frame,
            &label,
            center,
            color,
            alignment::Horizontal::Center,
            9.5,
        );
    }
}

/// State-aware hotkey legend; the armed line doubles as a final
/// what-will-fire confirmation.
fn hud_context_prompt_label(armed: bool, state: &ChartState) -> String {
    if !armed {
        return "[A] ARM  ·  [S] SIZE  ·  [L]/[M] MODE  ·  [C] CAM".to_string();
    }
    match state.hud_order_kind {
        HudOrderKind::Market => format!(
            "CLICK > FIRE MKT {}  ·  [Y]/[X] SIDE  ·  [A] SAFE",
            state.hud_market_side.label()
        ),
        HudOrderKind::Limit => "CLICK > FIRE LIMIT (SIDE BY PRICE)  ·  [A] SAFE".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Shared Glyphs & Text
// ---------------------------------------------------------------------------

pub(super) fn draw_hud_text_sized(
    frame: &mut canvas::Frame,
    content: &str,
    position: Point,
    color: Color,
    align_x: alignment::Horizontal,
    size: f32,
) {
    // The drop shadow fades with the glyph so ghost text and fading feed
    // rows dim out instead of decaying into dark smudges.
    frame.fill_text(canvas::Text {
        content: content.to_string(),
        position: Point::new(position.x + 1.0, position.y + 1.0),
        color: Color {
            a: HUD_SHADOW.a * color.a.clamp(0.0, 1.0),
            ..HUD_SHADOW
        },
        size: iced::Pixels(size),
        align_x: align_x.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
    frame.fill_text(canvas::Text {
        content: content.to_string(),
        position,
        color,
        size: iced::Pixels(size),
        align_x: align_x.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

pub(super) fn fill_chevron_right(
    frame: &mut canvas::Frame,
    center: Point,
    half_height: f32,
    color: Color,
) {
    fill_chevron(frame, center, half_height, 1.0, color);
}

pub(super) fn fill_chevron_left(
    frame: &mut canvas::Frame,
    center: Point,
    half_height: f32,
    color: Color,
) {
    fill_chevron(frame, center, half_height, -1.0, color);
}

fn fill_chevron(
    frame: &mut canvas::Frame,
    center: Point,
    half_height: f32,
    direction: f32,
    color: Color,
) {
    let path = canvas::Path::new(|path| {
        path.move_to(Point::new(center.x, center.y - half_height));
        path.line_to(Point::new(
            center.x + direction * half_height * 1.4,
            center.y,
        ));
        path.line_to(Point::new(center.x, center.y + half_height));
        path.close();
    });
    frame.fill(&path, color);
}

/// Filled vertical triangle: `up` true points upward (long), false downward.
pub(super) fn fill_triangle(
    frame: &mut canvas::Frame,
    center: Point,
    half_width: f32,
    height: f32,
    up: bool,
    color: Color,
) {
    let (tip_y, base_y) = if up {
        (center.y - height * 0.5, center.y + height * 0.5)
    } else {
        (center.y + height * 0.5, center.y - height * 0.5)
    };
    let path = canvas::Path::new(|path| {
        path.move_to(Point::new(center.x, tip_y));
        path.line_to(Point::new(center.x - half_width, base_y));
        path.line_to(Point::new(center.x + half_width, base_y));
        path.close();
    });
    frame.fill(&path, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_wave_stays_normalized() {
        for phase in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let wave = hud_pulse_wave(phase);
            assert!((0.0..=1.0).contains(&wave), "wave {wave} at phase {phase}");
        }
        assert!(hud_pulse_wave(0.25) > 0.99);
        assert!(hud_pulse_wave(0.75) < 0.01);
    }

    #[test]
    fn station_metrics_anchor_bottom_right_inside_plot() {
        let metrics = hud_station_metrics(800.0, 500.0).expect("station should fit");
        assert!(!metrics.compact);
        let bounds = metrics.bounds;
        assert_eq!(bounds.x + bounds.width, 800.0 - STATION_MARGIN);
        assert_eq!(bounds.y + bounds.height, 500.0 - STATION_MARGIN);
    }

    #[test]
    fn station_metrics_collapse_then_disappear_on_small_charts() {
        assert!(
            hud_station_metrics(360.0, 240.0).is_some_and(|metrics| metrics.compact),
            "mid-size charts get the compact station"
        );
        assert_eq!(hud_station_metrics(180.0, 100.0), None);
    }

    #[test]
    fn selector_popup_sits_above_the_station_with_room_to_spare() {
        let station = hud_station_metrics(800.0, 500.0).expect("station should fit");
        let selector = hud_selector_bounds(800.0, 500.0).expect("selector should fit");

        assert_eq!(selector.x, station.bounds.x);
        assert_eq!(selector.width, station.bounds.width);
        assert!(selector.y + selector.height <= station.bounds.y);
        // No station to anchor to: no popup either.
        assert_eq!(hud_selector_bounds(800.0, STATION_MIN_H - 2.0), None);
    }

    #[test]
    fn selector_alpha_pops_in_holds_then_fades_out() {
        assert_eq!(hud_selector_alpha(0.0), 0.0);
        assert_eq!(hud_selector_alpha(0.08), 1.0);
        assert_eq!(hud_selector_alpha(0.5), 1.0);
        assert!(hud_selector_alpha(0.9) < 0.5);
        assert_eq!(hud_selector_alpha(1.0), 0.0);
    }

    #[test]
    fn feed_alpha_steps_down_and_expires() {
        let added = 10_000;
        let fresh = hud_feed_alpha(added, added).expect("fresh entry visible");
        let old = hud_feed_alpha(added, added + 4_500).expect("aging entry visible");
        assert!(fresh > old);
        assert_eq!(hud_feed_alpha(added, added + HUD_FEED_TTL_MS), None);
    }

    #[test]
    fn context_prompt_teaches_safe_keys_and_confirms_armed_fire() {
        let mut state = ChartState::default();
        assert!(hud_context_prompt_label(false, &state).contains("[A] ARM"));

        state.hud_order_kind = HudOrderKind::Market;
        state.hud_market_side = HudMarketSide::Short;
        let armed = hud_context_prompt_label(true, &state);
        assert!(armed.contains("FIRE MKT SHORT"));
        assert!(armed.contains("[A] SAFE"));

        state.hud_order_kind = HudOrderKind::Limit;
        assert!(hud_context_prompt_label(true, &state).contains("FIRE LIMIT"));
    }

    #[test]
    fn idle_fuse_fraction_is_full_while_hovering_and_drains_when_idle() {
        let mut chart = CandlestickChart::new(1);
        chart.set_crosshair_style(crate::config::ChartCrosshairStyle::Hud);
        chart.set_hud_armed_at(true, 1_000);
        chart.set_clock_now_ms(8_500);

        assert_eq!(chart.hud_idle_fuse_fraction(), 1.0);

        chart.record_hud_activity(1_000, false);
        assert_eq!(chart.hud_idle_fuse_fraction(), 0.5);

        chart.set_clock_now_ms(60_000);
        assert_eq!(chart.hud_idle_fuse_fraction(), 0.0);
    }

    #[test]
    fn candle_progress_tracks_elapsed_fraction_of_the_open_candle() {
        let mut chart = CandlestickChart::new(1);
        chart.timeframe = crate::timeframe::Timeframe::H1;
        chart.candles = vec![crate::api::Candle::test_ohlcv(
            0,
            3_599_999,
            [100.0, 101.0, 99.0, 100.5],
            10.0,
        )];

        chart.set_clock_now_ms(900_000);
        assert_eq!(chart.hud_candle_progress(), Some(0.25));

        chart.set_clock_now_ms(3_600_000);
        assert_eq!(chart.hud_candle_progress(), Some(1.0));

        chart.candles.clear();
        assert_eq!(chart.hud_candle_progress(), None);
    }

    #[test]
    fn station_size_text_shrinks_and_drops_the_unit_for_long_values() {
        assert_eq!(hud_station_size_text(3), (15.0, true));
        assert_eq!(hud_station_size_text(7), (15.0, true));
        assert_eq!(hud_station_size_text(8), (11.0, true));
        assert_eq!(hud_station_size_text(10), (11.0, true));
        assert_eq!(hud_station_size_text(13), (11.0, false));
    }

    #[test]
    fn compact_summary_reflects_mode_and_side() {
        let mut state = ChartState {
            hud_size_input: "2.5".to_string(),
            ..ChartState::default()
        };
        assert_eq!(hud_station_compact_summary(&state), "L>LIMIT 2.5");
        state.hud_order_kind = HudOrderKind::Market;
        state.hud_market_side = HudMarketSide::Short;
        assert_eq!(hud_station_compact_summary(&state), "M>SHORT 2.5");
    }
}
