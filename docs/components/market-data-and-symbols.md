# Market Data And Symbols

The market system owns the tradable symbol universe, real-time prices, order
books, market widgets, watchlists, ticker tape, outcomes, positioning info, and
HYPE-specific informational panes.

It bridges REST snapshots, websocket streams, user preferences, hidden-symbol
risk filters, and pure iced views.

## Component Map

| Component | Key files | Responsibility |
| --- | --- | --- |
| API models and fetches | `src/api/` | Hyperliquid REST info requests, candles, books, symbols, watchlist context/history, outcome volume, HYPE data. |
| Websocket streams | `src/ws/market_streams/`, `src/subscription_state/market/` | Candle, book, asset-context, and comparison-chart streams. |
| Market state | `src/market_state/` | Order book instances, live watchlist models, symbol search, mids, DOM ladder helpers. |
| Market updates | `src/market_update/` | Symbol selection, order book snapshots, watchlists, positioning info, session data, ticker tape, HYPE widgets. |
| Market views | `src/market_views/` | Watchlist/symbol search, live watchlists, order books, ticker tape, outcomes, positioning info, HYPE widgets. |
| Risk filtering | `src/risk_state/` | Muted ticker and market-universe matching used across market/account/order surfaces. |

## Symbol Universe

`api::fetch_exchange_symbols` loads the exchange symbol universe. Symbols are
represented by `ExchangeSymbol` and include:

- main-dex perpetual markets
- HIP-3 dex perpetual markets
- spot pairs
- outcome markets

Symbol selection state lives in `TradingTerminal`:

- `exchange_symbols`
- `active_symbol`
- `active_symbol_display`
- `symbol_search_query`
- `symbol_search_sort_mode`
- `symbol_search_market_filter`
- `symbol_search_hip3_dex_filter`
- `market_universe`
- `outcome_search_query`
- `outcome_collapsed_market_groups`

Symbol search is implemented in `market_state/symbol_search/` and
`market_update/symbols.rs`. It normalizes labels, applies market-universe
filters, hides muted tickers, resolves aliases, and feeds chart/order-book/order
entry selection.

## Spot Metadata And Identity Safety

Perpetual, spot, and outcome metadata families are fetched independently, so a
perpetual metadata outage does not prevent a valid spot universe from loading.
Spot parsing is strict: error-shaped or empty responses, invalid token
references, duplicate indices, malformed pairs, and unsupported precision are
rejected rather than converted into an empty or partially guessed universe.

`ExchangeSymbol` retains both the spot asset index and quote-token identity.
Only a complete live metadata result is cacheable and orderable. A cached
universe is displayed while an immediate live verification runs; if live spot
metadata fails, last-known markets can remain visible but
`spot_metadata_degraded` disables spot trading until verification succeeds.

Cached startup, its immediate live verification, and periodic live metadata
refreshes share one runtime request generation. A completion can update the
symbol universe or release the refresh gate only while its generation still
owns the active startup/loading or background-refresh state; accepting a result
invalidates duplicate delivery before applying the established merge path.

Persisted aliases for API-named spot pairs are migrated only after metadata
proves the mapping. This currently rewrites legacy `@0` to `PURR/USDC` across
regular chart primary/secondary series, fixed order books, spaghetti charts,
and live watchlists, deduplicates collisions, invalidates old requests, and
refetches under the canonical key. Startup and runtime layout restoration defer
raw legacy candle/book requests when metadata is not yet available.

## Active Symbol

The active symbol is the app-level trading context. It drives:

- order entry
- order books in active-symbol mode
- the default chart/watchlist selection behavior
- close-position and chart click workflows
- symbol-specific market metadata and outcome restrictions

Charts can also have independent symbols. The primary chart tracks broader
symbol selection more closely than secondary charts.

## Mids And Price Updates

Real-time mids are stored in:

- `all_mids`
- `all_mids_updated_at_ms`
- `live_watchlist_flashes`

The user-data websocket supplies all-mids updates. `market_state/mids.rs`
applies them and then updates dependent systems:

