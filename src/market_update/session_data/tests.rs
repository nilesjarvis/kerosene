use super::requests::{
    DAY_MS, INTRADAY_CANDLE_MS, INTRADAY_MAX_CANDLES_PER_REQUEST, intraday_chunk_ranges,
};
use super::*;
use crate::api::{ExchangeSymbol, MarketType};
use crate::session_data_state::{SessionDataCandles, SessionDataRequest};

mod requests;
mod symbols;

fn exchange_symbol(key: &str, ticker: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 50,
        only_isolated: false,
        growth_mode: false,
        market_type,
        outcome: None,
    }
}

fn history_instance(id: SessionDataId, symbol: &str) -> SessionDataInstance {
    let mut instance =
        SessionDataInstance::new(id, symbol.to_string(), SessionDataLookback::FourWeeks);
    instance.apply_candles(
        SessionDataCandles {
            daily: vec![crate::api::Candle::test_ohlcv(
                0,
                1,
                [100.0, 110.0, 100.0, 110.0],
                10.0,
            )],
            intraday: Vec::new(),
        },
        2,
    );
    instance.loading = true;
    instance.error = Some("previous error".to_string());
    instance.last_fetch_ms = Some(2);
    instance.search_query = "keep search".to_string();
    instance.symbol_picker_open = true;
    instance.pending_request = Some(SessionDataRequest {
        id,
        symbol: symbol.to_string(),
        lookback: instance.lookback,
        requested_at_ms: 123,
    });
    instance
}
