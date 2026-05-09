use crate::account::{AccountData, WalletDetailsData};
use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Muted Ticker Data Filtering
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn filter_account_data_for_muted_tickers_with(
        exchange_symbols: &[ExchangeSymbol],
        muted_tickers: &HashSet<String>,
        mut data: AccountData,
    ) -> AccountData {
        let is_muted =
            |symbol: &str| Self::key_matches_muted_tickers(exchange_symbols, muted_tickers, symbol);

        data.clearinghouse
            .asset_positions
            .retain(|position| !is_muted(&position.position.coin));
        data.open_orders.retain(|order| !is_muted(&order.coin));
        data.fills.retain(|fill| !is_muted(&fill.coin));
        data.funding_history
            .retain(|entry| !is_muted(&entry.delta.coin));
        data.spot
            .balances
            .retain(|balance| !is_muted(&balance.coin));

        data
    }

    pub(crate) fn filter_account_data_for_muted_tickers(&self, data: AccountData) -> AccountData {
        Self::filter_account_data_for_muted_tickers_with(
            &self.exchange_symbols,
            &self.muted_tickers,
            data,
        )
    }

    pub(crate) fn filter_wallet_details_for_muted_tickers_with(
        exchange_symbols: &[ExchangeSymbol],
        muted_tickers: &HashSet<String>,
        mut data: WalletDetailsData,
    ) -> WalletDetailsData {
        let is_muted =
            |symbol: &str| Self::key_matches_muted_tickers(exchange_symbols, muted_tickers, symbol);

        data.clearinghouse
            .asset_positions
            .retain(|position| !is_muted(&position.position.coin));
        data.positions
            .retain(|position| !is_muted(&position.asset_position.position.coin));
        data.open_orders
            .retain(|order| !is_muted(&order.order.coin));
        data.spot
            .balances
            .retain(|balance| !is_muted(&balance.coin));

        data
    }
}
