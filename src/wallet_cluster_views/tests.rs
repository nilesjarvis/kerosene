use super::*;
use crate::config::{AccountProfile, KeroseneConfig};
use crate::signing::OrderKind;
use crate::wallet_cluster_state::{
    WalletCluster, WalletClusterCloseSide, WalletClusterExecution, WalletClusterExecutionKind,
    WalletClusterExecutionLeg, WalletClusterLegStatus, WalletClusterMember,
    WalletClusterMemberData, WalletClusterPositionSummary,
};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

#[tokio::test]
async fn cluster_close_controls_preserve_order_enablement_and_messages() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    let theme = terminal.theme();
    let size = Size::new(1200.0, 80.0);
    for (index, long, short, enabled) in [
        (0, 2.0, 0.0, [true, false]),
        (1, 0.0, 3.0, [false, true]),
        (2, 2.0, 3.0, [true, true]),
        (3, 0.0, 0.0, [false, false]),
        (4, 1e-12, 1e-12, [false, false]),
    ] {
        let summary = WalletClusterPositionSummary {
            symbol: "BTC".into(),
            net_size: long - short,
            long_size: long,
            short_size: short,
            value: Some(200.0),
            unrealized_pnl: Some(-10.0),
            members: vec![],
        };
        let mut view = terminal.view_wallet_cluster_position_row(summary, &theme);
        let (mut tree, node) = render(
            &mut view,
            &mut renderer,
            &theme,
            size,
            &format!("close-{index}"),
        );
        let layout = Layout::new(&node);
        let row = layout.children().next().expect("position row");
        let controls = row.children().nth(6).expect("close controls container");
        let buttons = controls.children().next().expect("close controls row");
        let points: Vec<_> = buttons
            .children()
            .map(|button| button.bounds().center())
            .collect();
        assert_eq!(points.len(), 6);
        for (button_index, point) in points.into_iter().enumerate() {
            let messages = click(&mut view, &mut tree, &node, &mut renderer, size, point);
            if enabled[button_index / 3] {
                assert_eq!(messages.len(), 1);
                match &messages[0] {
                    Message::WalletClusterClosePosition {
                        symbol,
                        side,
                        fraction,
                        use_market,
                    } => {
                        assert_eq!(symbol, "BTC");
                        assert_eq!(
                            *side,
                            if button_index < 3 {
                                WalletClusterCloseSide::Long
                            } else {
                                WalletClusterCloseSide::Short
                            }
                        );
                        assert_eq!(*fraction, [0.25, 0.5, 1.0][button_index % 3]);
                        assert_eq!(*use_market, button_index % 3 == 2);
                    }
                    _ => panic!("close button must emit a close-position message"),
                }
            } else {
                assert!(messages.is_empty());
            }
        }
    }
}

#[tokio::test]
async fn cluster_window_renders_empty_members_ticket_and_execution_states() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    let size = Size::new(1200.0, 1100.0);
    let theme = terminal.theme();
    render(
        &mut terminal.view_wallet_clusters(),
        &mut renderer,
        &theme,
        size,
        "empty",
    );
    terminal.wallet_clusters.clusters = vec![WalletCluster {
        id: "cluster".into(),
        name: "Test cluster".into(),
        members: vec![WalletClusterMember {
            profile_secret_id: "member".into(),
            weight: 1.0,
            weight_input: "1".into(),
        }],
    }];
    terminal.wallet_clusters.selected_cluster_id = Some("cluster".into());
    terminal.accounts = ["member", "available"]
        .into_iter()
        .map(|id| AccountProfile {
            secret_id: id.into(),
            name: id.into(),
            wallet_address: ADDRESS.into(),
            master_address: None,
            agent_key: "synthetic-key".to_string().into(),
            hydromancer_api_key: String::new().into(),
        })
        .collect();
    terminal.wallet_clusters.member_data.insert(
        "member".into(),
        WalletClusterMemberData {
            address: ADDRESS.into(),
            error: Some("Snapshot unavailable".into()),
            ..Default::default()
        },
    );
    terminal.wallet_clusters.status = Some(("Cluster status".into(), true));
    let statuses = [
        WalletClusterLegStatus::Pending,
        WalletClusterLegStatus::Confirmed,
        WalletClusterLegStatus::Failed,
        WalletClusterLegStatus::Uncertain,
        WalletClusterLegStatus::Checking,
    ];
    for (kind, order_kind, name) in [
        (
            WalletClusterExecutionKind::Order,
            OrderKind::Market,
            "market-order",
        ),
        (
            WalletClusterExecutionKind::Close,
            OrderKind::Limit,
            "limit-close",
        ),
    ] {
        terminal.wallet_clusters.order_kind = order_kind;
        terminal.wallet_clusters.executions.clear();
        terminal
            .wallet_clusters
            .push_execution(WalletClusterExecution {
                id: 1,
                cluster_name: "Test cluster".into(),
                kind,
                symbol: "BTC".into(),
                order_kind,
                created_at_ms: 42,
                legs: statuses
                    .into_iter()
                    .map(|status| WalletClusterExecutionLeg {
                        profile_secret_id: "member".into(),
                        address: ADDRESS.into(),
                        label: "Member label".into(),
                        symbol: "BTC".into(),
                        is_buy: true,
                        size: "1".into(),
                        price: "100".into(),
                        cloid: "synthetic-cloid".into(),
                        status,
                        message: format!("{} result", status.label()),
                    })
                    .collect(),
            });
        render(
            &mut terminal.view_wallet_clusters(),
            &mut renderer,
            &theme,
            size,
            name,
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
    if let Some(directory) = std::env::var_os("KEROSENE_CLUSTER_PREVIEW_DIR") {
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

fn click(
    view: &mut Element<'_, Message>,
    tree: &mut Tree,
    node: &layout::Node,
    renderer: &mut iced::Renderer,
    size: Size,
    point: Point,
) -> Vec<Message> {
    let mut messages = Vec::new();
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        view.as_widget_mut().update(
            tree,
            &Event::Mouse(event),
            Layout::new(node),
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &Rectangle::with_size(size),
        );
    }
    messages
}
