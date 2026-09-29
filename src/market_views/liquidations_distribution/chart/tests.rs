use super::*;
use crate::hyperdash_api::LiquidationLevel;
use crate::liquidations_distribution_state::LiquidationDistributionRequest;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::renderer::Headless;
use iced::widget::canvas::Program as _;
use iced::{Font, Pixels, Size};

fn data() -> LiquidationDistributionData {
    LiquidationDistributionData::from_level(
        LiquidationDistributionRequest::new(
            "BTC".to_string(),
            "BTC".to_string(),
            "BTC".to_string(),
            100.0,
            0.0,
            200.0,
            1,
        ),
        LiquidationLevel {
            coin: "BTC".to_string(),
            min: 0.0,
            max: 200.0,
            liquidations: serde_json::from_value(serde_json::json!([
                { "amount": 2.0, "price": 80.0 },
                { "amount": -3.0, "price": 120.0 }
            ]))
            .expect("fixture liquidations"),
        },
        1_000,
    )
}

#[test]
fn wheel_zoom_preserves_anchor_capture_and_scroll_direction() {
    let data = data();
    let chart = LiquidationsDistributionChart {
        data: &data,
        denomination: DisplayDenominationContext::usd(),
        zoom: 1.0,
        zoom_center_price: None,
    };
    let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(600.0, 400.0));
    for (delta, factor) in [
        (mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 }, 1.25),
        (mouse::ScrollDelta::Pixels { x: 0.0, y: 28.0 }, 1.25),
        (mouse::ScrollDelta::Lines { x: 0.0, y: -3.0 }, 0.8),
        (mouse::ScrollDelta::Pixels { x: 0.0, y: -56.0 }, 0.8),
    ] {
        let event = iced::Event::Mouse(mouse::Event::WheelScrolled { delta });
        for (cursor, expected_anchor) in [
            (Point::new(305.0, 120.0), Some((100.0, 0.5))),
            (Point::new(68.0, 34.0), Some((0.0, 0.0))),
            (Point::new(12.0, 120.0), None),
        ] {
            let action = chart
                .update(&mut (), &event, bounds, mouse::Cursor::Available(cursor))
                .expect("wheel over canvas");
            let (message, _, status) = action.into_inner();
            let Some(Message::LiquidationsDistributionZoomed {
                factor: actual,
                anchor,
            }) = message
            else {
                panic!("expected zoom message");
            };
            assert_eq!(actual, factor);
            assert_eq!(
                anchor.map(|anchor| (anchor.price, anchor.fraction)),
                expected_anchor
            );
            assert_eq!(status, iced::event::Status::Captured);
        }
        assert!(
            chart
                .update(&mut (), &event, bounds, mouse::Cursor::Unavailable)
                .is_none()
        );
    }
    let zero = iced::Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Lines { x: 1.0, y: 0.0 },
    });
    assert!(
        chart
            .update(
                &mut (),
                &zero,
                bounds,
                mouse::Cursor::Available(Point::new(305.0, 120.0))
            )
            .is_none()
    );
}

#[test]
fn visible_geometry_preserves_bounds_ties_and_single_point_width() {
    let plot = PlotArea::new(
        Rectangle::with_size(Size::new(600.0, 400.0)),
        ChartMargins::for_width(600.0),
        (50.0, 150.0),
    );
    assert_eq!(plot.price_to_x(50.0), 58.0);
    assert_eq!(plot.price_to_x(150.0), 532.0);
    assert_eq!(plot.x_to_price(-100.0), 50.0);
    assert_eq!(plot.x_to_price(1_000.0), 150.0);
    let mut data = data();
    data.points = [40.0, 60.0, 100.0, 100.0, 140.0, 160.0]
        .into_iter()
        .enumerate()
        .map(|(index, price)| LiquidationDistributionPoint {
            price,
            long_usd: index as f64,
            short_usd: 0.0,
            cumulative_long_usd: index as f64 * 10.0,
            cumulative_short_usd: 0.0,
        })
        .collect();
    assert_eq!(visible_max_bucket_usd(&data, &plot), Some(4.0));
    assert_eq!(visible_max_cumulative_usd(&data, &plot), Some(40.0));
    assert_eq!(visible_bucket_width(&data.points, &plot), 1.0);
    let nearest =
        nearest_distribution_point(&data, &plot, plot.price_to_x(100.0)).expect("visible point");
    assert_eq!(nearest.long_usd, 2.0);
    assert_eq!(visible_bucket_width(&data.points[1..2], &plot), f32::MAX);
    data.points.clear();
    assert!(nearest_distribution_point(&data, &plot, 100.0).is_none());
    assert_eq!(visible_max_bucket_usd(&data, &plot), None);
}

#[tokio::test]
async fn distribution_canvas_renders_synthetic_sizes_zoom_and_hover() {
    let data = data();
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        for (width, height) in [(720, 360), (320, 260), (170, 100)] {
            let size = Size::new(width as f32, height as f32);
            let bounds = Rectangle::with_size(size);
            for zoom in [1.0, 4.0] {
                let chart = LiquidationsDistributionChart {
                    data: &data,
                    denomination: DisplayDenominationContext::usd(),
                    zoom,
                    zoom_center_price: Some(100.0),
                };
                for hover in [false, true] {
                    let cursor = if hover {
                        mouse::Cursor::Available(Point::new(size.width * 0.4, 80.0))
                    } else {
                        mouse::Cursor::Unavailable
                    };
                    let geometry = chart.draw(&(), &renderer, &theme, bounds, cursor);
                    iced::advanced::Renderer::reset(&mut renderer, bounds);
                    for geometry in geometry {
                        renderer.draw_geometry(geometry);
                    }
                    let pixels = renderer.screenshot(
                        Size::new(width, height),
                        1.0,
                        theme.palette().background,
                    );
                    assert_eq!(pixels.len(), width as usize * height as usize * 4);
                    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
                    if let Some(directory) = std::env::var_os("KEROSENE_DISTRIBUTION_PREVIEW_DIR") {
                        let directory = std::path::PathBuf::from(directory);
                        std::fs::create_dir_all(&directory).expect("preview directory");
                        image::save_buffer(
                            directory.join(format!("{theme_name}-{width}-{zoom}-{hover}.png")),
                            &pixels,
                            width,
                            height,
                            image::ColorType::Rgba8,
                        )
                        .expect("synthetic preview");
                    }
                }
            }
        }
    }
}
