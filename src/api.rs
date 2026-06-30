mod calendar;
mod candles;
mod chart_asset_context;
mod exchange_symbols;
mod hype_etfs;
mod hype_unstaking_queue;
mod order_book;
mod order_status;
mod outcome_volume;
mod sec;
mod user_fills;
mod watchlist;

pub use calendar::{CalendarEvent, fetch_economic_calendar};
pub use candles::{
    Candle, candles_have_interior_gap, fetch_candles, fetch_chart_backfill_candles,
    is_valid_candle, normalize_candles, open_time_starts_after_gap, trailing_contiguous_run_start,
};
pub(crate) use chart_asset_context::fetch_chart_asset_context;
pub use exchange_symbols::{
    ExchangeSymbol, ExchangeSymbolsPayload, MarketType, OutcomeSymbolInfo, fetch_exchange_symbols,
    fetch_exchange_symbols_cached,
};
pub(crate) use hype_etfs::fetch_hype_etfs;
pub(crate) use hype_unstaking_queue::fetch_hype_unstaking_queue;
pub use order_book::{BookLevel, OrderBook, fetch_order_book, parse_ws_book};
pub(crate) use order_status::{
    OrderStatusResult, fetch_order_status_by_cloid, fetch_order_status_by_oid,
};
pub(crate) use outcome_volume::{OutcomeVolume24h, fetch_outcome_volumes_24h};
pub(crate) use sec::{SecEarningsEvent, fetch_sec_earnings_events, sec_filing_document_url};
pub use user_fills::{UserFill, UserFillsPage, UserFillsRequest, fetch_user_fills};
pub use watchlist::{
    WatchlistContext, fetch_screener_history, fetch_watchlist_contexts, fetch_watchlist_history,
};

use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use std::sync::LazyLock;

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let mut headers = HeaderMap::new();
    if let Ok(user_agent) = HeaderValue::from_str(KEROSENE_USER_AGENT) {
        headers.insert(USER_AGENT, user_agent);
    }
    Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(15))
        .connect_timeout(std::time::Duration::from_secs(5))
        .pool_idle_timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| Client::new())
});

pub(crate) const KEROSENE_USER_AGENT: &str = concat!("Kerosene/", env!("CARGO_PKG_VERSION"));

pub const API_URL: &str = "https://api.hyperliquid.xyz/info";
pub const OUTCOME_ASSET_ID_OFFSET: u32 = 100_000_000;
pub const USDC_TOKEN_INDEX: u32 = 0;
pub const USDH_TOKEN_INDEX: u32 = 360;
