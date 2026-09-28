use crate::account::OpenOrder;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::signing::ChaseOrder;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SpotAutomationSymbolIdentity {
    key: String,
    ticker: String,
    display_name: Option<String>,
    asset_index: u32,
    quote_token: Option<u32>,
    sz_decimals: u32,
}

impl SpotAutomationSymbolIdentity {
    fn from_symbol(symbol: &crate::api::ExchangeSymbol) -> Option<Self> {
        (symbol.market_type == MarketType::Spot).then(|| Self {
            key: symbol.key.clone(),
            ticker: symbol.ticker.clone(),
            display_name: symbol.display_name.clone(),
            asset_index: symbol.asset_index,
            quote_token: symbol.collateral_token,
            sz_decimals: symbol.sz_decimals,
        })
    }

    fn matches(&self, symbol: &crate::api::ExchangeSymbol) -> bool {
        symbol.market_type == MarketType::Spot
            && self.key == symbol.key
            && self.ticker == symbol.ticker
            && self.display_name == symbol.display_name
            && self.asset_index == symbol.asset_index
            && self.quote_token == symbol.collateral_token
            && self.sz_decimals == symbol.sz_decimals
    }
}

fn chase_open_order_side_is_buy(side: &str) -> Option<bool> {
    match side {
        "B" => Some(true),
        "A" => Some(false),
        _ => None,
    }
}

pub(crate) fn open_order_matches_chase_identity(chase: &ChaseOrder, order: &OpenOrder) -> bool {
    chase.tracks_oid(order.oid)
        && order.coin == chase.coin
        && chase_open_order_side_is_buy(&order.side) == Some(chase.is_buy)
        && (chase.is_spot || order.reduce_only == Some(chase.reduce_only))
}

impl TradingTerminal {
    pub(crate) fn record_chase_spot_symbol_identity(
        &mut self,
        chase_id: u64,
        symbol: &crate::api::ExchangeSymbol,
    ) {
        if let Some(identity) = SpotAutomationSymbolIdentity::from_symbol(symbol) {
            self.chase_spot_symbol_identities.insert(chase_id, identity);
        }
    }

    pub(crate) fn record_twap_spot_symbol_identity(
        &mut self,
        twap_id: u64,
        symbol: &crate::api::ExchangeSymbol,
    ) {
        if let Some(identity) = SpotAutomationSymbolIdentity::from_symbol(symbol) {
            self.twap_spot_symbol_identities.insert(twap_id, identity);
        }
    }

    pub(crate) fn chase_spot_symbol_identity_is_current(
        &self,
        chase_id: u64,
        symbol_key: &str,
    ) -> bool {
        self.chase_spot_symbol_identities
            .get(&chase_id)
            .zip(self.exchange_symbol_for_key(symbol_key))
            .is_some_and(|(identity, symbol)| identity.matches(symbol))
    }

    pub(crate) fn twap_spot_symbol_identity_is_current(
        &self,
        twap_id: u64,
        symbol_key: &str,
    ) -> bool {
        self.twap_spot_symbol_identities
            .get(&twap_id)
            .zip(self.exchange_symbol_for_key(symbol_key))
            .is_some_and(|(identity, symbol)| identity.matches(symbol))
    }
}
