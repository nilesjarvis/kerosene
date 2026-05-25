use crate::account::{AccountData, ClearinghouseState};
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::parse_finite_number;

// ---------------------------------------------------------------------------
// Account Visibility
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn visible_clearinghouse_state<'a>(
        &self,
        data: &'a AccountData,
    ) -> &'a ClearinghouseState {
        self.market_universe
            .selected_hip3_dex()
            .and_then(|dex| data.clearinghouses_by_dex.get(dex))
            .unwrap_or(&data.clearinghouse)
    }

    pub(crate) fn market_universe_includes_spot(&self) -> bool {
        self.market_universe.selected_hip3_dex().is_none()
    }

    pub(crate) fn account_view_includes_spot_balances(&self, data: &AccountData) -> bool {
        self.market_universe_includes_spot() || data.is_portfolio_margin()
    }

    pub(crate) fn account_spot_balance_is_hidden(&self, data: &AccountData, coin: &str) -> bool {
        let outcome_trade_coin = self.outcome_trade_coin_for_balance_coin(coin);
        if let Some(trade_coin) = Self::outcome_balance_coin_to_trade_coin(coin)
            && self.exchange_symbols.iter().any(|symbol| {
                symbol.key == trade_coin
                    && symbol.market_type == MarketType::Outcome
                    && !symbol.is_user_selectable_market()
            })
        {
            return true;
        }

        if data.is_portfolio_margin() {
            self.is_ticker_muted(coin)
                || outcome_trade_coin
                    .as_deref()
                    .is_some_and(|trade_coin| self.is_ticker_muted(trade_coin))
        } else {
            outcome_trade_coin
                .as_deref()
                .map(|trade_coin| self.symbol_key_is_hidden(trade_coin))
                .unwrap_or_else(|| self.symbol_key_is_hidden(coin))
        }
    }

    pub(crate) fn visible_collateral_token(&self) -> Option<u32> {
        self.market_universe
            .selected_hip3_dex()
            .and_then(|selected_dex| {
                self.exchange_symbols.iter().find_map(|symbol| {
                    if symbol.market_type == MarketType::Perp
                        && symbol.key.split_once(':').is_some_and(|(symbol_dex, _)| {
                            symbol_dex.eq_ignore_ascii_case(selected_dex)
                        })
                    {
                        symbol.collateral_token
                    } else {
                        None
                    }
                })
            })
            .or(Some(0).filter(|_| self.market_universe.selected_hip3_dex().is_none()))
    }

    pub(crate) fn visible_available_margin_usdc(&self, data: &AccountData) -> Option<f64> {
        if data.is_portfolio_margin() {
            return data.available_margin_usdc();
        }

        if data.uses_shared_account_balance() {
            return self
                .visible_collateral_token()
                .and_then(|token| data.available_margin_for_token(token));
        }

        if self.market_universe.selected_hip3_dex().is_some() {
            return parse_finite_number(&self.visible_clearinghouse_state(data).withdrawable);
        }

        data.available_margin_usdc()
    }
}
