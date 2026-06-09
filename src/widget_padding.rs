use crate::app_state::TradingTerminal;
use crate::config::{
    WidgetPaddingConfig, WidgetPaddingOverrideConfig, WidgetPaddingTargetConfig,
    normalize_widget_padding,
};
use crate::pane_state::PaneKind;
use iced::widget::pane_grid;

// ---------------------------------------------------------------------------
// Widget Padding
// ---------------------------------------------------------------------------

impl WidgetPaddingTargetConfig {
    pub(crate) fn from_pane_kind(kind: &PaneKind) -> Self {
        match kind {
            PaneKind::Chart(chart_id) => Self::Chart {
                chart_id: *chart_id,
            },
            PaneKind::OrderBook(id) => Self::OrderBook { id: *id },
            PaneKind::Watchlist => Self::Watchlist,
            PaneKind::LiveWatchlist(id) => Self::LiveWatchlist { id: *id },
            PaneKind::PositioningInfo(id) => Self::PositioningInfo { id: *id },
            PaneKind::SessionData(id) => Self::SessionData { id: *id },
            PaneKind::Portfolio => Self::Portfolio,
            PaneKind::Income => Self::Income,
            PaneKind::BottomTabs { .. } => Self::BottomTabs,
            PaneKind::OrderEntry => Self::OrderEntry,
            PaneKind::AdvancedOrders => Self::AdvancedOrders,
            PaneKind::SpaghettiChart(spaghetti_id) => Self::SpaghettiChart {
                spaghetti_id: *spaghetti_id,
            },
            PaneKind::Settings => Self::Settings,
            PaneKind::Calendar => Self::Calendar,
            PaneKind::Liquidations => Self::Liquidations,
            PaneKind::LiquidationsDistribution => Self::LiquidationsDistribution,
            PaneKind::TrackedTrades => Self::TrackedTrades,
            PaneKind::TelegramFeed => Self::TelegramFeed,
            PaneKind::XFeed => Self::XFeed,
            PaneKind::Outcomes => Self::Outcomes,
            PaneKind::HypeEtfs => Self::HypeEtfs,
            PaneKind::HypeUnstakingQueue => Self::HypeUnstakingQueue,
        }
    }
}

impl TradingTerminal {
    pub(crate) fn widget_padding_config_snapshot(&self) -> WidgetPaddingConfig {
        let default_px = normalize_widget_padding(self.widget_padding_default);
        let active_targets = self.active_widget_padding_targets();

        WidgetPaddingConfig {
            default_px,
            overrides: self
                .widget_padding_overrides
                .iter()
                .filter(|(target, padding_px)| {
                    active_targets.contains(target)
                        && (**padding_px - default_px).abs() > f32::EPSILON
                })
                .map(|(target, padding_px)| WidgetPaddingOverrideConfig {
                    target: target.clone(),
                    padding_px: normalize_widget_padding(*padding_px),
                })
                .collect(),
        }
        .normalized()
    }

    pub(crate) fn apply_widget_padding_config(&mut self, config: &WidgetPaddingConfig) {
        let config = config.clone().normalized();
        let active_targets = self.active_widget_padding_targets();
        self.widget_padding_default = config.default_px;
        self.widget_padding_overrides = config
            .overrides
            .into_iter()
            .filter(|item| active_targets.contains(&item.target))
            .map(|item| (item.target, item.padding_px))
            .collect();
    }

    pub(crate) fn widget_padding_for_kind(&self, kind: &PaneKind) -> f32 {
        let target = WidgetPaddingTargetConfig::from_pane_kind(kind);
        self.widget_padding_for_target(&target)
    }

    pub(crate) fn widget_padding_for_target(&self, target: &WidgetPaddingTargetConfig) -> f32 {
        self.widget_padding_overrides
            .get(target)
            .copied()
            .unwrap_or(self.widget_padding_default)
    }

    pub(crate) fn focused_widget_padding_target(
        &self,
    ) -> Option<(pane_grid::Pane, WidgetPaddingTargetConfig)> {
        let pane = self.focus?;
        let kind = self.panes.get(pane)?;
        Some((pane, WidgetPaddingTargetConfig::from_pane_kind(kind)))
    }

    pub(crate) fn focused_widget_padding(&self) -> Option<f32> {
        self.focused_widget_padding_target()
            .map(|(_, target)| self.widget_padding_for_target(&target))
    }

    pub(crate) fn set_default_widget_padding(&mut self, value: f32) {
        let default_px = normalize_widget_padding(value);
        self.widget_padding_default = default_px;
        self.widget_padding_overrides
            .retain(|_, padding_px| (*padding_px - default_px).abs() > f32::EPSILON);
    }

