use super::*;
use crate::api::MarketType;

mod drafts;
mod intents;

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn terminal_with_hype() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("HYPE", MarketType::Perp)];
    terminal
}

fn trade_intent_or_panic(query: &str) -> ParsedTradeIntent {
    match parse_trade_intent(query) {
        Some(intent) => intent,
        None => panic!("missing trade intent for {query}"),
    }
}

fn trade_draft_or_panic(terminal: &TradingTerminal, query: &str) -> AlfredTradeDraft {
    match terminal.alfred_trade_draft(query) {
        Some(draft) => draft,
        None => panic!("missing trade draft for {query}"),
    }
}