- chart reference prices and price flashes
- order form defaults
- live watchlist row caches
- order-book precision refresh planning
- liquidation distribution refresh when relevant
- account/position PnL surfaces that depend on latest mids

Mids are not persisted. They are live market state.

Spot mid lookup is exact-pair-only. Indexed keys and API-named pairs may use a
metadata-verified alias for the same spot asset, but a missing spot mid never
falls back to a same-ticker perpetual. This invariant applies to chart values,
order defaults, USD-to-coin conversion, presets, Chase, and TWAP.

## Spot Context And Candle Recovery

Spot chart asset-context fallback validates the complete
`spotMetaAndAssetCtxs` schema and coalesces all eligible spot charts into one
request. Missing/error results use bounded exponential backoff; rate limits set
a shared cooldown so multiple charts cannot create a per-chart retry storm. A
live websocket context always wins a race with REST fallback data.

Watchlist/context requests are request-scoped: malformed top-level spot data is
rejected, missing unrelated universe rows do not poison requested results, and
missing requested symbols are reported without presenting a partial response
as complete. Healthy requested market families are returned alongside explicit
partial errors when another family fails.

Sparse spot candle history is loaded but visibly marked stale when its tail is
too old. A live jump beyond the normal contiguous window triggers a bounded
reconciliation reload; further sparse updates during the cooldown append
normally instead of causing continuous reload churn. The same rules apply to
primary and comparison series.

## Order Books

Runtime order books are keyed by `OrderBookId` and stored as
`OrderBookInstance`.

Order books support:

- active-symbol or fixed-symbol mode
- REST snapshots
- L2 websocket updates
- canonical sigfig/tick handling
- configurable tick grouping
- center-on-mid behavior
- reverse-side layout
- regular depth rows or DOM ladder display
- optional spread chart
- user open-order overlays

Key modules:

- `market_state/types.rs`
- `market_state/dom_ladder.rs`
- `market_update/order_book.rs`
- `market_update/order_book/book_data.rs`
- `market_update/order_book/ws_updates.rs`
- `market_views/order_book/`
- `spread_chart/`

Data flow:

```text
order book pane opens
  -> fetch plan chooses symbol and precision
  -> api::fetch_order_book
  -> Message::BookLoaded
  -> OrderBookInstance stores snapshot/revision
  -> subscription receives WsBookUpdate
  -> update validates symbol and sigfigs
  -> cached projections are invalidated by revision
  -> view renders depth/DOM/spread
```

The update path rejects websocket data that does not match the instance's
symbol mode or canonical precision. Tick-size changes reuse cached book data
when possible and refetch when precision changes require it.

REST snapshot ownership is terminal-scoped rather than pane-instance-scoped,
so applying a runtime layout cannot reset a recreated pane to an ID still held
by its previous in-flight task. The allocator skips active IDs across numeric
wrap; the result handler still requires the exact request ID, pane, symbol,
selected tick, and server sigfig tuple before replacing a book or clearing its
loading owner.

## Symbol Search And Watchlist

`PaneKind::Watchlist` is the symbol-search pane. It shows tradable markets,
filters by market type or HIP-3 dex, displays favourites, and can select the
active symbol.

Important modules:

- `market_views/watchlist.rs`
- `market_views/watchlist/controls.rs`
- `market_views/watchlist/rows.rs`
- `market_update/symbols.rs`
- `market_update/symbols/contexts.rs`
- `market_update/symbols/outcome_volumes.rs`

Watchlist context data uses `api::fetch_watchlist_contexts` for slower metadata
such as price change, volume, open interest, mark/oracle price, or funding
fields. Outcome 24h volumes are fetched separately through
`api::fetch_outcome_volumes_24h`.

## Live Watchlists

`LiveWatchlistInstance` is a multi-instance widget keyed by `LiveWatchlistId`.
It lets users maintain custom symbol lists with configurable columns and sort
order.

State includes:

- symbol list
- search/autocomplete text
- column visibility
- sort column and direction
- settings menu state
- row caches and flash state

Data flow:

```text
timer or symbol change
  -> LiveWatchlistRefreshTick
  -> fetch contexts/history for visible symbols
  -> LiveWatchlistContextsLoaded / LiveWatchlistHistoryLoaded
  -> row projections update
  -> view renders rows and flashes
```

