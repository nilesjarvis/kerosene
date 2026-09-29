use super::outcome_symbol_with;
use crate::api::OutcomeVolume24h;
use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::message::Message;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Element, Event, Font, Pixels, Point, Rectangle, Size, Theme};
use std::collections::BTreeSet;

#[tokio::test]
async fn outcome_cards_preserve_responsive_actions_and_display_states() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        for width in [379.0, 380.0, 700.0] {
            for state in 0..6 {
                let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
                terminal.status_bar_now_ms = 1_000;
                terminal.active_symbol = "#1011".to_string();
                terminal.exchange_symbols = [1, 0]
                    .map(|side| outcome_symbol_with(101, side, Some(19), "Below 4.3%"))
                    .to_vec();
                for symbol in &mut terminal.exchange_symbols {
                    let info = symbol.outcome.as_mut().expect("outcome");
                    info.venue = Some("skew".to_string());
                    info.contract.rules = Some("Synthetic contract rules".to_string());
                    info.contract.scalar = state == 3;
                    info.contract.blocked_reason =
                        (state == 4).then(|| "Synthetic block".to_string());
                }
                terminal.outcome_volumes_loading = state == 1;
                if state == 2 {
                    terminal.outcome_volumes_24h.insert(
                        "#1010".to_string(),
                        OutcomeVolume24h {
                            contract: 12_345.0,
                            notional: 4_000.0,
                        },
                    );
                }
                if state == 4 {
                    terminal.outcome_expanded_rules.insert(101);
                }
                if state == 5 {
                    terminal
                        .outcome_collapsed_market_groups
                        .insert("question:19".to_string());
                }
                let group = terminal.grouped_outcome_markets().pop().expect("group");
                let mut view = terminal.view_outcome_market_set(&theme, group, width);
                let size = Size::new(width, 500.0);
                let (mut tree, node) = render(
                    &mut view,
                    &mut renderer,
                    &theme,
                    size,
                    &format!("card-{theme_name}-{width}-{state}"),
                );
                let mut symbols = BTreeSet::new();
                let mut groups = BTreeSet::new();
                let mut rules = BTreeSet::new();
                for y in (5..495).step_by(5) {
                    for x in [15.0, width * 0.25, width * 0.75] {
                        let mut messages = Vec::new();
                        for event in [
                            mouse::Event::ButtonPressed(mouse::Button::Left),
                            mouse::Event::ButtonReleased(mouse::Button::Left),
                        ] {
                            view.as_widget_mut().update(
                                &mut tree,
                                &Event::Mouse(event),
                                Layout::new(&node),
                                mouse::Cursor::Available(Point::new(x, y as f32)),
                                &renderer,
                                &mut clipboard::Null,
                                &mut Shell::new(&mut messages),
                                &Rectangle::with_size(size),
                            );
                        }
                        for message in messages {
                            match message {
                                Message::SymbolSelected(key) => {
                                    symbols.insert(key);
                                }
                                Message::OutcomeMarketGroupToggled(key) => {
                                    groups.insert(key);
                                }
                                Message::OutcomeRulesToggled(id) => {
                                    rules.insert(id);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                assert_eq!(groups, BTreeSet::from(["question:19".to_string()]));
                if state == 5 {
                    assert!(symbols.is_empty() && rules.is_empty());
                } else {
                    assert_eq!(
                        symbols,
                        BTreeSet::from(["#1010".to_string(), "#1011".to_string()])
                    );
                    assert_eq!(rules, BTreeSet::from([101]));
                }
            }
        }
    }

    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    for state in 0..8 {
        terminal.symbols_loading = state == 0;
        terminal.outcome_search_query = if state == 1 { "  missing  " } else { "" }.to_string();
        terminal.outcome_venue_filter = matches!(state, 2 | 7).then(|| "unavailable".to_string());
        terminal.exchange_symbols = if state >= 4 {
            vec![outcome_symbol_with(101, 0, Some(19), "Below 4.3%")]
        } else {
            Vec::new()
        };
        terminal.outcome_volumes_loading = state == 5;
        terminal.outcome_volumes_error = (state == 6).then(|| "synthetic failure".to_string());
        render(
            &mut terminal.view_outcomes_sized(450.0),
            &mut renderer,
            &terminal.theme(),
            Size::new(450.0, 550.0),
            &format!("pane-{state}"),
        );
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
    if let Some(directory) = std::env::var_os("KEROSENE_OUTCOME_VIEW_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("preview directory");
        image::save_buffer(
            directory.join(format!("{name}.png")),
            &pixels,
            size.width as u32,
            size.height as u32,
            image::ColorType::Rgba8,
        )
        .expect("synthetic preview");
    }
    (tree, node)
}
