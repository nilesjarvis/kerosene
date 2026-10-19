use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::feed_state::TrackedTradeIntent;
use crate::trade_feed_state::{TradeFeedEvent, TradeFeedInstance, TradePrint};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Font, Pixels, Rectangle, Size};

#[tokio::test]
async fn trade_feed_summary_stays_at_bottom_when_resized_or_picker_opens() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.active_symbol = "BTC".into();
    terminal.exchange_symbols.push(ExchangeSymbol {
        key: "BTC".into(),
        ticker: "BTC".into(),
        category: "crypto".into(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 40,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Perp,
        outcome: None,
    });
    terminal.status_bar_now_ms = 120 * 60_000;
    let mut feed = TradeFeedInstance::new(Default::default());
    feed.symbol = Some("BTC".into());
    feed.apply(TradeFeedEvent::Trades(
        (0..100)
            .map(|tid| TradePrint {
                time_ms: terminal.status_bar_now_ms.saturating_sub(tid * 10_000),
                tid,
                is_buy: tid % 3 != 0,
                price: "60000".into(),
                size: "0.1".into(),
                value: 6_000.0,
                trader: None,
                intent: TrackedTradeIntent::Unknown,
            })
            .collect(),
    ));
    terminal.trade_feeds.instances.insert(7, feed);

    for (width, height, picker) in [
        (320.0, 240.0, false),
        (640.0, 360.0, false),
        (640.0, 360.0, true),
        (1_000.0, 600.0, false),
    ] {
        terminal
            .trade_feeds
            .instances
            .get_mut(&7)
            .expect("feed")
            .picker_open = picker;
        let theme = terminal.theme();
        let mut view =
            terminal.view_trade_feed_sized(7, &terminal.trade_feeds.instances[&7], width);
        let size = Size::new(width, height);
        let bounds = Rectangle::with_size(size);
        let mut tree = Tree::new(view.as_widget());
        let node =
            view.as_widget_mut()
                .layout(&mut tree, &renderer, &layout::Limits::new(size, size));
        let footer = node.children().last().expect("summary footer").bounds();
        assert!((footer.y + footer.height - height).abs() < 0.01);
        assert!(footer.height > 30.0 && footer.height < height / 2.0);
        let tape = node.children()[0].bounds();
        assert!(tape.y + tape.height <= footer.y);

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
            Size::new(width as u32, height as u32),
            1.0,
            theme.palette().background,
        );
        if let Some(directory) = std::env::var_os("KEROSENE_TRADE_FEED_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).expect("preview directory");
            image::save_buffer(
                directory.join(format!("summary-{width}-{height}-{picker}.png")),
                &pixels,
                width as u32,
                height as u32,
                image::ColorType::Rgba8,
            )
            .expect("synthetic preview");
        }
    }
}
