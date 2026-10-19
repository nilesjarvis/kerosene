use crate::app_state::TradingTerminal;
use crate::config::{OrderBookSymbolModeConfig, TradeFeedConfig};
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::trade_feed_state::{TradeFeedInstance, TradeFeedSource, parse_trade_feed_minimum};
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
            Message::TradeFeedToggleGrouping(id) => {
                if let Some(feed) = self.trade_feeds.instances.get_mut(&id) {
                    feed.group_trades = !feed.group_trades;
                    self.persist_config();
                }
            }
            Message::TradeFeedToggleSizeUnit(id) => {
                if let Some(feed) = self.trade_feeds.instances.get_mut(&id) {
                    feed.size_in_notional = !feed.size_in_notional;
                    self.persist_config();
                }
            }
            Message::TradeFeedMinimumChanged(id, input) => {
                if input.len() <= 64
                    && let Some(feed) = self.trade_feeds.instances.get_mut(&id)
                {
                    let minimum = parse_trade_feed_minimum(&input);
                    feed.min_group_notional_input = input;
                    if let Some(minimum) = minimum
                        && feed.min_group_notional != minimum
                    {
                        feed.min_group_notional = minimum;
                        self.persist_config();
                    }
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
            Message::TradeFeedToggleContext(id) => {
                if let Some(feed) = self.trade_feeds.instances.get_mut(&id) {
                    feed.show_context = !feed.show_context;
                    feed.failed_context_generation = None;
                    self.persist_config();
                }
            }
            Message::TradeFeedRetryContext(id) => {
                if let Some(feed) = self.trade_feeds.instances.get_mut(&id) {
                    feed.failed_context_generation = None;
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
                    return Some((id, None, TradeFeedSource::Hyperliquid, true));
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
                let source = if feed.show_context
                    && !self.hydromancer_api_key.trim().is_empty()
                    && (feed.symbol.as_deref() != symbol
                        || feed.failed_context_generation != Some(self.hydromancer_key_generation))
                {
                    TradeFeedSource::Hydromancer {
                        key_generation: self.hydromancer_key_generation,
                    }
                } else {
                    TradeFeedSource::Hyperliquid
                };
                (feed.symbol.as_deref() != symbol || feed.source != source || feed.generation == 0)
                    .then(|| (id, symbol.map(str::to_owned), source, false))
            })
            .collect();
        for (id, symbol, source, remove) in updates {
            if remove {
                self.trade_feeds.instances.remove(&id);
            } else if let Some(feed) = self.trade_feeds.instances.get_mut(&id) {
                if feed.symbol != symbol || matches!(source, TradeFeedSource::Hydromancer { .. }) {
                    feed.failed_context_generation = None;
                }
                self.trade_feeds.generation = self.trade_feeds.generation.wrapping_add(1);
                feed.reset(symbol, self.trade_feeds.generation);
                feed.source = source;
                feed.has_gap = feed.failed_context_generation.is_some();
            }
        }
        for (&id, feed) in &mut self.trade_feeds.instances {
            feed.sync_address_hover(id, self.hovered_wallet_address_actions.as_deref());
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
            let config = configs.iter().find(|config| config.id == id);
            let mode = config.map(|config| config.mode.clone()).unwrap_or_default();
            let mut feed = TradeFeedInstance::new(mode);
            feed.group_trades = config.is_some_and(|config| config.group_trades);
            feed.min_group_notional = config
                .map(|config| config.min_group_notional)
                .filter(|minimum| minimum.is_finite() && *minimum > 0.0)
                .unwrap_or_default();
            if feed.min_group_notional > 0.0 {
                feed.min_group_notional_input = feed.min_group_notional.to_string();
                if feed.min_group_notional_input.len() > 64 {
                    feed.min_group_notional_input = format!("{:e}", feed.min_group_notional);
                }
            }
            feed.size_in_notional = config.is_some_and(|config| config.size_in_notional);
            feed.show_context = config.is_some_and(|config| config.show_context);
            self.trade_feeds.instances.insert(id, feed);
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
                group_trades: feed.group_trades,
                min_group_notional: feed.min_group_notional,
                size_in_notional: feed.size_in_notional,
                show_context: feed.show_context,
            })
            .collect();
        configs.sort_by_key(|config| config.id);
        configs
    }
}

#[cfg(test)]
mod tests;