Live watchlists are persisted as widget configs in saved layouts and current
config snapshots.

## Ticker Tape

The ticker tape is an optional full-width strip below the top bar. It displays
favourite symbols and scrolls continuously.

State includes:

- `ticker_tape_enabled`
- `ticker_tape_scroll_px`
- `ticker_tape_ctxs`
- refresh timestamps and loading flags

Messages include:

- `ToggleTickerTape`
- `TickerTapeTick`
- `TickerTapeRefreshTick`
- `TickerTapeContextsLoaded`

Ticker tape context fetches are separate from live watchlists so disabling or
changing one surface does not disturb the other.

## Positioning Info

Positioning info panes are keyed by `PositioningInfoId` and backed by
HyperDash data.

They show:

- position distribution for a market
- side filters
- entry-price range filters
- search/symbol picker
- sort fields
- change timeframes
- flow and summary metrics

Key modules:

- `positioning_state/`
- `market_update/positioning_info/`
- `market_views/positioning_info/`
- `hyperdash_api/positioning/`

Positioning requests use request keys for dedupe and stale-response protection.
Asset-context streams update live mark/mid metadata for matching panes.

## Session Data

Session data panes are keyed by `SessionDataId`. They fetch daily and chunked
intraday candles for a selected symbol and lookback window to display weekday
and market-session behavior. Every refresh receives a terminal-lifetime request
ID in addition to its pane/symbol/lookback/timestamp context. The allocator
outlives runtime layout reconstruction and skips live IDs across wrap, so an old
pane task cannot consume the replacement pane's loading owner or results.

Key modules:

- `session_data_state.rs`
- `market_update/session_data.rs`
- `market_views/session_data.rs`

Session data instances are persisted in layout/widget configs. Request IDs and
pending work remain runtime-only.

## Outcomes

Outcome markets use `#`-style symbols and have special display and trading
rules. Outcome modules handle:

- grouped outcome market display
- probability bars
- buy/sell side buttons
- 24h volume fetches from candles
- sell-prefill for held outcome balances

Key modules:

- `api/exchange_symbols/outcomes/`
- `market_views/outcomes/`
- `market_update/symbols/outcome_volumes.rs`
- `order_execution/symbols/outcome/`

Outcome markets force coin-size input for some order flows and should avoid
incorrect USD-notional assumptions.

## HYPE ETF And Unstaking Widgets

HYPE-specific market widgets live in:

- `hype_etf_state.rs`
- `market_update/hype_etfs.rs`
- `market_views/hype_etfs.rs`
- `api/hype_etfs/`
- `hype_unstaking_state.rs`
- `market_update/hype_unstaking_queue.rs`
- `market_views/hype_unstaking_queue.rs`
- `api/hype_unstaking_queue.rs`

HYPE ETFs combine THYP, BHYP, and Farside BHYP flow data where available.
Unstaking queue state supports window filters, amount filters, sorting, and
mine-only filtering.

These panes are informational. They are refreshed by timers and manual refresh
messages, not by trading-order state.

## Hidden Symbols And Market Universe

Muted tickers and market-universe settings are enforced through `risk_state/`.
Market surfaces should apply these filters before:

- showing rows
- creating subscriptions
- starting order-book or chart fetches
- using mids for trading actions

Hidden symbols should not silently route trading automation.

## Tests To Check

Use focused tests in these areas:

- `src/market_update/tests.rs`
- `src/market_state/mids/**/tests`
- `src/market_state/symbol_search/**/tests`
- `src/market_state/live_watchlist/**/tests`
- `src/market_update/live_watchlist/**/tests`
- `src/market_update/order_book/book_data/tests/**`
- `src/market_update/order_book/ws_updates/tests/**`
- `src/market_views/order_book/**/tests`
- `src/market_views/positioning_info/tests/**`
- `src/hype_etf_state/tests/**`
- `src/market_update/hype_etfs/tests.rs`

Run broader checks when market data changes affect order sizing, active symbol
resolution, chart subscriptions, or hidden-symbol filtering.
