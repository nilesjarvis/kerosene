use super::drawing::tooltip_origin;
use crate::denomination::DisplayDenominationContext;
use crate::journal::AggregatedTrade;
use crate::message::Message;

use iced::widget::{canvas, container, text};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme};

const RECENT_OUTCOME_TILE_LIMIT: usize = 56;
const OUTCOME_TILE_GAP: f32 = 4.0;
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
) -> Element<'static, Message> {
    let tiles = journal_recent_trade_outcome_tiles(filtered_trades);
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
) -> Vec<JournalOutcomeTile> {
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
