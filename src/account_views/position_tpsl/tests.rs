use super::*;
use crate::account_state::position_tpsl::PositionTpslForm;
use crate::config::KeroseneConfig;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

#[tokio::test]
async fn position_tpsl_modal_renders_in_own_workspace_and_captures_background_clicks() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.connected_address = Some("test-account".into());
    terminal.position_tpsl.form = Some(PositionTpslForm {
        workspace: WorkspaceId::Main,
        visible: true,
        account_address: "test-account".into(),
        coin: "BTC".into(),
        is_long: true,
        take_profit: "110000".into(),
        stop_loss: "90000".into(),
        status: None,
    });
    assert!(
        terminal
            .view_position_tpsl_modal(WorkspaceId::Canvas(3))
            .is_none()
    );
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("renderer");
    let theme = terminal.theme();
    for size in [Size::new(800.0, 600.0), Size::new(340.0, 360.0)] {
        let mut view: Element<'_, Message> = stack![
            button("Background")
                .width(Fill)
                .height(Fill)
                .on_press(Message::NukePositions),
            terminal
                .view_position_tpsl_modal(WorkspaceId::Main)
                .expect("modal"),
        ]
        .into();
        let mut tree = Tree::new(view.as_widget());
        let node =
            view.as_widget_mut()
                .layout(&mut tree, &renderer, &layout::Limits::new(size, size));
        let bounds = Rectangle::with_size(size);
        iced::advanced::Renderer::reset(&mut renderer, bounds);
        view.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style {
                text_color: theme.palette().text,
            },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &bounds,
        );
        let pixels = renderer.screenshot(
            Size::new(size.width as u32, size.height as u32),
            1.0,
            theme.palette().background,
        );
        assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
        if let Some(directory) = std::env::var_os("KEROSENE_TPSL_PREVIEW_DIR") {
            let path = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&path).expect("preview directory");
            image::save_buffer(
                path.join(format!("{}.png", size.width)),
                &pixels,
                size.width as u32,
                size.height as u32,
                image::ColorType::Rgba8,
            )
            .expect("synthetic preview");
        }
        let mut messages = Vec::new();
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            view.as_widget_mut().update(
                &mut tree,
                &Event::Mouse(event),
                Layout::new(&node),
                mouse::Cursor::Available(Point::new(2.0, 2.0)),
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &bounds,
            );
        }
        assert!(matches!(messages.as_slice(), [Message::ClosePositionTpsl]));
    }
}
