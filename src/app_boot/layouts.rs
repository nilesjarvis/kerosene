use crate::account_state::BottomTab;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::config::{KeroseneConfig, SavedLayout};
use crate::pane_state::PaneKind;
use iced::widget::pane_grid;

impl TradingTerminal {
    pub(crate) fn register_last_layout(cfg: &mut KeroseneConfig) {
        let last_layout = SavedLayout {
            name: "last".to_string(),
            pane_layout: cfg.pane_layout.clone(),
            layout_ratios: cfg.layout_ratios.clone(),
            charts: cfg.charts.clone(),
            order_books: cfg.order_books.clone(),
            live_watchlists: cfg.live_watchlists.clone(),
            positioning_infos: cfg.positioning_infos.clone(),
            spaghetti_charts: cfg.spaghetti_charts.clone(),
            widget_padding: cfg.widget_padding.clone().normalized(),
            active_symbol: cfg.active_symbol.clone(),
            active_timeframe: cfg.active_timeframe.clone(),
            order_kind: cfg.order_kind.clone(),
            reduce_only: cfg.reduce_only,
            book_tick_size: cfg.book_tick_size,
            favourite_symbols: cfg.favourite_symbols.clone(),
            ticker_tape_enabled: cfg.ticker_tape_enabled,
            active_theme: cfg.active_theme.clone(),
            custom_themes: cfg.custom_themes.clone(),
            sound_enabled: cfg.sound_enabled,
            desktop_notifications: cfg.desktop_notifications,
            income_alerts_enabled: cfg.income_alerts_enabled,
            liquidation_alerts_enabled: cfg.liquidation_alerts_enabled,
            liquidation_alert_threshold: cfg.liquidation_alert_threshold,
            market_slippage_pct: cfg.market_slippage_pct,
            tracked_trade_alerts_enabled: cfg.tracked_trade_alerts_enabled,
            tracked_trade_aggregation_enabled: cfg.tracked_trade_aggregation_enabled,
            liquidation_feed_aggregation_enabled: cfg.liquidation_feed_aggregation_enabled,
            preset_is_usd: cfg.preset_is_usd,
            order_presets: cfg.order_presets.clone(),
        };

        if let Some(pos) = cfg
            .saved_layouts
            .iter()
            .position(|layout| layout.name == "last")
        {
            cfg.saved_layouts[pos] = last_layout;
        } else {
            cfg.saved_layouts.insert(0, last_layout);
        }

        if cfg.active_layout_name.is_none() {
            cfg.active_layout_name = Some("last".to_string());
        }
    }

    pub(crate) fn default_boot_pane_configuration(
        first_chart_id: ChartId,
        ratios: [f32; 4],
    ) -> pane_grid::Configuration<PaneKind> {
        use pane_grid::{Axis, Configuration as PaneCfg};

        PaneCfg::Split {
            axis: Axis::Horizontal,
            ratio: ratios[0],
            a: Box::new(PaneCfg::Split {
                axis: Axis::Vertical,
                ratio: ratios[1],
                a: Box::new(PaneCfg::Pane(PaneKind::Chart(first_chart_id))),
                b: Box::new(PaneCfg::Split {
                    axis: Axis::Vertical,
                    ratio: ratios[2],
                    a: Box::new(PaneCfg::Pane(PaneKind::OrderBook(0))),
                    b: Box::new(PaneCfg::Pane(PaneKind::Watchlist)),
                }),
            }),
            b: Box::new(PaneCfg::Split {
                axis: Axis::Vertical,
                ratio: ratios[3],
                a: Box::new(PaneCfg::Pane(PaneKind::BottomTabs {
                    active_tab: BottomTab::Positions,
                })),
                b: Box::new(PaneCfg::Pane(PaneKind::OrderEntry)),
            }),
        }
    }

    pub(super) fn boot_layout_ratios(cfg: &KeroseneConfig) -> [f32; 4] {
        let ratios = movable_pane_layout_ratios(&cfg.layout_ratios);
        [
            ratios.first().copied().unwrap_or(0.70),
            ratios.get(1).copied().unwrap_or(0.50),
            ratios.get(2).copied().unwrap_or(0.55),
            ratios.get(3).copied().unwrap_or(0.65),
        ]
    }
}

fn movable_pane_layout_ratios(ratios: &[f32]) -> &[f32] {
    if ratios.len() >= 5 {
        &ratios[1..]
    } else {
        ratios
    }
}
