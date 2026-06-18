use crate::app_state::TradingTerminal;
use crate::config;
use crate::timeframe::TIMEFRAME_HOTKEY_OPTIONS;

pub(crate) struct HotkeyActionGroup {
    pub(crate) title: &'static str,
    pub(crate) actions: Vec<(config::HotkeyAction, String)>,
}

impl TradingTerminal {
    pub(crate) fn available_hotkey_action_groups(&self) -> Vec<HotkeyActionGroup> {
        let mut groups = vec![HotkeyActionGroup {
            title: "General",
            actions: vec![
                (
                    config::HotkeyAction::AddCandlestickChart,
                    "Add Candlestick Chart".to_string(),
                ),
                (
                    config::HotkeyAction::ChartTimeframePrefix,
                    format!("Chart Timeframes 1..{}", TIMEFRAME_HOTKEY_OPTIONS.len()),
                ),
                (config::HotkeyAction::OpenAlfred, "alfred".to_string()),
                (
                    config::HotkeyAction::OpenTradingJournal,
                    "Open Trading Journal".to_string(),
                ),
                (
                    config::HotkeyAction::OpenWalletTracker,
                    "Open Wallet Tracker".to_string(),
                ),
                (
                    config::HotkeyAction::OpenQuickSymbolSearch,
                    "Quick Symbol Search".to_string(),
                ),
                (
                    config::HotkeyAction::OpenSettingsWindow,
                    "Open Settings".to_string(),
                ),
            ],
        }];

        if !self.saved_layouts.is_empty() {
            groups.push(HotkeyActionGroup {
                title: "Layouts",
                actions: self
                    .saved_layouts
                    .iter()
                    .map(|layout| {
                        (
                            config::HotkeyAction::SwitchLayout {
                                name: layout.name.clone(),
                            },
                            layout.name.clone(),
                        )
                    })
                    .collect(),
            });
        }

        let account_actions: Vec<_> = self
            .accounts
            .iter()
            .filter(|profile| !self.ghost_account_secret_ids.contains(&profile.secret_id))
            .map(|profile| {
                (
                    config::HotkeyAction::SwitchAccount {
                        secret_id: profile.secret_id.clone(),
                    },
                    profile.name.clone(),
                )
            })
            .collect();
        if !account_actions.is_empty() {
            groups.push(HotkeyActionGroup {
                title: "Accounts",
                actions: account_actions,
            });
        }

        groups
    }
}
