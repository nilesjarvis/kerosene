use super::drawing::tooltip_origin;
use crate::denomination::DisplayDenominationContext;
use crate::journal::AggregatedTrade;
use crate::message::Message;

use iced::widget::{canvas, container, text};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme};

const RECENT_OUTCOME_TILE_LIMIT: usize = 56;
const OUTCOME_TILE_TOOLTIP_WIDTH: f32 = 128.0;
const OUTCOME_TILE_TOOLTIP_HEIGHT: f32 = 34.0;

// ---------------------------------------------------------------------------
// Recent Outcome Strip
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JournalTradeOutcome {
    Win,
    Loss,
    Flat,
}

#[derive(Debug, Clone)]
pub(crate) struct JournalOutcomeTile {
    pub(crate) outcome: JournalTradeOutcome,
    pub(crate) pnl: f64,
    pub(crate) trade_type: String,
}

pub(super) fn journal_outcome_strip(
    filtered_trades: &[&AggregatedTrade],
    denomination: DisplayDenominationContext,
    include_fees: bool,
) -> Element<'static, Message> {
    let tiles = journal_recent_trade_outcome_tiles(filtered_trades, include_fees);
    if tiles.is_empty() {
        return container(
            text("No closed trades")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
        )
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

pub(super) fn journal_recent_trade_outcome_tiles(
    trades: &[&AggregatedTrade],
    include_fees: bool,
) -> Vec<JournalOutcomeTile> {
    let mut tiles = trades
        .iter()
        .filter(|trade| journal_trade_counts_toward_win_rate(trade))
        .map(|trade| {
            let pnl = if include_fees {
                trade.pnl - trade.fee
            } else {
                trade.pnl
            };
            let outcome = if pnl > 0.0 {
                JournalTradeOutcome::Win
            } else if pnl < 0.0 {
                JournalTradeOutcome::Loss
            } else {
                JournalTradeOutcome::Flat
            };
            (
                trade.end_time.unwrap_or(trade.start_time),
                JournalOutcomeTile {
                    outcome,
                    pnl,
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

    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return vec![frame.into_geometry()];
    }

    let rows = 4;
    let cols = 14;
    let tile_size = 8.0;
    let gap = 2.0;

    let grid_h = rows as f32 * tile_size + (rows - 1) as f32 * gap;

    let start_x = 0.0;
    let start_y = (bounds.height - grid_h).max(0.0) / 2.0;

    let cursor_pos = cursor.position_in(bounds);
    let mut hovered_tile = None;
    let mut hovered_origin = Point::ORIGIN;

    let max_abs_pnl = tiles.iter().map(|t| t.pnl.abs()).fold(0.0_f64, f64::max);

    for i in 0..(rows * cols) {
        let col = i / rows;
        let row = i % rows;

        let x = start_x + col as f32 * (tile_size + gap);
        let y = start_y + row as f32 * (tile_size + gap);
        let tile_origin = Point::new(x, y);
        let tile_rect = Size::new(tile_size, tile_size);

        if let Some(tile) = tiles.get(i) {
            let intensity = if max_abs_pnl > 0.0 {
                (tile.pnl.abs() / max_abs_pnl) as f32
            } else {
                0.0
            };

            let level = if tile.outcome == JournalTradeOutcome::Flat {
                0.3
            } else if intensity <= 0.25 {
                0.4
            } else if intensity <= 0.50 {
                0.6
            } else if intensity <= 0.75 {
                0.8
            } else {
                1.0
            };

            let fill = outcome_color(tile.outcome, theme);
            frame.fill_rectangle(tile_origin, tile_rect, Color { a: level, ..fill });
            let outline = canvas::Path::rectangle(tile_origin, tile_rect);
            frame.stroke(
                &outline,
                canvas::Stroke::default()
                    .with_color(Color {
                        a: level * 0.5,
                        ..fill
                    })
                    .with_width(1.0),
            );

            if let Some(pos) = cursor_pos
                && pos.x >= x
                && pos.x <= x + tile_size
                && pos.y >= y
                && pos.y <= y + tile_size
            {
                hovered_tile = Some(tile);
                hovered_origin = Point::new(x + tile_size / 2.0, y + tile_size);
            }
        } else {
            // Empty tile
            let empty_color = Color {
                a: 0.05,
                ..theme.palette().text
            };
            frame.fill_rectangle(tile_origin, tile_rect, empty_color);
            let outline = canvas::Path::rectangle(tile_origin, tile_rect);
            frame.stroke(
                &outline,
                canvas::Stroke::default()
                    .with_color(Color {
                        a: 0.10,
                        ..theme.palette().text
                    })
                    .with_width(1.0),
            );
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
            font: crate::app_fonts::monospace_font(),
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
