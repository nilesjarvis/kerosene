use super::outcome_symbol;
use crate::api::{ExchangeStats, WatchlistContext};
use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::message::Message;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

#[tokio::test]
async fn ticker_tape_preserves_scrolling_copies_clipping_and_selection() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.exchange_symbols = vec![outcome_symbol("#950")];
    terminal.ticker_tape_exchange_stats = Some(ExchangeStats {
        volume_24h_notional_usd: 4_250_000_000.0,
        open_interest_notional_usd: 1_000_000_000.0,
    });
    terminal.all_mids.insert("BTC".to_string(), 50_000.0);
    terminal.all_mids.insert("#950".to_string(), 0.42);
    let now = TradingTerminal::now_ms();
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), now);
    terminal
        .all_mids_updated_at_ms
        .insert("#950".to_string(), now);
    terminal.ticker_tape_ctxs.insert(
        "BTC".to_string(),
        WatchlistContext {
            funding: None,
            prev_day_px: Some(40_000.0),
            mark_px: None,
            day_vlm: None,
            open_interest_notional: None,
        },
    );
    terminal.muted_tickers.insert("HIDDEN".to_string());
    let theme = terminal.theme();
    for width in [600.0, 1500.0] {
        for offset in [0.0, 149.0, 482.5, -20.0] {
            for dividers in [false, true] {
                for populated in [false, true] {
                    terminal.favourite_symbols = if populated {
                        ["BTC", "#950", "???Ω", "HIDDEN"]
                            .map(str::to_string)
                            .to_vec()
                    } else {
                        Vec::new()
                    };
                    terminal.pane_dividers_enabled = dividers;
                    terminal.ticker_tape_scroll_px = offset;
                    let mut view = terminal.view_ticker_tape_bar_sized(width);
                    let size = Size::new(width, 34.0);
                    let bounds = Rectangle::with_size(size);
                    let mut tree = Tree::new(view.as_widget());
                    let node = view.as_widget_mut().layout(
                        &mut tree,
                        &renderer,
                        &layout::Limits::new(size, size),
                    );
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
                        Size::new(width as u32, 34),
                        1.0,
                        theme.palette().background,
                    );
                    assert_eq!(pixels.len(), width as usize * 34 * 4);
                    if let Some(directory) = std::env::var_os("KEROSENE_TICKER_VIEW_PREVIEW_DIR") {
                        let directory = std::path::PathBuf::from(directory);
                        std::fs::create_dir_all(&directory).expect("preview directory");
                        image::save_buffer(
                            directory.join(format!("{width}-{offset}-{dividers}-{populated}.png")),
                            &pixels,
                            width as u32,
                            34,
                            image::ColorType::Rgba8,
                        )
                        .expect("synthetic preview");
                    }
                    let mut selected = Vec::new();
                    for x in (5..width as usize).step_by(5) {
                        let mut messages = Vec::new();
                        for event in [
                            mouse::Event::ButtonPressed(mouse::Button::Left),
                            mouse::Event::ButtonReleased(mouse::Button::Left),
                        ] {
                            view.as_widget_mut().update(
                                &mut tree,
                                &Event::Mouse(event),
                                Layout::new(&node),
                                mouse::Cursor::Available(Point::new(x as f32, 17.0)),
                                &renderer,
                                &mut clipboard::Null,
                                &mut Shell::new(&mut messages),
                                &bounds,
                            );
                        }
                        for message in messages {
                            if let Message::SymbolSelected(key) = message {
                                assert!(
                                    (x as f32) < width - 356.0,
                                    "fixed stats must not select a symbol"
                                );
                                assert_ne!(key, "HIDDEN");
                                if selected.last() != Some(&key) {
                                    selected.push(key);
                                }
                            }
                        }
                    }
                    if !populated {
                        assert!(selected.is_empty());
                    } else if width == 1500.0 {
                        assert_eq!(selected, ["BTC", "#950", "???Ω"]);
                    } else {
                        assert!(!selected.is_empty());
                    }
                }
            }
        }
    }
}
