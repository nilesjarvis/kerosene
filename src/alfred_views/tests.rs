use super::*;
use crate::config::KeroseneConfig;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

#[tokio::test]
async fn alfred_rows_and_overlay_render_with_stable_click_targets() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    assert!(crate::helpers::symbol_icon("BTC", 14, Color::WHITE).is_some());
    assert!(crate::helpers::symbol_icon("UNKNOWN_FIXTURE", 14, Color::WHITE).is_none());

    for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        // Plain titles, every icon fallback, and anchors at either end or repeated.
        for (index, (title, symbol, anchor, tag, enabled, reason)) in [
            ("Chart", None, None, "Open", true, None),
            ("Buy BTC", Some("BTC"), None, "Window", true, None),
            (
                "Buy BTC",
                Some("UNKNOWN_FIXTURE"),
                Some("BTC"),
                "Trade",
                false,
                None,
            ),
            (
                "Buy ETH",
                Some("BTC"),
                Some("BTC"),
                "Requires PM",
                false,
                Some("Portfolio margin required"),
            ),
            ("BTC", Some("BTC"), Some("BTC"), "Market", true, None),
            (
                "Buy BTC",
                Some("BTC"),
                Some("BTC"),
                "Limit",
                true,
                Some("Ignored while enabled"),
            ),
            (
                "BTC at market",
                Some("BTC"),
                Some("BTC"),
                "Close",
                false,
                Some("Account refresh in progress"),
            ),
            (
                "BTC →  BTC  at market",
                Some("BTC"),
                Some("BTC"),
                "Custom",
                true,
                None,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let mut command = terminal.alfred_filtered_commands().remove(0);
            let command_id = command.id;
            command.title = title.into();
            command.detail = "Preview detail".into();
            command.icon_symbol = symbol.map(str::to_owned);
            command.icon_title_anchor = anchor.map(str::to_owned);
            command.tag = tag.into();
            command.enabled = enabled;
            command.disabled_reason = reason.map(str::to_owned);
            let mut view =
                alfred_result_row(command, index % 2 == 0, &theme, [0.1, 1.0, 2.0][index % 3]);
            let size = Size::new(600.0, 100.0);
            let (mut tree, node) = render(
                &mut view,
                &mut renderer,
                &theme,
                size,
                &format!("{theme_name}-row-{index}"),
            );
            let mut messages = Vec::new();
            for event in [
                mouse::Event::ButtonPressed(mouse::Button::Left),
                mouse::Event::ButtonReleased(mouse::Button::Left),
            ] {
                view.as_widget_mut().update(
                    &mut tree,
                    &Event::Mouse(event),
                    Layout::new(&node),
                    mouse::Cursor::Available(Point::new(20.0, 20.0)),
                    &renderer,
                    &mut clipboard::Null,
                    &mut Shell::new(&mut messages),
                    &Rectangle::with_size(size),
                );
            }
            if enabled {
                assert!(
                    matches!(messages.as_slice(), [Message::AlfredCommandSelected(id)] if *id == command_id)
                );
            } else {
                assert!(messages.is_empty());
            }
        }

        terminal.alfred.open = false;
        assert!(terminal.view_alfred_overlay(&theme).is_none());
        terminal.alfred.open = true;
        for (index, (query, selected_index, scale)) in [
            ("", 0, 1.0),
            ("", usize::MAX, 1.0),
            ("no_such_command_fixture", 0, 0.85),
            ("buy", 0, 1.35),
            ("close btc", 0, 1.0),
        ]
        .into_iter()
        .enumerate()
        {
            terminal.alfred.query = query.into();
            terminal.alfred.selected_index = selected_index;
            terminal.alfred_popup_scale = scale;
            let mut view = terminal.view_alfred_overlay(&theme).expect("open overlay");
            render(
                &mut view,
                &mut renderer,
                &theme,
                Size::new(800.0, 650.0),
                &format!("{theme_name}-overlay-{index}"),
            );
        }
        terminal.alfred.query.clear();
    }
}

fn render(
    view: &mut Element<'_, Message>,
    renderer: &mut iced::Renderer,
    theme: &Theme,
    size: Size,
    name: &str,
) -> (Tree, layout::Node) {
    let bounds = Rectangle::with_size(size);
    let mut tree = Tree::new(view.as_widget());
    let node = view
        .as_widget_mut()
        .layout(&mut tree, renderer, &layout::Limits::new(size, size));
    iced::advanced::Renderer::reset(renderer, bounds);
    view.as_widget().draw(
        &tree,
        renderer,
        theme,
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
    assert_eq!(pixels.len(), size.width as usize * size.height as usize * 4);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    if let Some(directory) = std::env::var_os("KEROSENE_ALFRED_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("preview directory");
        image::save_buffer(
            directory.join(format!("{name}.png")),
            &pixels,
            size.width as u32,
            size.height as u32,
            image::ColorType::Rgba8,
        )
        .expect("synthetic Alfred preview");
    }
    (tree, node)
}
