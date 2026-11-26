use super::super::{canvas::JournalSnapshotCanvas, plot::SnapshotPlot};
use super::*;
use crate::api::Candle;
use crate::chart::{PRICE_AXIS_WIDTH, TradeMarker};
use crate::journal::{JournalTradeSnapshotMetrics, JournalTradeSnapshotStatus};
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::renderer::Headless;
use iced::widget::canvas::Program as _;
use iced::{Font, Pixels, Renderer, Theme};

fn snapshot() -> JournalTradeSnapshot {
    JournalTradeSnapshot {
        trade_id: "synthetic-trade".to_string(),
        coin: "BTC".to_string(),
        source: crate::config::ChartBackfillSource::Hyperliquid,
        coverage: crate::journal::JournalSnapshotCoverage::TwoX,
        timeframe: crate::timeframe::Timeframe::M1,
        trade_start_ms: 1_000_000,
        trade_end_ms: 2_000_000,
        is_open: false,
        live_position: false,
        start_ms: 1_000_000,
        end_ms: 2_000_000,
        candles: vec![
            Candle::test_flat(1_000_000, 100.0),
            Candle::test_flat(1_940_001, 110.0),
        ],
        markers: Vec::new(),
        metrics: JournalTradeSnapshotMetrics {
            timeframe: crate::timeframe::Timeframe::M1,
            candle_count: 2,
            entry_price: 100.0,
            exit_price: 110.0,
            raw_asset_move: 0.1,
            directional_move: 0.1,
            max_adverse_excursion: 0.0,
            max_favorable_excursion: 0.1,
            asset_drawdown: 0.0,
        },
        status: JournalTradeSnapshotStatus::Loaded,
    }
}

#[test]
fn reset_identity_keeps_its_existing_fields_and_exclusions() {
    let original = snapshot();
    let key = snapshot_reset_key(&original);
    let changes: [fn(&mut JournalTradeSnapshot); 7] = [
        |snapshot| snapshot.trade_id.push_str("-other"),
        |snapshot| snapshot.source = crate::config::ChartBackfillSource::Hydromancer,
        |snapshot| snapshot.coverage = crate::journal::JournalSnapshotCoverage::FourX,
        |snapshot| snapshot.timeframe = crate::timeframe::Timeframe::M5,
        |snapshot| snapshot.start_ms += 1,
        |snapshot| snapshot.end_ms += 1,
        |snapshot| snapshot.candles.push(snapshot.candles[0].clone()),
    ];
    for change in changes {
        let mut changed = original.clone();
        change(&mut changed);
        assert_ne!(snapshot_reset_key(&changed), key);
    }

    let mut changed = original;
    changed.coin = "ETH".to_string();
    changed.trade_start_ms += 1;
    changed.trade_end_ms += 1;
    changed.is_open = true;
    changed.live_position = true;
    changed.candles[0].close = 101.0;
    changed.metrics.entry_price = 99.0;
    changed.markers.push(TradeMarker {
        time_ms: 1_100_000,
        price: 100.0,
        size: 1.0,
        is_buy: true,
    });
    assert_eq!(snapshot_reset_key(&changed), key);
}

#[test]
fn snapshot_interaction_keeps_zoom_pan_reset_and_cursor_exit_behavior() {
    let snapshot = snapshot();
    let mut state = JournalSnapshotCanvasState::default();
    let bounds = Rectangle::new(
        Point::ORIGIN,
        Size::new(400.0 + SNAPSHOT_LEFT_PAD + PRICE_AXIS_WIDTH, 240.0),
    );
    let center = Point::new(206.0, 80.0);
    let wheel = iced::Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
    });
    assert!(
        update_snapshot_interaction(
            &mut state,
            &snapshot,
            &wheel,
            bounds,
            mouse::Cursor::Available(center)
        )
        .is_some()
    );
    assert_eq!(
        (state.view_start_ms, state.view_end_ms),
        (893_200, 2_106_800)
    );

    let press = iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
    assert!(
        update_snapshot_interaction(
            &mut state,
            &snapshot,
            &press,
            bounds,
            mouse::Cursor::Available(center)
        )
        .is_some()
    );
    assert!(state.drag.is_some());
    let moved = Point::new(246.0, 80.0);
    let movement = iced::Event::Mouse(mouse::Event::CursorMoved { position: moved });
    assert!(
        update_snapshot_interaction(
            &mut state,
            &snapshot,
            &movement,
            bounds,
            mouse::Cursor::Available(moved)
        )
        .is_some()
    );
    assert_eq!(
        (state.view_start_ms, state.view_end_ms),
        (771_840, 1_985_440)
    );

    assert!(
        update_snapshot_interaction(
            &mut state,
            &snapshot,
            &movement,
            bounds,
            mouse::Cursor::Unavailable
        )
        .is_some()
    );
    assert!(state.drag.is_none());
    let reset = iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
    assert!(
        update_snapshot_interaction(
            &mut state,
            &snapshot,
            &reset,
            bounds,
            mouse::Cursor::Available(center)
        )
        .is_some()
    );
    assert_eq!(
        (state.view_start_ms, state.view_end_ms),
        (760_000, 2_240_000)
    );

    // A wheel event inside the canvas but outside the plot is captured without zooming.
    assert!(
        update_snapshot_interaction(
            &mut state,
            &snapshot,
            &wheel,
            bounds,
            mouse::Cursor::Available(Point::new(1.0, 80.0))
        )
        .is_some()
    );
    assert_eq!(
        (state.view_start_ms, state.view_end_ms),
        (760_000, 2_240_000)
    );
    let mut replacement = snapshot;
    replacement.start_ms = 500_000;
    assert!(
        update_snapshot_interaction(
            &mut state,
            &replacement,
            &movement,
            bounds,
            mouse::Cursor::Available(center)
        )
        .is_some()
    );
    assert_eq!(
        (state.view_start_ms, state.view_end_ms),
        (260_000, 2_240_000)
    );
    assert_eq!(state.reset_key, snapshot_reset_key(&replacement));
}

