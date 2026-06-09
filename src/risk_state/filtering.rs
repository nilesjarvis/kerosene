use crate::account::WalletDetailsData;
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
