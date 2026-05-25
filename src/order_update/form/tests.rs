use super::{TradingTerminal, position_size_for_symbol};
use crate::account::{
    AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
    Position, PositionLeverage, SpotClearinghouseState, UserFeeRates,
};
use crate::api::{BookLevel, ExchangeSymbol, MarketType, OrderBook};
use crate::market_state::{OrderBookInstance, OrderBookSymbolMode};
use crate::signing::OrderKind;

mod order_book;
mod reduce_only;
mod reference_price;

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

fn book(best_bid: f64, best_ask: f64) -> OrderBook {
    OrderBook {
        bids: vec![BookLevel {
            px: best_bid,
            sz: 1.0,
        }],
        asks: vec![BookLevel {
            px: best_ask,
            sz: 1.0,
        }],
    }
}

fn terminal_with_order_book(mode: OrderBookSymbolMode) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_books.clear();
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.order_kind = OrderKind::Market;
    terminal.order_price.clear();
    terminal
        .order_books
        .insert(7, OrderBookInstance::new(7, mode, 1.0));
    terminal
}

fn set_order_book(terminal: &mut TradingTerminal, pane_id: u64, book: OrderBook) {
    let Some(instance) = terminal.order_books.get_mut(&pane_id) else {
        panic!("test order book");
    };
    instance.set_book(book);
}

fn account_data_with_positions(positions: Vec<AssetPosition>) -> AccountData {
    AccountData {
        fetch_scope: Default::default(),
        request_weight_estimate: 0,
        account_abstraction: Default::default(),
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".to_string(),
                total_ntl_pos: "0".to_string(),
                total_margin_used: "0".to_string(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "1000".to_string(),
            asset_positions: positions,
        },
        clearinghouses_by_dex: std::collections::HashMap::new(),
        spot: SpotClearinghouseState {
            balances: Vec::new(),
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        open_orders: Vec::new(),
        fills: Vec::new(),
        funding_history: Vec::new(),
        fee_rates: UserFeeRates::default(),
        completeness: AccountDataCompleteness::default(),
        fetched_at_ms: TradingTerminal::now_ms(),
    }
}

fn asset_position(coin: &str, szi: &str) -> AssetPosition {
    AssetPosition {
        position: Position {
            coin: coin.to_string(),
            szi: szi.to_string(),
            entry_px: "100".to_string(),
            position_value: "0".to_string(),
            unrealized_pnl: "0".to_string(),
            liquidation_px: None,
            leverage: PositionLeverage {
                leverage_type: "cross".to_string(),
                value: 10,
            },
            margin_used: "0".to_string(),
            cum_funding: None,
        },
        liquidation_px: None,
    }
}

fn terminal_with_position(coin: &str, szi: &str) -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.active_symbol = "BTC".to_string();
    terminal.active_symbol_display = "BTC".to_string();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.order_kind = OrderKind::Market;
    terminal.order_price.clear();
    terminal.account_data = Some(account_data_with_positions(vec![asset_position(coin, szi)]));
    terminal
}
