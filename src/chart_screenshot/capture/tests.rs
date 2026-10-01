use super::*;
use crate::api::Candle;
use crate::chart::{CandlestickChart, ChartState};
use crate::chart_screenshot::widget::{CaptureChartCanvas, ScreenshotCanvas};
use crate::message::Message;
use iced::advanced::widget::{Id, Operation, operation, tree};
use iced::advanced::{Layout, layout};
use iced::widget::canvas::Program;
use iced::{Element, Event, Point, Size};

fn synthetic_chart() -> CandlestickChart {
    let mut chart = CandlestickChart::new(7);
    chart.candles = (0..120)
        .map(|i| {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let time = 1_700_000_000_000 + i * 3_600_000;
            Candle::test_ohlcv(
                time,
                time + 3_599_999,
                [price, price + 4.0, price - 2.0, price + 2.0],
                1000.0 + i as f64 * 10.0,
            )
        })
        .collect();
    chart.request_view_reset();
    chart
}

fn positioned_state(chart: &CandlestickChart, bounds: Rectangle) -> ChartState {
    let mut state = ChartState::default();
    let cursor = mouse::Cursor::Available(Point::new(bounds.x + 200.0, bounds.y + 100.0));
    // Process the reset, zoom, pan into history and manually scale the Y axis.
    chart.update(
        &mut state,
        &Event::Mouse(mouse::Event::CursorEntered),
        bounds,
        cursor,
    );
    chart.update(
        &mut state,
        &Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 3.0 },
        }),
        bounds,
        cursor,
    );
    chart.update(
        &mut state,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        bounds,
        cursor,
    );
    let panned = mouse::Cursor::Available(Point::new(bounds.x + 290.0, bounds.y + 100.0));
    chart.update(
        &mut state,
        &Event::Mouse(mouse::Event::CursorMoved {
            position: Point::ORIGIN,
        }),
        bounds,
        panned,
    );
    chart.update(
        &mut state,
        &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        bounds,
        panned,
    );
    chart.update(
        &mut state,
        &Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: -2.0 },
        }),
        bounds,
        mouse::Cursor::Available(Point::new(bounds.x + bounds.width - 10.0, bounds.y + 100.0)),
    );
    chart.update(
        &mut state,
        &Event::Mouse(mouse::Event::CursorLeft),
        bounds,
        mouse::Cursor::Unavailable,
    );
    state
}

#[tokio::test]
async fn export_pixels_match_live_canvas_at_higher_density() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("software renderer");

    for panels in [false, true] {
        let mut chart = synthetic_chart();
        chart.macro_indicators.show_funding_rate = panels;
        chart.macro_indicators.show_session_indicator = panels;
        chart.funding_panel_height = 83.0;
        chart.session_panel_height = 57.0;
        chart.inverted = panels;
        let theme = if panels { Theme::Light } else { Theme::Dark };
        let bounds = Rectangle {
            x: 53.0,
            y: 91.0,
            width: 640.25,
            height: 440.5,
        };
        let state = positioned_state(&chart, bounds);
        let target = Id::from("synthetic-chart");
        let mut canvas: Element<'_, Message> = ScreenshotCanvas::new(&chart, target.clone()).into();
        let mut tree = tree::Tree::new(&canvas);
        tree.state = tree::State::new(state);
        let node = layout::Node::new(bounds.size()).move_to(bounds.position());
        let mut operation = CaptureChartCanvas::new(target);
        canvas.as_widget_mut().operate(
            &mut tree,
            Layout::new(&node),
            &renderer,
            &mut operation::black_box(&mut operation),
        );
        let operation::Outcome::Some(Some(snapshot)) = operation.finish() else {
            panic!("canvas snapshot");
        };
        let state = tree.state.downcast_ref::<ChartState>();
        let resolution = ExportResolution::new(bounds.size()).expect("resolution");
        let background = theme.palette().background;
        let captured_at = Local::now();
        let request = ChartScreenshotRenderRequest {
            symbol: "SYNTHETIC".to_string(),
            timeframe: "1H".to_string(),
            chart: chart.snapshot_for_export(),
            background_color: background,
            captured_at,
            theme: theme.clone(),
        };

        // Reference: the actual live Program::draw, with its existing state and
        // fractional layout, rasterized at the requested output density.
        iced::advanced::Renderer::reset(&mut renderer, Rectangle::with_size(bounds.size()));
        for geometry in chart.draw(state, &renderer, &theme, bounds, mouse::Cursor::Unavailable) {
            renderer.draw_geometry(geometry);
        }
        let expected = renderer.screenshot(resolution.size, resolution.scale_factor, background);
        iced::advanced::Renderer::reset(&mut renderer, Rectangle::with_size(bounds.size()));
        let result = render_with_renderer(request, snapshot, resolution, &mut renderer)
            .expect("rendered export");
        assert_eq!(Size::new(result.width, result.height), resolution.size);
        assert_eq!(result.captured_at, captured_at);
        assert_eq!(result.rgba.len(), expected.len());
        assert!(
            result.rgba.as_ref() == expected,
            "export reflowed the live canvas"
        );
        assert!(
            expected
                .chunks_exact(4)
                .any(|pixel| pixel != &expected[..4]),
            "chart is not blank"
        );
        let decoded = image::load_from_memory(&result.png)
            .expect("valid PNG")
            .to_rgba8();
        assert!(decoded.as_raw().as_slice() == result.rgba.as_ref());

        if let Some(directory) = std::env::var_os("KEROSENE_SCREENSHOT_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).expect("preview directory");
            std::fs::write(
                directory.join(format!("chart-panels-{panels}.png")),
                &result.png,
            )
            .expect("synthetic preview");
        }
    }
}
