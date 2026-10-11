use super::*;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{Layout, layout};
use iced::{Font, Pixels, Rectangle, Renderer, Size, Theme};

#[derive(Default, PartialEq, Debug)]
struct RowText(Vec<String>);

impl Operation for RowText {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, _bounds: Rectangle, text: &str) {
        self.0.push(text.to_string());
    }
}

fn row_text(
    renderer: &Renderer,
    position: &TickerPositionEntry,
    mark: Option<f64>,
    width: f32,
) -> RowText {
    let theme = Theme::Dark;
    let denomination = DisplayDenominationContext::usd();
    let mut row = positioning_position_row(
        PositioningRowContext {
            instance_id: 7,
            hovered_wallet_action_key: None,
            theme: &theme,
            live_mark: mark,
            denomination: &denomination,
            max_notional: 1_000.0,
        },
        position,
        WalletDisplay {
            primary: "Synthetic trader".to_string(),
            secondary: String::new(),
            has_label: true,
        },
        PositioningInfoColumns::for_width(width),
    );
    let mut tree = Tree::new(row.as_widget());
    let node = row.as_widget_mut().layout(
        &mut tree,
        renderer,
        &layout::Limits::new(Size::ZERO, Size::new(width, 100.0)),
    );
    let mut text = RowText::default();
    row.as_widget_mut()
        .operate(&mut tree, Layout::new(&node), renderer, &mut text);
    text
}

#[tokio::test]
async fn positioning_rows_never_revert_to_snapshot_upnl() {
    let renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer for positioning rows");
    for (width, expected) in [(450.0, "+$60"), (1_200.0, "+$60.00")] {
        let mut position = sample_position();
        position.unrealized_pnl = 67.0;
        let live = row_text(&renderer, &position, Some(31.0), width);
        assert!(live.0.iter().any(|value| value == expected));
        let unavailable = row_text(&renderer, &position, None, width);
        assert!(unavailable.0.iter().any(|value| value == "-"));

        // A fetched snapshot must not replace the live value, even during a
        // price gap; otherwise each recovery jumps between valuation times.
        position.unrealized_pnl = 99_999.0;
        assert_eq!(row_text(&renderer, &position, Some(31.0), width), live);
        assert_eq!(row_text(&renderer, &position, None, width), unavailable);
    }
}
