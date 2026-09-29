use super::super::summary::WinRateBullet;
use super::drawing::tooltip_origin;
use super::*;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::renderer::Headless;
use iced::widget::canvas::Program as _;
use iced::{Font, Pixels, Size};

fn lane(compact: bool, empty: bool) -> SessionLane {
    let mut rows = [
        ("Mon", 8, 1.25, 75.0, Some(1.5), true),
        ("Tue", 2, -0.75, 40.0, Some(0.5), false),
        ("Wed", 0, 0.0, 0.0, None, false),
        ("Asia", 1, 0.0, 0.0, None, false),
        ("London", 10, -0.5, 25.0, None, false),
    ]
    .into_iter()
    .map(
        |(label, sample_count, average_return_pct, win_rate_pct, dispersion_pct, is_best)| {
            LaneRow {
                label,
                sample_count,
                average_return_pct,
                win_rate_pct,
                dispersion_pct,
                is_best,
            }
        },
    )
    .collect::<Vec<_>>();
    let session_rows = rows.split_off(3);
    SessionLane {
        weekday_rows: if empty { Vec::new() } else { rows },
        session_rows: if empty { Vec::new() } else { session_rows },
        scale_max: 1.25,
        compact,
    }
}

#[test]
fn session_lane_preserves_row_boundaries_hover_and_tooltip_clamping() {
    let lane = lane(false, false);
    assert_eq!(lane_height(3, 2), 168.0);
    assert_eq!(lane.row_y_center(0), 34.0);
    assert_eq!(lane.row_y_center(3), 127.0);
    assert!(lane.row_at(5).is_none());
    let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(600.0, 240.0));
    for (y, expected) in [
        (22.0, None),
        (23.0, Some(0)),
        (45.0, Some(0)),
        (45.5, Some(1)),
        (89.0, Some(2)),
        (90.0, None),
        (115.0, None),
        (116.0, Some(3)),
        (138.0, Some(3)),
        (138.5, Some(4)),
        (161.0, None),
    ] {
        let cursor = mouse::Cursor::Available(Point::new(100.0, y + 20.0));
        assert_eq!(lane.row_at_cursor(bounds, cursor), expected);
    }
    assert_eq!(lane.row_at_cursor(bounds, mouse::Cursor::Unavailable), None);
    let mut state = SessionLaneState::default();
    let position = Point::new(100.0, 54.0);
    let cursor = mouse::Cursor::Available(position);
    let moved = iced::Event::Mouse(mouse::Event::CursorMoved { position });
    let (message, redraw, status) = lane
        .update(&mut state, &moved, bounds, cursor)
        .expect("hover redraw")
        .into_inner();
    assert!(message.is_none());
    assert_eq!(redraw, iced::window::RedrawRequest::NextFrame);
    assert_eq!(status, iced::event::Status::Ignored);
    assert_eq!(state.hovered, Some(0));
    assert!(lane.update(&mut state, &moved, bounds, cursor).is_none());
    assert!(
        lane.update(
            &mut state,
            &iced::Event::Mouse(mouse::Event::CursorLeft),
            bounds,
            cursor
        )
        .is_some()
    );
    assert_eq!(state.hovered, None);

    for width in [160.0, 359.0, 360.0, 600.0] {
        let compact = width < 360.0;
        let layout = LaneLayout::new(Size::new(width, 240.0), compact);
        assert_eq!(layout.value_right.is_none(), compact);
        assert_eq!(layout.n_right.is_none(), compact);
        assert!(layout.half_w >= 1.0);
        assert!(layout.bullet_left >= layout.bar_left);
    }
    assert_eq!(
        tooltip_origin(Point::new(10.0, 10.0), Size::new(600.0, 400.0)),
        Point::new(18.0, 18.0)
    );
    assert_eq!(
        tooltip_origin(Point::new(599.0, 399.0), Size::new(600.0, 400.0)),
        Point::new(401.0, 307.0)
    );
    assert_eq!(
        tooltip_origin(Point::new(80.0, 45.0), Size::new(160.0, 90.0)),
        Point::new(0.0, 0.0)
    );
}

fn render_preview(
    renderer: &mut Renderer,
    theme: &Theme,
    name: &str,
    size: Size<u32>,
    geometry: Vec<canvas::Geometry>,
    populated: bool,
) {
    iced::advanced::Renderer::reset(
        renderer,
        Rectangle::with_size(Size::new(size.width as f32, size.height as f32)),
    );
    for geometry in geometry {
        renderer.draw_geometry(geometry);
    }
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    assert_eq!(pixels.len(), size.width as usize * size.height as usize * 4);
    assert_eq!(
        pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]),
        populated
    );
    if let Some(directory) = std::env::var_os("KEROSENE_SESSION_VIEW_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("preview directory");
        image::save_buffer(
            directory.join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            image::ColorType::Rgba8,
        )
        .expect("synthetic preview");
    }
}

#[tokio::test]
async fn session_canvases_render_compact_full_empty_and_hover_states() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        for (width, height) in [(160, 90), (359, 210), (360, 210), (600, 240)] {
            let size = Size::new(width, height);
            let bounds = Rectangle::with_size(Size::new(width as f32, height as f32));
            for empty in [false, true] {
                let lane = lane(width < 360, empty);
                for (hover_name, hovered) in
                    [("none", None), ("weekday", Some(0)), ("session", Some(4))]
                {
                    let geometry = lane.draw(
                        &SessionLaneState { hovered },
                        &renderer,
                        &theme,
                        bounds,
                        mouse::Cursor::Unavailable,
                    );
                    render_preview(
                        &mut renderer,
                        &theme,
                        &format!("{theme_name}-lane-{width}-{empty}-{hover_name}"),
                        size,
                        geometry,
                        !empty,
                    );
                }
            }
        }
        for ratio in [-0.5, 0.0, 0.5, 1.0, 1.5] {
            let bullet = WinRateBullet { ratio };
            let geometry = bullet.draw(
                &(),
                &renderer,
                &theme,
                Rectangle::with_size(Size::new(34.0, 8.0)),
                mouse::Cursor::Unavailable,
            );
            render_preview(
                &mut renderer,
                &theme,
                &format!("{theme_name}-bullet-{ratio}"),
                Size::new(34, 8),
                geometry,
                true,
            );
        }
    }
}