    pub(crate) fn set_focused_widget_padding(&mut self, value: f32) -> bool {
        let Some((_, target)) = self.focused_widget_padding_target() else {
            return false;
        };

        let padding_px = normalize_widget_padding(value);
        if (padding_px - self.widget_padding_default).abs() <= f32::EPSILON {
            self.widget_padding_overrides.remove(&target);
        } else {
            self.widget_padding_overrides.insert(target, padding_px);
        }

        true
    }

    pub(crate) fn reset_focused_widget_padding(&mut self) -> bool {
        let Some((_, target)) = self.focused_widget_padding_target() else {
            return false;
        };

        self.widget_padding_overrides.remove(&target).is_some()
    }

    pub(crate) fn remove_widget_padding_override_for_kind(&mut self, kind: &PaneKind) {
        let target = WidgetPaddingTargetConfig::from_pane_kind(kind);
        self.widget_padding_overrides.remove(&target);
    }

    fn active_widget_padding_targets(&self) -> Vec<WidgetPaddingTargetConfig> {
        self.panes
            .iter()
            .map(|(_, kind)| WidgetPaddingTargetConfig::from_pane_kind(kind))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::TradingTerminal;
    use crate::config::{KeroseneConfig, WidgetPaddingOverrideConfig, WidgetPaddingTargetConfig};
    use crate::message::Message;
    use crate::pane_state::PaneKind;

    #[test]
    fn bottom_tabs_padding_target_ignores_active_tab() {
        let positions = WidgetPaddingTargetConfig::from_pane_kind(&PaneKind::BottomTabs {
            active_tab: crate::account_state::BottomTab::Positions,
        });
        let balances = WidgetPaddingTargetConfig::from_pane_kind(&PaneKind::BottomTabs {
            active_tab: crate::account_state::BottomTab::Balances,
        });

        assert_eq!(positions, balances);
    }

    #[test]
    fn focused_widget_padding_sets_and_resets_sparse_override() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig {
            widget_padding: crate::config::WidgetPaddingConfig {
                default_px: 4.0,
                overrides: Vec::new(),
            },
            ..KeroseneConfig::default()
        });
        let pane = terminal
            .panes
            .iter()
            .find_map(|(pane, kind)| matches!(kind, PaneKind::Watchlist).then_some(*pane))
            .expect("default layout should include watchlist");

        let _ = terminal.update(Message::PaneClicked(pane));
        assert_eq!(terminal.focused_widget_padding(), Some(4.0));

        assert!(terminal.set_focused_widget_padding(12.0));
        assert_eq!(terminal.focused_widget_padding(), Some(12.0));
        assert_eq!(terminal.widget_padding_config_snapshot().overrides.len(), 1);

        assert!(terminal.reset_focused_widget_padding());
        assert_eq!(terminal.focused_widget_padding(), Some(4.0));
        assert!(
            terminal
                .widget_padding_config_snapshot()
                .overrides
                .is_empty()
        );
    }

    #[test]
    fn padding_snapshot_prunes_inactive_overrides() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        terminal.widget_padding_default = 2.0;
        terminal
            .widget_padding_overrides
            .insert(WidgetPaddingTargetConfig::Watchlist, 8.0);
        terminal
            .widget_padding_overrides
            .insert(WidgetPaddingTargetConfig::Chart { chart_id: 999 }, 16.0);

        let snapshot = terminal.widget_padding_config_snapshot();

        assert_eq!(snapshot.default_px, 2.0);
        assert_eq!(
            snapshot.overrides,
            vec![WidgetPaddingOverrideConfig {
                target: WidgetPaddingTargetConfig::Watchlist,
                padding_px: 8.0,
            }]
        );
    }

    #[test]
    fn apply_layout_restores_widget_padding_config() {
        let (mut terminal, _task) = TradingTerminal::boot_from_config(KeroseneConfig::default());
        let mut layout = terminal.saved_layout_snapshot("padded".to_string());
        layout.widget_padding = crate::config::WidgetPaddingConfig {
            default_px: 3.0,
            overrides: vec![WidgetPaddingOverrideConfig {
                target: WidgetPaddingTargetConfig::Watchlist,
                padding_px: 9.0,
            }],
        };

        let _task = terminal.apply_layout(layout);

        assert_eq!(terminal.widget_padding_default, 3.0);
        assert_eq!(
            terminal
                .widget_padding_overrides
                .get(&WidgetPaddingTargetConfig::Watchlist),
            Some(&9.0)
        );
    }
}
