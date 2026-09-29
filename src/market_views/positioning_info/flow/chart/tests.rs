use super::super::super::metrics::{format_signed_size, positioning_flow_data};
use super::super::POSITIONING_FLOW_ROW_LIMIT;
use super::formatting::compact_size;
use super::*;
use crate::hyperdash_api::PerpDeltaEntry;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::renderer::Headless;
use iced::widget::canvas::Program as _;
use iced::{Font, Pixels, Size};

fn chart(mark: Option<f64>, empty: bool) -> PositioningFlowChart {
    let deltas = if empty {
        Vec::new()
    } else {
        [(100.0, 5.0), (10.0, -50.0), (30.0, 50.0)]
            .into_iter()
            .enumerate()
            .map(|(index, (current, delta))| PerpDeltaEntry {
                address: format!("synthetic-trader-{index}"),
                current,
                delta,
            })
            .collect()
    };
    let data = positioning_flow_data(&deltas, mark, POSITIONING_FLOW_ROW_LIMIT);
    PositioningFlowChart::new(&data, &DisplayDenominationContext::usd())
}

#[test]
fn flow_labels_preserve_signs_rounding_and_units() {
    for (value, expected) in [
        (f64::NAN, "-"),
        (f64::INFINITY, "-"),
        (f64::NEG_INFINITY, "-"),
        (-0.0, "0.0000"),
        (0.00001, "+0.0000"),
        (-0.00001, "-0.0000"),
        (1.25, "+1.25"),
        (-125.0, "-125.0"),
        (10_000.0, "+10.0K"),
        (1_000_000.0, "+1.0M"),
    ] {
        assert_eq!(format_signed_size(value, true), expected);
    }
    for (value, expected) in [
        (0.0, "0"),
        (0.001, "0"),
        (12.5, "12.5"),
        (999.0, "999"),
        (1_000.0, "1k"),
        (1_250.0, "1.2k"),
        (999_999.0, "1000k"),
        (1_000_000.0, "1m"),
        (1_000_000_000.0, "1b"),
    ] {
        assert_eq!(compact_size(value), expected);
        assert_eq!(compact_size(-value), expected);
    }
    let usd = chart(Some(10.0), false);
    assert_eq!(usd.content_height(), 112.0);
    assert_eq!(
        (&*usd.long_label, &*usd.short_label, &*usd.net_label),
        ("$550", "$500", "NET + $50")
    );
    assert_eq!(usd.rows[0].value_text, "-$500");
    assert_eq!(usd.rows[0].tooltip[0], ("Kind".into(), "Cut".into()));
    assert_eq!(usd.rows[0].tooltip[1], ("Prev".into(), "+60.00".into()));
    assert_eq!(usd.rows[0].tooltip.len(), 6);
    let raw = chart(None, false);
    assert_eq!(raw.net_label, "NET +5");
    assert_eq!(raw.rows[0].value_text, "-50");
    assert_eq!(raw.rows[0].tooltip.len(), 4);
}

#[test]
fn flow_layout_and_hover_preserve_boundaries_and_animation() {
    for width in [239.0, 240.0, 339.0, 340.0, 479.0, 480.0] {
        let layout = FlowLayout::new(width);
        assert_eq!(layout.show_value, width >= 240.0);
        assert_eq!(PositioningFlowChart::labels_visible(width), width >= 340.0);
        assert_eq!(layout.show_tag, width >= 480.0);
        assert!(layout.half_width >= 2.0);
    }
    let chart = chart(Some(10.0), false);
    let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(600.0, 300.0));
    for (y, expected) in [
        (39.0, None),
        (40.0, Some(0)),
        (62.0, Some(0)),
        (63.0, None),
        (65.0, Some(1)),
        (112.0, Some(2)),
        (115.0, None),
    ] {
        let cursor = mouse::Cursor::Available(Point::new(100.0, y + 20.0));
        assert_eq!(chart.row_index_at(bounds, cursor), expected);
    }
    assert_eq!(chart.row_index_at(bounds, mouse::Cursor::Unavailable), None);
    let mut state = PositioningFlowState::default();
    let cursor = mouse::Cursor::Available(Point::new(100.0, 61.0));
    let moved = iced::Event::Mouse(mouse::Event::CursorMoved {
        position: Point::new(100.0, 61.0),
    });
    let (_, redraw, status) = chart
        .update(&mut state, &moved, bounds, cursor)
        .expect("hover redraw")
        .into_inner();
    assert_eq!(redraw, iced::window::RedrawRequest::NextFrame);
    assert_eq!(status, iced::event::Status::Ignored);
    assert_eq!(state.hovered, Some(0));
    assert!(chart.update(&mut state, &moved, bounds, cursor).is_none());
    let now = time::Instant::now();
    let redraw = iced::Event::Window(iced::window::Event::RedrawRequested(now));
    let (_, requested, _) = chart
        .update(&mut state, &redraw, bounds, cursor)
        .expect("animation redraw")
        .into_inner();
    assert_eq!(
        requested,
        iced::window::RedrawRequest::At(now + time::Duration::from_millis(16))
    );
    assert_eq!(state.tooltip_progress, 0.34);
    assert!(state.tooltip_visibility() > 0.0);
    for _ in 0..30 {
        chart.update(&mut state, &redraw, bounds, cursor);
    }
    assert_eq!(state.tooltip_progress, 1.0);
    assert!(!state.tooltip_animation_active());
    assert!(state.set_hovered(Some(1)));
    assert_eq!(state.tooltip_progress, 0.25);
    let left = iced::Event::Mouse(mouse::Event::CursorLeft);
    assert!(chart.update(&mut state, &left, bounds, cursor).is_some());
    assert_eq!(state.hovered, None);
    assert_eq!(state.tooltip_visibility(), 0.0);
}

#[tokio::test]
async fn flow_canvas_renders_responsive_units_empty_and_hover_states() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        for width in [200, 300, 400, 600] {
            let height = 300;
            let bounds = Rectangle::with_size(Size::new(width as f32, height as f32));
            for (case, mark, empty) in [
                ("usd", Some(10.0), false),
                ("size", None, false),
                ("empty", None, true),
            ] {
                for (hover, progress, actions) in [
                    ("none", 0.0, false),
                    ("tooltip", 0.5, false),
                    ("actions", 1.0, true),
                ] {
                    let mut chart = chart(mark, empty);
                    if actions {
                        chart.hovered_action_key = Some(String::new());
                    }
                    let state = PositioningFlowState {
                        hovered: (progress > 0.0).then_some(0),
                        tooltip_progress: progress,
                    };
                    let geometry = chart.draw(
                        &state,
                        &renderer,
                        &theme,
                        bounds,
                        mouse::Cursor::Unavailable,
                    );
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
                    if let Some(directory) =
                        std::env::var_os("KEROSENE_POSITIONING_FLOW_PREVIEW_DIR")
                    {
                        let directory = std::path::PathBuf::from(directory);
                        std::fs::create_dir_all(&directory).expect("preview directory");
                        image::save_buffer(
                            directory.join(format!("{theme_name}-{width}-{case}-{hover}.png")),
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
