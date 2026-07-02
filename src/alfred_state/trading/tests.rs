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

fn spot_symbol(key: &str, ticker: &str, display_name: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "spot".to_string(),
        display_name: Some(display_name.to_string()),
        keywords: vec!["spot".to_string()],
        asset_index: 10_107,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    }
}

fn terminal_with_hype() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("HYPE", MarketType::Perp)];
    terminal
}

fn terminal_with_hype_perp_and_spot() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    // Spot pair listed first to prove perp preference is a rule, not list order.
    terminal.exchange_symbols = vec![
        spot_symbol("@107", "HYPE", "HYPE/USDC"),
        symbol("HYPE", MarketType::Perp),
    ];
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
