use crate::account::{AccountData, WalletDetailsData};
use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::config::MarketUniverseConfig;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Muted Ticker Data Filtering
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn symbol_key_is_hidden_with(
        exchange_symbols: &[ExchangeSymbol],
        muted_tickers: &HashSet<String>,
        market_universe: &MarketUniverseConfig,
        symbol: &str,
    ) -> bool {
        !Self::key_matches_market_universe(exchange_symbols, market_universe, symbol)
            || Self::key_matches_muted_tickers(exchange_symbols, muted_tickers, symbol)
    }

    pub(crate) fn filter_account_data_for_hidden_symbols_with(
        exchange_symbols: &[ExchangeSymbol],
        muted_tickers: &HashSet<String>,
        market_universe: &MarketUniverseConfig,
        mut data: AccountData,
    ) -> AccountData {
        let is_hidden = |symbol: &str| {
            Self::symbol_key_is_hidden_with(
                exchange_symbols,
                muted_tickers,
                market_universe,
                symbol,
            )
        };
        let is_muted =
            |symbol: &str| Self::key_matches_muted_tickers(exchange_symbols, muted_tickers, symbol);

        data.clearinghouse
            .asset_positions
            .retain(|position| !is_hidden(&position.position.coin));
        for state in data.clearinghouses_by_dex.values_mut() {
            state
                .asset_positions
                .retain(|position| !is_hidden(&position.position.coin));
        }
        data.open_orders.retain(|order| !is_hidden(&order.coin));
        data.fills.retain(|fill| !is_hidden(&fill.coin));
        data.funding_history
            .retain(|entry| !is_hidden(&entry.delta.coin));
        data.spot
            .balances
            .retain(|balance| !is_muted(&balance.coin));

        data
    }

    pub(crate) fn filter_account_data_for_hidden_symbols(&self, data: AccountData) -> AccountData {
        Self::filter_account_data_for_hidden_symbols_with(
            &self.exchange_symbols,
            &self.muted_tickers,
            &self.market_universe,
            data,
        )
    }

    pub(crate) fn filter_account_data_for_muted_tickers(&self, data: AccountData) -> AccountData {
        self.filter_account_data_for_hidden_symbols(data)
    }

    pub(crate) fn filter_wallet_details_for_hidden_symbols_with(
        exchange_symbols: &[ExchangeSymbol],
        muted_tickers: &HashSet<String>,
        market_universe: &MarketUniverseConfig,
        mut data: WalletDetailsData,
    ) -> WalletDetailsData {
        let is_hidden = |symbol: &str| {
            Self::symbol_key_is_hidden_with(
                exchange_symbols,
                muted_tickers,
                market_universe,
                symbol,
            )
        };

        data.clearinghouse
            .asset_positions
            .retain(|position| !is_hidden(&position.position.coin));
        data.positions
            .retain(|position| !is_hidden(&position.asset_position.position.coin));
        data.open_orders
            .retain(|order| !is_hidden(&order.order.coin));
        data.spot
            .balances
            .retain(|balance| !is_hidden(&balance.coin));

        data
    }
}
