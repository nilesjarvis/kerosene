use crate::app_state::TradingTerminal;
use crate::config::{OrderBookSymbolModeConfig, TradeFeedConfig};
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::trade_feed_state::TradeFeedInstance;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn update_trade_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddTradeFeedPane => {
                self.add_widget_menu_open = false;
                let workspace = self.add_widget_workspace;
                let Some(target) = self.add_target_pane_in(workspace) else {
                    self.push_toast(
                        "Could not add Trade Feed: no pane is available".into(),
                        true,
                    );
                    return Task::none();
                };
                let id = self.trade_feeds.next_id;
                self.trade_feeds.next_id = id.saturating_add(1);
                self.trade_feeds
                    .instances
                    .insert(id, TradeFeedInstance::new(Default::default()));
                if self
                    .add_pane_to_target(
                        workspace,
                        self.add_widget_axis(),
                        target,
                        PaneKind::TradeFeed(id),
                        "Trade Feed",
                    )
                    .is_none()
                {
                    self.trade_feeds.instances.remove(&id);
                }
            }
            Message::TradeFeedTogglePicker(id) => {
                if let Some(feed) = self.trade_feeds.instances.get_mut(&id) {
                    feed.picker_open = !feed.picker_open;
                    feed.search.clear();
                }
            }
            Message::TradeFeedSearchChanged(id, query) => {
                if let Some(feed) = self.trade_feeds.instances.get_mut(&id) {
                    feed.search = query;
                }
            }
            Message::TradeFeedSetMode(id, mode) => {
                let mode = match mode {
                    OrderBookSymbolModeConfig::Fixed(key) => {
                        let Some(symbol) = self.exchange_symbol_for_key(&key) else {
                            return Task::none();
                        };
                        if !symbol.is_user_selectable_market()
                            || self.exchange_symbol_is_hidden(symbol)
                        {
                            return Task::none();
                        }
                        OrderBookSymbolModeConfig::Fixed(symbol.key.clone())
                    }
                    OrderBookSymbolModeConfig::Active => OrderBookSymbolModeConfig::Active,
                };
                if let Some(feed) = self.trade_feeds.instances.get_mut(&id) {
                    feed.mode = mode;
                    feed.picker_open = false;
                    feed.search.clear();
                    self.persist_config();
                }
            }
            Message::TradeFeedStream(id, generation, coin, event) => {
                if let Some(feed) = self.trade_feeds.instances.get_mut(&id)
                    && feed.generation == generation
                    && feed.symbol.as_deref() == Some(&coin)
                {
                    feed.apply(event);
                }
            }
            _ => {}
        }
        Task::none()
    }

    /// Runs after updates so chart clicks, layout changes, risk filters, and symbol
    /// metadata refreshes all invalidate the same subscription identity.
    pub(crate) fn sync_trade_feeds(&mut self) {
        if self.trade_feeds.instances.is_empty() {
            return;
        }
        let updates: Vec<_> = self
            .trade_feeds
            .instances
            .iter()
            .filter_map(|(&id, feed)| {
                if !self.pane_is_open(
                    |kind| matches!(kind, PaneKind::TradeFeed(pane_id) if *pane_id == id),
                ) {
                    return Some((id, None, true));
                }
                let requested = match &feed.mode {
                    OrderBookSymbolModeConfig::Active => &self.active_symbol,
                    OrderBookSymbolModeConfig::Fixed(key) => key,
                };
                let canonical = self
                    .exchange_symbol_for_key(requested)
                    .map(|symbol| symbol.key.as_str())
                    .unwrap_or(requested);
                let symbol = (!canonical.is_empty()
                    && canonical != "@0"
                    && !self.symbol_key_is_hidden(canonical))
                .then_some(canonical);
                (feed.symbol.as_deref() != symbol || feed.generation == 0)
                    .then(|| (id, symbol.map(str::to_owned), false))
            })
            .collect();
        for (id, symbol, remove) in updates {
            if remove {
                self.trade_feeds.instances.remove(&id);
            } else if let Some(feed) = self.trade_feeds.instances.get_mut(&id) {
                self.trade_feeds.generation = self.trade_feeds.generation.wrapping_add(1);
                feed.reset(symbol, self.trade_feeds.generation);
            }
        }
    }

    pub(crate) fn restore_trade_feeds(&mut self, configs: &[TradeFeedConfig]) {
        self.trade_feeds.instances.clear();
        self.trade_feeds.next_id = 0;
        let ids: Vec<_> = self
            .workspace_pane_kinds()
            .filter_map(|(_, _, kind)| match kind {
                PaneKind::TradeFeed(id) => Some(*id),
                _ => None,
            })
            .collect();
        for id in ids {
            let mode = configs
                .iter()
                .find(|config| config.id == id)
                .map(|config| config.mode.clone())
                .unwrap_or_default();
            self.trade_feeds
                .instances
                .insert(id, TradeFeedInstance::new(mode));
            self.trade_feeds.next_id = self.trade_feeds.next_id.max(id.saturating_add(1));
        }
        self.sync_trade_feeds();
    }

    pub(crate) fn trade_feed_configs_snapshot(&self) -> Vec<TradeFeedConfig> {
        let mut configs: Vec<_> = self
            .trade_feeds
            .instances
            .iter()
            .map(|(&id, feed)| TradeFeedConfig {
                id,
                mode: feed.mode.clone(),
            })
            .collect();
        configs.sort_by_key(|config| config.id);
        configs
    }
}

#[cfg(test)]
mod tests;
