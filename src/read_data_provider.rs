use crate::app_state::TradingTerminal;
use crate::config::ReadDataProvider;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Read Data Provider Selection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReadDataRequestContext {
    pub(crate) provider: ReadDataProvider,
    pub(crate) read_data_provider_generation: u64,
    pub(crate) hydromancer_key_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MarketDataSourceContext {
    pub(crate) provider: ReadDataProvider,
    pub(crate) read_data_provider_generation: u64,
    pub(crate) hydromancer_key_generation: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccountDataRequestContext {
    pub(crate) read_data: ReadDataRequestContext,
    pub(crate) scope: AccountDataRequestScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AccountDataRequestScope {
    ConnectedSnapshot { generation: u64 },
    TwapReconciliation { generation: u64 },
}

impl AccountDataRequestContext {
    pub(crate) fn connected_snapshot(read_data: ReadDataRequestContext, generation: u64) -> Self {
        Self {
            read_data,
            scope: AccountDataRequestScope::ConnectedSnapshot { generation },
        }
    }

    pub(crate) fn twap_reconciliation(read_data: ReadDataRequestContext, generation: u64) -> Self {
        Self {
            read_data,
            scope: AccountDataRequestScope::TwapReconciliation { generation },
        }
    }
}

impl TradingTerminal {
    pub(crate) fn read_data_request_context(&self) -> ReadDataRequestContext {
        ReadDataRequestContext {
            provider: self.read_data_provider,
            read_data_provider_generation: self.read_data_provider_generation,
            hydromancer_key_generation: self.hydromancer_key_generation,
        }
    }

    pub(crate) fn read_data_request_context_is_current(
        &self,
        context: ReadDataRequestContext,
    ) -> bool {
        self.read_data_provider == context.provider
            && self.read_data_provider_generation == context.read_data_provider_generation
            && (context.provider != ReadDataProvider::Hydromancer
                || self.hydromancer_key_generation_is_current(context.hydromancer_key_generation))
    }

    pub(crate) fn market_data_source_context(&self) -> MarketDataSourceContext {
        MarketDataSourceContext {
            provider: self.read_data_provider,
            read_data_provider_generation: self.read_data_provider_generation,
            hydromancer_key_generation: self
                .hydromancer_read_provider_enabled()
                .then_some(self.hydromancer_key_generation),
        }
    }

    pub(crate) fn hydromancer_keyed_market_data_source_context(&self) -> MarketDataSourceContext {
        MarketDataSourceContext {
            provider: self.read_data_provider,
            read_data_provider_generation: self.read_data_provider_generation,
            hydromancer_key_generation: (!self.hydromancer_api_key.trim().is_empty())
                .then_some(self.hydromancer_key_generation),
        }
    }

    pub(crate) fn current_account_data_request_context(&self) -> AccountDataRequestContext {
        AccountDataRequestContext::connected_snapshot(
            self.read_data_request_context(),
            self.account_data_request_generation,
        )
    }

    pub(crate) fn begin_account_data_request_context(&mut self) -> AccountDataRequestContext {
        self.account_data_request_generation = self.account_data_request_generation.wrapping_add(1);
        self.current_account_data_request_context()
    }

    pub(crate) fn begin_twap_reconciliation_account_data_request_context(
        &mut self,
        address: &str,
    ) -> AccountDataRequestContext {
        let generation = self
            .account_twap_reconciliation_generations
            .get(address)
            .copied()
            .unwrap_or_default()
            .wrapping_add(1);
        self.account_twap_reconciliation_generations
            .insert(address.to_string(), generation);
        AccountDataRequestContext::twap_reconciliation(self.read_data_request_context(), generation)
    }

    pub(crate) fn invalidate_account_data_requests(&mut self) {
        self.account_data_request_generation = self.account_data_request_generation.wrapping_add(1);
    }

    pub(crate) fn account_data_request_generation_is_current(
        &self,
        address: &str,
        context: AccountDataRequestContext,
    ) -> bool {
        match context.scope {
            AccountDataRequestScope::ConnectedSnapshot { generation } => {
                generation == self.account_data_request_generation
            }
            AccountDataRequestScope::TwapReconciliation { generation } => self
                .account_twap_reconciliation_generations
                .get(address)
                .is_some_and(|current_generation| *current_generation == generation),
        }
    }

    pub(crate) fn market_stream_source_is_current(&self, context: MarketDataSourceContext) -> bool {
        if self.read_data_provider != context.provider
            || self.read_data_provider_generation != context.read_data_provider_generation
        {
            return false;
        }

        match context.hydromancer_key_generation {
            Some(generation) => {
                self.hydromancer_read_provider_enabled()
                    && self.hydromancer_key_generation_is_current(generation)
            }
            None => true,
        }
    }

    pub(crate) fn hydromancer_keyed_market_stream_source_is_current(
        &self,
        context: MarketDataSourceContext,
    ) -> bool {
        if self.read_data_provider != context.provider
            || self.read_data_provider_generation != context.read_data_provider_generation
        {
            return false;
        }

        context
            .hydromancer_key_generation
            .is_some_and(|generation| {
                !self.hydromancer_api_key.trim().is_empty()
                    && self.hydromancer_key_generation_is_current(generation)
            })
    }

    pub(crate) fn hydromancer_read_provider_enabled(&self) -> bool {
        self.read_data_provider == ReadDataProvider::Hydromancer
            && !self.hydromancer_api_key.trim().is_empty()
    }

    pub(crate) fn hydromancer_read_provider_key(&self) -> Option<Zeroizing<String>> {
        self.hydromancer_read_provider_enabled()
            .then(|| Zeroizing::new(self.hydromancer_api_key.trim().to_string()))
    }

    pub(crate) fn invalidate_wallet_read_data_requests(&mut self) {
        for state in self.wallet_detail_windows.values_mut() {
            if state.loading {
                state.loading = false;
                state.loading_context = None;
                if state.data.is_none() && state.error.is_none() {
                    state.error = Some(
                        "Wallet detail refresh was interrupted by read data provider change"
                            .to_string(),
                    );
                }
            }
        }

        let mut core_refreshes = Vec::new();
        let mut order_refreshes = Vec::new();
        for (address, row) in &mut self.wallet_tracker.rows {
            if row.loading {
                row.loading = false;
                row.loading_context = None;
                core_refreshes.push(address.clone());
            }
            if row.order_loading {
                row.order_loading = false;
                row.order_loading_context = None;
                order_refreshes.push(address.clone());
            }
        }
        for address in core_refreshes {
            self.queue_wallet_tracker_core_refresh(address);
        }
        for address in order_refreshes {
            self.queue_wallet_tracker_order_refresh(address);
        }
    }
}

pub(crate) fn fallback_warning(scope: &str, error: &str) -> String {
    format!(
        "Hydromancer {scope} failed; used Hyperliquid fallback: {}",
        provider_error_summary(error)
    )
}

fn provider_error_summary(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("401")
        || lower.contains("403")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("invalid api key")
        || lower.contains("invalid token")
        || lower.contains("authentication")
    {
        return "authentication failed".to_string();
    }

    crate::helpers::text_excerpt(error, 160)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::WalletTrackerSnapshot;
    use crate::config::ReadDataProvider;
    use crate::wallet_state::WalletDetailsWindowState;

    const TEST_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

    fn read_context(
        terminal: &TradingTerminal,
        provider: ReadDataProvider,
        hydromancer_key_generation: u64,
    ) -> ReadDataRequestContext {
        ReadDataRequestContext {
            provider,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation,
        }
    }

    #[test]
    fn read_data_context_rejects_stale_hydromancer_generation() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_key_generation = 2;

        assert!(!terminal.read_data_request_context_is_current(read_context(
            &terminal,
            ReadDataProvider::Hydromancer,
            1
        )));
        assert!(terminal.read_data_request_context_is_current(read_context(
            &terminal,
            ReadDataProvider::Hydromancer,
            2
        )));
    }

    #[test]
    fn read_data_context_rejects_provider_mismatch_but_hyperliquid_ignores_key_generation() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.read_data_provider = ReadDataProvider::Hyperliquid;
        terminal.hydromancer_key_generation = 9;

        assert!(!terminal.read_data_request_context_is_current(read_context(
            &terminal,
            ReadDataProvider::Hydromancer,
            9
        )));
        assert!(terminal.read_data_request_context_is_current(read_context(
            &terminal,
            ReadDataProvider::Hyperliquid,
            1
        )));
    }

    #[test]
    fn read_data_context_rejects_provider_generation_after_away_and_back_toggle() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.read_data_provider = ReadDataProvider::Hyperliquid;
        let stale_context = terminal.read_data_request_context();

        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.bump_read_data_provider_generation();
        terminal.read_data_provider = ReadDataProvider::Hyperliquid;
        terminal.bump_read_data_provider_generation();

        assert!(!terminal.read_data_request_context_is_current(stale_context));
        assert!(
            terminal.read_data_request_context_is_current(terminal.read_data_request_context())
        );
    }

    #[test]
    fn hydromancer_read_provider_key_returns_trimmed_task_key() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_api_key = "  hydro-secret  ".to_string().into();

        let key = terminal
            .hydromancer_read_provider_key()
            .expect("hydromancer key");

        assert_eq!(key.as_str(), "hydro-secret");
    }

    #[test]
    fn market_stream_source_scope_tracks_provider_and_hydromancer_key() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.hydromancer_key_generation = 2;

        terminal.read_data_provider = ReadDataProvider::Hyperliquid;
        terminal.hydromancer_api_key = "hydro-secret".to_string().into();
        let hyperliquid_context = terminal.market_data_source_context();
        assert!(terminal.market_stream_source_is_current(hyperliquid_context));
        assert!(
            !terminal.market_stream_source_is_current(MarketDataSourceContext {
                hydromancer_key_generation: Some(2),
                ..hyperliquid_context
            })
        );

        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.bump_read_data_provider_generation();
        terminal.hydromancer_api_key = String::new().into();
        let fallback_context = terminal.market_data_source_context();
        assert!(terminal.market_stream_source_is_current(fallback_context));
        assert!(!terminal.market_stream_source_is_current(hyperliquid_context));
        assert!(
            !terminal.market_stream_source_is_current(MarketDataSourceContext {
                hydromancer_key_generation: Some(2),
                ..fallback_context
            })
        );

        terminal.hydromancer_api_key = "hydro-secret".to_string().into();
        let hydromancer_context = terminal.market_data_source_context();
        assert!(terminal.market_stream_source_is_current(fallback_context));
        assert!(
            !terminal.market_stream_source_is_current(MarketDataSourceContext {
                hydromancer_key_generation: Some(1),
                ..hydromancer_context
            })
        );
        assert!(terminal.market_stream_source_is_current(hydromancer_context));
    }

    #[test]
    fn hydromancer_keyed_market_stream_context_allows_keyed_streams_without_provider_switch() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.read_data_provider = ReadDataProvider::Hyperliquid;
        terminal.hydromancer_api_key = "hydro-secret".to_string().into();
        terminal.hydromancer_key_generation = 2;

        let context = terminal.hydromancer_keyed_market_data_source_context();

        assert!(terminal.hydromancer_keyed_market_stream_source_is_current(context));
        assert!(!terminal.hydromancer_keyed_market_stream_source_is_current(
            MarketDataSourceContext {
                hydromancer_key_generation: Some(1),
                ..context
            }
        ));

        terminal.hydromancer_api_key = String::new().into();
        assert!(!terminal.hydromancer_keyed_market_stream_source_is_current(context));
    }

    #[test]
    fn hydromancer_key_generation_change_unblocks_wallet_read_requests() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_key_generation = 1;
        let context = terminal.read_data_request_context();

        let window_id = iced::window::Id::unique();
        let mut details = WalletDetailsWindowState::new(TEST_ADDRESS.to_string());
        details.loading_context = Some(context);
        terminal.wallet_detail_windows.insert(window_id, details);

        terminal
            .wallet_tracker
            .tracked_addresses
            .push(TEST_ADDRESS.to_string());
        let row = terminal
            .wallet_tracker
            .rows
            .entry(TEST_ADDRESS.to_string())
            .or_default();
        row.loading = true;
        row.loading_context = Some(context);
        row.order_loading = true;
        row.order_loading_context = Some(context);
        row.snapshot = Some(wallet_tracker_snapshot());

        terminal.bump_hydromancer_key_generation();

        let details = terminal
            .wallet_detail_windows
            .get(&window_id)
            .expect("details window");
        assert!(!details.loading);
        assert_eq!(details.loading_context, None);
        assert!(
            details
                .error
                .as_deref()
                .is_some_and(|error| error.contains("read data provider change"))
        );

        let row = terminal
            .wallet_tracker
            .rows
            .get(TEST_ADDRESS)
            .expect("tracker row");
        assert!(!row.loading);
        assert_eq!(row.loading_context, None);
        assert!(!row.order_loading);
        assert_eq!(row.order_loading_context, None);
        assert_eq!(
            terminal.wallet_tracker_next_core_address(TradingTerminal::now_ms()),
            Some(TEST_ADDRESS.to_string())
        );
        assert_eq!(
            terminal.wallet_tracker_next_order_address(TradingTerminal::now_ms()),
            Some(TEST_ADDRESS.to_string())
        );
    }

    fn wallet_tracker_snapshot() -> WalletTrackerSnapshot {
        WalletTrackerSnapshot {
            equity: Some(100.0),
            withdrawable: Some(50.0),
            unrealized_pnl: Some(1.0),
            margin_used_pct: Some(0.1),
            open_trade_count: Some(1),
            open_order_count: 2,
            long_exposure: Some(10.0),
            short_exposure: Some(0.0),
        }
    }
}
