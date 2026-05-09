mod calendar;
mod candles;
mod exchange_symbols;
mod order_book;
mod user_fills;
mod watchlist;

pub use calendar::{CalendarEvent, fetch_economic_calendar};
pub use candles::{Candle, fetch_candles, is_valid_candle, normalize_candles};
pub use exchange_symbols::{ExchangeSymbol, MarketType, OutcomeSymbolInfo, fetch_exchange_symbols};
pub use order_book::{BookLevel, OrderBook, fetch_order_book, parse_ws_book};
pub use user_fills::{UserFill, UserFillsPage, UserFillsRequest, fetch_user_fills};
pub use watchlist::{WatchlistContext, fetch_watchlist_contexts, fetch_watchlist_history};

use reqwest::Client;
use std::sync::LazyLock;

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .connect_timeout(std::time::Duration::from_secs(5))
        .pool_idle_timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| Client::new())
});

const KEROSENE_USER_AGENT: &str = concat!("Kerosene/", env!("CARGO_PKG_VERSION"));

pub const API_URL: &str = "https://api-ui.hyperliquid.xyz/info";
pub const OUTCOME_ASSET_ID_OFFSET: u32 = 100_000_000;
pub const USDH_TOKEN_INDEX: u32 = 360;
