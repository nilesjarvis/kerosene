use super::*;
use crate::chart::state::DragKind;
use crate::chart::{OrderOverlay, OrderOverlayPendingState, PositionOverlay};
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::renderer::Headless;
use iced::{Font, Pixels, Rectangle, Renderer, Size};

const WIDTH: u32 = 500;
const HEIGHT: u32 = 260;
const CHART_W: f32 = 400.0;
const PRICE_H: f32 = 240.0;

fn overlay_chart(case: usize) -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.candles = vec![Candle::test_ohlcv(
        0,
        60_000,
        [99.0, 110.0, 90.0, 100.0],
        1.0,
    )];
    chart.macro_indicators.show_high_low = false;
    chart.show_trade_markers = false;
    chart.order_line_phase = [0.0, 8.5, f32::NAN][case];
    chart.hover_order_cancel_oid = Some(1);
    chart.order_cancel_hover_progress = [0.0, 0.5, 1.0][case];
    if case == 2 {
        chart.series_style = ChartSeriesStyle::Line;
    }
    chart.active_position = Some(PositionOverlay {
        entry_px: 100.0,
        szi: if case == 1 { -2.0 } else { 2.0 },
        liquidation_px: Some(80.0),
    });
    chart.active_orders = [
        (80.0, true, false, None),
        (90.0, false, false, None),
        (101.0, true, true, None),
        (104.0, false, false, None),
        (99.0, true, false, Some(OrderOverlayPendingState::Placing)),
        (
            102.0,
            false,
            false,
            Some(OrderOverlayPendingState::Cancelling),
        ),
        (
            100.0,
            true,
            false,
            Some(OrderOverlayPendingState::Modifying),
        ),
        (f64::NAN, true, false, None),
        (500.0, false, false, None),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(index, (limit_px, is_buy, is_moving, pending_state))| OrderOverlay {
            coin: "SYNTH".into(),
            limit_px,
            sz: 1.25,
            is_buy,
            oid: index as u64 + 1,
            is_moving,
            pending_state,
        },
    )
    .collect();
    chart
}

fn render(
    chart: &CandlestickChart,
    state: &ChartState,
    renderer: &mut Renderer,
    theme: &Theme,
    fisheye: ChartFisheye,
) -> Vec<u8> {
    let bounds = Rectangle::with_size(Size::new(WIDTH as f32, HEIGHT as f32));
    let mut frame = canvas::Frame::new(renderer, bounds.size());
    let price_to_y = |price| (140.0 - price) as f32 * 3.0;
    let right_axis_badges =
        chart.right_axis_badge_layout(state, PRICE_H, 80.0, CHART_W, fisheye, &price_to_y);
    chart.draw_trading_overlays(&mut TradingOverlayContext {
        frame: &mut frame,
        state,
        theme,
        chart_w: CHART_W,
        price_h: PRICE_H,
        price_range: 80.0,
        candles: &chart.candles,
        first_vis: 0,
        last_vis: 0,
        candle_bull_color: theme.palette().success,
        candle_bear_color: theme.palette().danger,
        right_axis_badges: &right_axis_badges,
        fisheye,
        price_to_y: &price_to_y,
        idx_to_cx: &|_| 100.0,
    });
    iced::advanced::Renderer::reset(renderer, bounds);
    renderer.draw_geometry(frame.into_geometry());
    let pixels = renderer.screenshot(Size::new(WIDTH, HEIGHT), 1.0, theme.palette().background);
    assert_eq!(pixels.len(), (WIDTH * HEIGHT * 4) as usize);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    pixels
}

fn save_preview(name: &str, pixels: &[u8]) {
    // Synthetic previews support direct before/after image comparisons.
    if let Some(directory) = std::env::var_os("KEROSENE_OVERLAY_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("preview directory");
        image::save_buffer(
            directory.join(format!("{name}.png")),
            pixels,
            WIDTH,
            HEIGHT,
            image::ColorType::Rgba8,
        )
        .expect("save synthetic overlay preview");
    }
}

#[tokio::test]
async fn trading_overlays_render_pending_dragged_and_private_states() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for theme in [Theme::Dark, Theme::Light] {
        for distorted in [false, true] {
            let fisheye = ChartFisheye::new(distorted, 0.7, CHART_W, PRICE_H)
                .with_chromatic(distorted, 0.5)
                .with_edge_blur(distorted, 0.5);
            for case in 0..3 {
                let mut chart = overlay_chart(case);
                let state = ChartState {
                    drag: Some(DragKind::MoveOrder { oid: 4 }),
                    drag_order_new_price: Some(100.5),
                    ..Default::default()
                };
                let visible = render(&chart, &state, &mut renderer, &theme, fisheye);
                save_preview(&format!("{theme}-{distorted}-{case}-visible"), &visible);

                chart.obscure_position_prices = true;
                let obscure = render(&chart, &state, &mut renderer, &theme, fisheye);
                save_preview(&format!("{theme}-{distorted}-{case}-obscure"), &obscure);
                let position = chart.active_position.take();
                chart.obscure_position_prices = false;
                let without_position = render(&chart, &state, &mut renderer, &theme, fisheye);
                assert!(
                    obscure == without_position,
                    "obscured position leaked into rendering"
                );
                assert!(visible != obscure, "position should render when visible");
                chart.active_position = position;

                chart.hide_positions_and_orders = true;
                let hidden = render(&chart, &state, &mut renderer, &theme, fisheye);
                save_preview(&format!("{theme}-{distorted}-{case}-hidden"), &hidden);
                chart.active_orders.clear();
                chart.active_position = None;
                chart.hide_positions_and_orders = false;
                let bare = render(&chart, &state, &mut renderer, &theme, fisheye);
                assert!(
                    hidden == bare,
                    "hidden positions or orders leaked into rendering"
                );
                assert!(
                    obscure != hidden,
                    "orders should render with position prices obscured"
                );
            }
        }
    }
}
