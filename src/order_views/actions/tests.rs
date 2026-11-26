use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

#[tokio::test]
async fn refresh_discards_held_presses_and_both_sides_require_a_new_click() {
    let renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_kind = OrderKind::Market;
    let size = Size::new(320.0, 80.0);
    for (x, is_buy) in [(50.0, true), (240.0, false)] {
        let mut tree = Tree::empty();
        let mut send = |terminal: &TradingTerminal, event| {
            let mut actions: Element<'_, Message> = terminal
                .push_order_action_controls(Column::new(), true)
                .into();
            tree.diff(actions.as_widget());
            let node = actions.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, size),
            );
            let mut messages = Vec::new();
            actions.as_widget_mut().update(
                &mut tree,
                &Event::Mouse(event),
                Layout::new(&node),
                mouse::Cursor::Available(Point::new(x, 12.0)),
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &Rectangle::with_size(size),
            );
            messages
        };
        let press = mouse::Event::ButtonPressed(mouse::Button::Left);
        let release = mouse::Event::ButtonReleased(mouse::Button::Left);
        assert!(send(&terminal, press).is_empty());
        terminal.account_reconciliation_required = true;
        assert!(send(&terminal, release).is_empty());
        assert!(send(&terminal, press).is_empty());
        terminal.account_reconciliation_required = false;
        assert!(send(&terminal, release).is_empty());
        assert!(send(&terminal, press).is_empty());
        assert!(matches!(send(&terminal, release).as_slice(),
            [Message::PlaceOrder { is_buy: side, .. }] if *side == is_buy));
    }
}
