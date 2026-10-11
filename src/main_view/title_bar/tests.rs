use super::*;
use crate::account::{AccountData, ClearinghouseState, MarginSummary, SpotClearinghouseState};
use crate::config::KeroseneConfig;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, renderer, widget::Tree};
use iced::widget::text;
use iced::{Event, Font, Pixels, Point, Rectangle, Size, Theme};

struct Scene<'a> {
    view: Element<'a, Message>,
    tree: Tree,
    node: layout::Node,
    bounds: Rectangle,
}

impl<'a> Scene<'a> {
    fn render(
        mut view: Element<'a, Message>,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        size: Size,
        name: &str,
    ) -> Self {
        let bounds = Rectangle::with_size(size);
        let mut tree = Tree::new(view.as_widget());
        let node =
            view.as_widget_mut()
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
        if let Some(directory) = std::env::var_os("KEROSENE_SHELL_VIEW_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).expect("preview directory");
            image::save_buffer(
                directory.join(format!("{name}.png")),
                &pixels,
                size.width as u32,
                size.height as u32,
                image::ColorType::Rgba8,
            )
            .expect("synthetic shell preview");
        }
        Self {
            view,
            tree,
            node,
            bounds,
        }
    }

    fn click(&mut self, renderer: &iced::Renderer, point: Point) -> Vec<Message> {
        let mut messages = Vec::new();
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            self.view.as_widget_mut().update(
                &mut self.tree,
                &Event::Mouse(event),
                Layout::new(&self.node),
                mouse::Cursor::Available(point),
                renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &self.bounds,
            );
        }
        messages
    }
}

