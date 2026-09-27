use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::{LiveWatchlistId, LiveWatchlistInstance};
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn ensure_live_watchlist_pane_instances(&mut self) {
        let missing_ids = self
            .workspace_pane_kinds()
            .filter_map(|(_, _, kind)| match kind {
                PaneKind::LiveWatchlist(id) if !self.live_watchlists.contains_key(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>();
        for id in missing_ids {
            self.insert_default_live_watchlist(id);
        }
    }

    pub(super) fn add_live_watchlist_pane(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        let workspace = self.add_widget_workspace;
        let Some(focus) = self.add_target_pane_in(workspace) else {
            self.push_toast(
                "Could not add Live Watchlist: no pane is available".to_string(),
                true,
            );
            return Task::none();
        };

        let id = crate::ws::now_ms();
        self.insert_default_live_watchlist(id);
        if self
            .add_pane_to_target(
                workspace,
                self.add_widget_axis(),
                focus,
                PaneKind::LiveWatchlist(id),
                "Live Watchlist",
            )
            .is_none()
        {
            self.live_watchlists.remove(&id);
        }
        Task::none()
    }

    fn insert_default_live_watchlist(&mut self, id: LiveWatchlistId) {
        let preset_id = self.ensure_default_watchlist_preset();
        let symbols = self
            .watchlist_preset(preset_id)
            .map(|preset| {
                preset
                    .symbols
                    .iter()
                    .filter(|symbol| !self.symbol_key_is_hidden(symbol))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        self.live_watchlists.insert(
            id,
            LiveWatchlistInstance {
                id,
                preset_id: Some(preset_id),
                symbols,
                search_query: String::new(),
                sort_column: Default::default(),
                sort_direction: Default::default(),
                visible_columns: config::default_live_watchlist_columns(),
                row_cache: Vec::new(),
            },
        );
    }
}