#[test]
fn clamp_view_range_allows_empty_time_beyond_loaded_range() {
    let loaded_range = (1_000, 2_000);
    let visual_range = (500, 2_500);

    let (start, end) = clamp_view_range(500, 2_500, loaded_range, visual_range, 100);

    assert_eq!((start, end), (500, 2_500));
}

#[test]
fn clamp_view_range_keeps_some_loaded_context_visible() {
    let loaded_range = (1_000, 2_000);
    let visual_range = (0, 3_000);

    let (start, end) = clamp_view_range(2_200, 2_800, loaded_range, visual_range, 100);

    assert!(ranges_overlap((start, end), loaded_range));
    assert!(end > loaded_range.1);
}

#[test]
fn hover_redraws_without_fills_or_capturing_pointer_movement() {
    let snapshot = snapshot();
    let bounds = Rectangle::new(Point::new(30.0, 10.0), Size::new(400.0, 240.0));
    let position = Point::new(80.0, 100.0);
    let motion = iced::Event::Mouse(mouse::Event::CursorMoved { position });
    let mut state = JournalSnapshotCanvasState::default();
    let (message, redraw, status) = update_snapshot_interaction(
        &mut state,
        &snapshot,
        &motion,
        bounds,
        mouse::Cursor::Available(position),
    )
    .expect("hover requests a frame")
    .into_inner();
    assert!(message.is_none());
    assert_eq!(redraw, iced::window::RedrawRequest::NextFrame);
    assert_eq!(status, iced::event::Status::Ignored);
    assert!(!state.is_dragging());
}

#[tokio::test]
async fn crosshair_and_price_badge_follow_the_pointer_on_a_markerless_chart() {
    let mut snapshot = snapshot();
    snapshot.live_position = true;
    let chart = JournalSnapshotCanvas {
        zone: Default::default(),
        snapshot: &snapshot,
    };
    let state = JournalSnapshotCanvasState::default();
    let bounds = Rectangle::new(Point::new(404.0, 200.0), Size::new(400.0, 240.0));
    let theme = Theme::Dark;
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut render = |point: Option<Point>| {
        let cursor = point.map_or(mouse::Cursor::Unavailable, |p| {
            mouse::Cursor::Available(Point::new(bounds.x + p.x, bounds.y + p.y))
        });
        iced::advanced::Renderer::reset(&mut renderer, Rectangle::with_size(bounds.size()));
        for layer in chart.draw(&state, &renderer, &theme, bounds, cursor) {
            renderer.draw_geometry(layer);
        }
        renderer.screenshot(Size::new(400, 240), 1.0, theme.palette().background)
    };
    let idle = render(None);
    let pixel = |pixels: &[u8], x: usize, y: usize| -> [u8; 4] {
        let offset = (y * 400 + x) * 4;
        pixels[offset..offset + 4].try_into().expect("RGBA pixel")
    };
    for (x, y) in [(75, 30), (300, 220)] {
        let hover = render(Some(Point::new(x as f32 + 0.5, y as f32 + 0.5)));
        // Compare with the idle chart at the same pixels, away from the intersection and text.
        assert!((90..180).any(|row| pixel(&hover, x, row) != pixel(&idle, x, row)));
        assert!((150..180).any(|col| pixel(&hover, col, y) != pixel(&idle, col, y)));
        assert_ne!(pixel(&hover, 334, y), pixel(&idle, 334, y));
    }
    assert!(
        render(Some(Point::new(380.0, 120.0))) == idle,
        "moving onto the price axis clears the crosshair"
    );
}

#[tokio::test]
async fn tooltip_uses_canvas_coordinates_and_hides_while_panning() {
    let mut snapshot = snapshot();
    let candle = &snapshot.candles[0];
    snapshot.markers.push(TradeMarker {
        time_ms: candle.open_time + 1_000,
        price: candle.close,
        size: 1.0,
        is_buy: true,
    });
    let chart = JournalSnapshotCanvas {
        zone: Default::default(),
        snapshot: &snapshot,
    };
    let mut state = JournalSnapshotCanvasState::default();
    let bounds = Rectangle::new(Point::new(404.0, 200.0), Size::new(680.0, 240.0));
    let range = state.view_or_full_range(&snapshot, loaded_time_range(&snapshot));
    let plot = SnapshotPlot::new(bounds.size(), range.0, range.1, &snapshot.candles, []);
    let on_buy = mouse::Cursor::Available(Point::new(
        bounds.x + plot.x_for_time(candle.open_time),
        bounds.y + plot.y_for_price(candle.low) + 7.0,
    ));
    let renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let theme = Theme::Dark;
    let layers = chart.draw(&state, &renderer, &theme, bounds, on_buy);
    assert_eq!(layers.len(), 2, "the tooltip is drawn over the chart");
    for cursor in [
        mouse::Cursor::Unavailable,
        mouse::Cursor::Available(bounds.position()),
    ] {
        assert_eq!(
            chart.draw(&state, &renderer, &theme, bounds, cursor).len(),
            1
        );
    }
    let press = iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
    let _ = chart.update(&mut state, &press, bounds, on_buy);
    assert_eq!(
        chart.draw(&state, &renderer, &theme, bounds, on_buy).len(),
        1
    );
}