#[tokio::test]
async fn title_bar_preserves_window_and_status_actions() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    let window_id = window::Id::unique();
    for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        for width in [600.0, 1000.0] {
            for toggles in [false, true] {
                for enabled in [false, true] {
                    terminal.hide_pnl = enabled;
                    terminal.sound_enabled = enabled;
                    terminal.desktop_notifications = enabled;
                    let mut scene = Scene::render(
                        terminal.view_window_title_bar(window_id, toggles),
                        &mut renderer,
                        &theme,
                        Size::new(width, 34.0),
                        &format!("title-{theme_name}-{width}-{toggles}-{enabled}"),
                    );
                    assert!(
                        matches!(scene.click(&renderer, Point::new(100.0, 17.0)).as_slice(),
                        [Message::WindowDrag(id)] if *id == window_id)
                    );
                    for (offset, expected) in [
                        (21.0, Message::WindowClose(window_id)),
                        (63.0, Message::WindowToggleMaximize(window_id)),
                        (105.0, Message::WindowMinimize(window_id)),
                    ] {
                        let messages = scene.click(&renderer, Point::new(width - offset, 17.0));
                        assert_eq!(messages.len(), 1);
                        assert_eq!(format!("{:?}", messages[0]), format!("{expected:?}"));
                    }
                    if toggles {
                        for (offset, expected) in [
                            (143.0, Message::ToggleDesktopNotifications),
                            (177.0, Message::ToggleSound),
                            (211.0, Message::ToggleHidePnl),
                        ] {
                            let messages = scene.click(&renderer, Point::new(width - offset, 17.0));
                            assert_eq!(messages.len(), 1);
                            assert_eq!(format!("{:?}", messages[0]), format!("{expected:?}"));
                        }
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn native_chrome_keeps_status_actions_in_every_account_summary_state() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        for width in [600.0, 1000.0, 1600.0] {
            terminal.main_window_size = Some(Size::new(width, 800.0));
            for state in ["disconnected", "loading", "error", "populated"] {
                terminal.connected_address =
                    (state != "disconnected").then(|| "synthetic".to_string());
                terminal.account_data_address = terminal.connected_address.clone();
                terminal.account_loading = state == "loading";
                terminal.account_error = (state == "error").then(|| "Synthetic error".to_string());
                terminal.account_data = (state == "populated").then(synthetic_account_data);
                for custom_chrome in [false, true] {
                    // Use the active chrome mode even if a restart is pending.
                    terminal.custom_window_chrome_enabled = !custom_chrome;
                    terminal.custom_window_chrome_active = custom_chrome;
                    for enabled in [false, true] {
                        terminal.hide_pnl = enabled;
                        terminal.sound_enabled = enabled;
                        terminal.desktop_notifications = enabled;
                        let height = terminal.account_summary_bar_height();
                        let mut scene = Scene::render(
                            terminal.view_account_summary_bar(),
                            &mut renderer,
                            &theme,
                            Size::new(width, height),
                            &format!(
                                "summary-{theme_name}-{width}-{state}-{custom_chrome}-{enabled}"
                            ),
                        );
                        for (offset, expected) in [
                            (29.0, Message::ToggleDesktopNotifications),
                            (63.0, Message::ToggleSound),
                            (97.0, Message::ToggleHidePnl),
                        ] {
                            // Wrapped summaries move the whole action row down.
                            let messages = (0..height as u32)
                                .step_by(2)
                                .map(|y| {
                                    scene.click(&renderer, Point::new(width - offset, y as f32))
                                })
                                .find(|messages| {
                                    messages.iter().any(|message| {
                                        matches!(
                                            message,
                                            Message::ToggleDesktopNotifications
                                                | Message::ToggleSound
                                                | Message::ToggleHidePnl
                                        )
                                    })
                                })
                                .unwrap_or_default();
                            if custom_chrome {
                                assert!(messages.is_empty());
                            } else {
                                assert_eq!(messages.len(), 1, "{state} summary at width {width}");
                                assert_eq!(format!("{:?}", messages[0]), format!("{expected:?}"));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn synthetic_account_data() -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".to_string(),
                total_ntl_pos: "200".to_string(),
                total_margin_used: "100".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "900".to_string(),
            asset_positions: Vec::new(),
        },
        clearinghouses_by_dex: Default::default(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: Default::default(),
        completeness: Default::default(),
        fetched_at_ms: 0,
    }
}

#[tokio::test]
async fn shell_framing_preserves_resize_layers_and_onboarding_action() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.status_bar_now_ms = 0;
    terminal.ticker_tape_enabled = false;
    let window_id = window::Id::unique();
    for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        for chrome in [false, true] {
            terminal.custom_window_chrome_active = chrome;
            for dividers in [false, true] {
                terminal.pane_dividers_enabled = dividers;
                let mut scene = Scene::render(
                    terminal.view_window_chrome(
                        window_id,
                        container(text("Synthetic content"))
                            .width(Fill)
                            .height(Fill)
                            .into(),
                    ),
                    &mut renderer,
                    &theme,
                    Size::new(600.0, 260.0),
                    &format!("frame-{theme_name}-{chrome}-{dividers}"),
                );
                for (point, direction) in [
                    (Point::new(300.0, 1.0), window::Direction::North),
                    (Point::new(1.0, 130.0), window::Direction::West),
                    (Point::new(599.0, 259.0), window::Direction::SouthEast),
                ] {
                    let messages = scene.click(&renderer, point);
                    if chrome {
                        assert_eq!(messages.len(), 1);
                        assert_eq!(
                            format!("{:?}", messages[0]),
                            format!("{:?}", Message::WindowDragResize(window_id, direction))
                        );
                    } else {
                        assert!(messages.is_empty());
                    }
                }
            }
            terminal.pane_dividers_enabled = true;
            for onboarding in [false, true] {
                terminal.app_onboarding_dismissed = !onboarding;
                for phase in [0.0, 12.5] {
                    terminal.onboarding_phase = phase;
                    let mut scene = Scene::render(
                        terminal.view_main_window(window_id),
                        &mut renderer,
                        &theme,
                        Size::new(900.0, 550.0),
                        &format!("main-{theme_name}-{chrome}-{onboarding}-{phase}"),
                    );
                    if onboarding {
                        let mut entered = false;
                        for y in (40..520).step_by(5) {
                            entered |= scene
                                .click(&renderer, Point::new(450.0, y as f32))
                                .iter()
                                .any(|message| matches!(message, Message::EnterApplication));
                        }
                        assert!(entered, "onboarding action must remain reachable");
                    }
                }
            }
        }
    }
}
