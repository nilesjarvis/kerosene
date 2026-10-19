use super::*;
use crate::feed_views::liquidations::ticker_clicks::with_ticker_clicks;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size, keyboard};

fn symbol_cell(symbol: &str) -> Element<'static, Message> {
    with_ticker_clicks(liquidation_symbol_button(symbol.into(), symbol.into(), &Theme::Dark).into())
}

#[tokio::test]
async fn liquidation_ticker_ctrl_click_opens_window_and_plain_click_selects_symbol() {
    let renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");

    for (modifiers, lose_focus, open_window) in [
        (keyboard::Modifiers::NONE, false, false),
        (keyboard::Modifiers::CTRL, false, true),
        (
            keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
            false,
            true,
        ),
        (keyboard::Modifiers::SHIFT, false, false),
        (keyboard::Modifiers::CTRL, true, false),
    ] {
        let mut view = symbol_cell("BTC");
        let mut tree = Tree::new(view.as_widget());
        let size = Size::new(80.0, 30.0);
        let limits = layout::Limits::new(size, size);
        let bounds = Rectangle::with_size(size);
        let node = view.as_widget_mut().layout(&mut tree, &renderer, &limits);
        let mut messages = Vec::new();
        view.as_widget_mut().update(
            &mut tree,
            &Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &bounds,
        );

        // Live feed rows can change while Ctrl is held. Keep the feed's state
        // across the rebuild and click the replacement ticker.
        view = symbol_cell("xyz:ETH");
        tree.diff(view.as_widget());
        let node = view.as_widget_mut().layout(&mut tree, &renderer, &limits);
        let mut events = Vec::new();
        if lose_focus {
            events.push(Event::Window(iced::window::Event::Unfocused));
        }
        events.extend([
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ]);
        for event in events {
            view.as_widget_mut().update(
                &mut tree,
                &event,
                Layout::new(&node),
                mouse::Cursor::Available(Point::new(10.0, 10.0)),
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &bounds,
            );
        }
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::OpenDetachedChartForSymbol(symbol) if open_window => {
                assert_eq!(symbol, "xyz:ETH");
            }
            Message::SymbolSelected(symbol) if !open_window => {
                assert_eq!(symbol, "xyz:ETH");
            }
            message => panic!("unexpected ticker action: {message:?}"),
        }

        messages.clear();
        for event in [
            Event::Keyboard(keyboard::Event::ModifiersChanged(keyboard::Modifiers::NONE)),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ] {
            view.as_widget_mut().update(
                &mut tree,
                &event,
                Layout::new(&node),
                mouse::Cursor::Available(Point::new(10.0, 10.0)),
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &bounds,
            );
        }
        assert!(
            matches!(messages.as_slice(), [Message::SymbolSelected(symbol)] if symbol == "xyz:ETH")
        );
    }
}
