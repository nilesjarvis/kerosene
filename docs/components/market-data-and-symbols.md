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

Official REST reads use `api::proxy::HyperliquidRequestExt::send_info` so the
optional [proxy pool](../operations/hyperliquid-proxies.md) can distribute each
request. `api/read_control.rs` provides weighted admission, reserved account-read
capacity, bounded concurrency/waits, and provider cooldown after direct HTTP 429.
`api/shared_reads.rs` shares complete public metadata/context snapshots and
in-flight work across feature callers. Candle endpoint reads also coalesce by
provider, credential scope, symbol, interval and range. No account responses are
stored in this public cache.

Candle adapters measure per-topic silence independently from socket health. They
emit `ChartWsCandleUnavailable` / `SpaghettiWsCandleUnavailable`, request a
reference-preserving topic resubscription, and reconcile affected history without
clearing the display. Sparse markets have a longer silence threshold. History
success and live-stream freshness remain separate. Comparison history starts via
`SpaghettiFetchRequested`, with exact pending-request matching, retry backoff and
live-update replay. See the [recovery audit](../market-data-pipeline-audit.md) for
budgets, thresholds and remaining architectural tradeoffs.

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
- `outcome_venue_filter` (runtime-only)
- `outcome_collapsed_market_groups`

Symbol search is implemented in `market_state/symbol_search/` and
`market_update/symbols.rs`. It normalizes labels, applies market-universe
filters, hides muted tickers, resolves aliases, and feeds chart/order-book/order
entry selection.

The symbol universe refreshes every 120 seconds to discover new and expired
markets. Metadata and label changes preserve open chart history and order-book
state. A full widget reload is only scheduled if the selected market universe
changes (for example, an unavailable HIP-3 dex falls back to all markets).
Canonical symbol migrations still refetch the affected widgets individually.

## New Listings Feed

The **New Listings** singleton pane is available under Add Widget > Feeds and
through Alfred. It displays native perps, HIP-3 perps, and spot pairs with
All / Perps / Spot filters. Selecting an available market uses the normal symbol
selection flow. Hidden symbols are omitted from the feed.

`api/exchange_symbols/listings.rs` reads public `allPerpMetas` and `spotMeta`
snapshots every 30 seconds while the pane is open, including in Canvas windows.
The one-second `ListingsTick` updates display time and checks the fetch cooldown;
only two metadata requests are made per scan. Requests are coalesced, manual
refresh has a five-second minimum interval, and request IDs reject stale results.
This is polling-based discovery, not an announcement WebSocket or a trading-open
guarantee. No API key or wallet is required.

`market_state/listings.rs` establishes an independent, silent baseline for each
market family on its first successful live response. Failures retain known IDs;
recovered data, renamed spot aliases, and previously seen markets do not generate
duplicate entries. Perps retain DEX-qualified identity and inactive metadata;
spot identity uses the pair's asset index, not the token ticker. Newly discovered
inactive perps are announced when they first become active. Inactive perps in the
initial baseline are treated as already known because metadata cannot establish
whether they previously traded.

`config/listings.rs` defines the history payload stored by
`config_persistence/listings.rs` in a versioned envelope in
`<config-dir>/cache/v1/listings.json` using the existing atomic cache writer,
off the UI thread. The latest 200 entries are retained, along with known market
IDs for deduplication. Storage failures remain visible and are retried; tests
and `--test` never access real user history. Clearing the app cache resets the
feed baseline. Existing layout JSON remains compatible; the new pane and its
padding target serialize as `NewListings`.

Timestamps are **first detected** locally. Markets discovered after the app was
closed are timestamped when next observed; no historical listing time is inferred.
The first installation starts with an empty feed. The view is in
`market_views/listings.rs`; `Listings*` messages route through
`market_update/listings.rs` and trigger a symbol-universe refresh on additions.

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
fields. The cached per-market context stores perpetual open interest as USD
notional (`openInterest * markPx`) for cross-market ranking and retains the
positive `markPx` used with `prevDayPx` for dynamic 24-hour gainer ranking.
Legacy cache rows default these newer fields to unavailable. Outcome 24h
volumes are fetched separately through `api::fetch_outcome_volumes_24h`. An
Outcomes widget in the main window or an open Canvas requests eligible markets;
otherwise only primary symbols of open outcome charts (including detached
charts) request volumes for their headers. Closed saved canvases and unused
chart instances create no volume demand. Pane/layout/window changes reconcile
demand immediately, and the existing status tick picks up chart symbol or
visibility changes within one second. Changed demand aborts the old batch and
rejects late results; repeated metadata refreshes reuse an in-flight batch for
the same symbols. Runtime-only task and requested-symbol state preserve known
volumes between batches. At most two candle fetches run concurrently, retaining
the existing candle cache and shared API read budget. Outcome metadata, trading,
chart subscriptions, and account reads remain independent of volume demand.

## Live Watchlists

`LiveWatchlistInstance` is a multi-instance widget keyed by `LiveWatchlistId`.
Each instance selects a named `WatchlistPresetConfig`, while column visibility
and sort order remain widget-specific. Presets are global rather than
layout-local, so several widgets and layouts can reuse the same asset list.
Creating, renaming, selecting, or deleting a preset is available from the live
watchlist controls. Adding or removing a symbol edits the selected preset and
synchronizes every live watchlist and comparison chart linked to it.

State includes:

- selected preset ID and its synchronized symbol list
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

Named presets are persisted globally in `KeroseneConfig::watchlist_presets`.
Live-watchlist layout configs store the preset ID plus an inline symbol snapshot
for compatibility with older and imported layouts. Legacy inline-only lists are
migrated to named presets when configuration is loaded.

## Ticker Tape

The ticker tape is an optional full-width strip below the top bar. It displays
favourite symbols and scrolls continuously. A fixed exchange-stats section on
the right remains visible while the favourites move. Its 24-hour volume sums
the live `dayNtlVlm` values from Hyperliquid's main-perp, HIP-3, and spot asset
contexts. Notional open interest sums `openInterest * markPx` across main-perp
and HIP-3 contexts; spot is excluded because it has no open interest.

State includes:

- `ticker_tape_enabled`
- `ticker_tape_scroll_px`
- `ticker_tape_ctxs`
- `ticker_tape_exchange_stats`
- refresh timestamps and loading flags

Messages include:

- `ToggleTickerTape`
- `TickerTapeTick`
- `TickerTapeRefreshTick`
- `TickerTapeContextsLoaded`
- `TickerTapeExchangeStatsLoaded`

Ticker tape context fetches are separate from live watchlists so disabling or
changing one surface does not disturb the other. Exchange stats refresh every
minute while the ticker tape is visible; incomplete API snapshots leave the
last complete value in place.

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

Session data panes are keyed by `SessionDataId`. They fetch daily candles for a
selected symbol and lookback window to display session-level behavior.

Key modules:

- `session_data_state.rs`
- `market_update/session_data.rs`
- `market_views/session_data.rs`

Session data instances are persisted in layout/widget configs.

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

### Skew / HIP-4 venues

Skew markets are native Hyperliquid outcome markets, discovered through the
same mainnet `outcomeMeta` request as other HIP-4 contracts. No Skew login,
API key, webview, or separate signing path is needed. Open the **Outcomes**
widget and choose **Skew** in the venue picker, or search `skew` / `skew.trade`
in symbol search. Selecting Yes or No selects that exact contract for the
existing charts, order books, and main order ticket.

The optional `venue` field is retained in `OutcomeSymbolInfo`, symbol labels,
search keywords, and the API metadata cache. Old cached symbols without the
field still deserialize. The venue picker is populated from selectable,
non-hidden metadata, combines with text search, and retains an unavailable
selection across market rolls or metadata failures. Its
`OutcomeVenueFilterChanged` message routes through the market update module;
the filter is runtime-only, like outcome text search.

Skew's current `template:binaryPrice` contracts normalize `perp`, `threshold`,
and `time` to the existing underlying, target-price, and expiry fields.
`template:Yes` / `template:No` become readable side labels. The original
description remains intact; other template types are not assumed to have
binary-price semantics. The Outcomes pane, ticket, and order book show the
published `priceDescription` and `seconds` settlement window. This is not
the underlying's live mark price: Skew's trade feed uses VWAP and its Pyth
feed uses different weighting, despite the shared template's TWAP wording.

Identity remains `#(10 * outcome + side)` for market data,
`+(10 * outcome + side)` for balances, and `100_000_000 + 10 * outcome + side`
for orders/cancels. Venue names never prefix the coin or alter asset IDs.
Skew's current contracts are USDC-quoted and use whole-contract sizes.
Trading continues through the existing HIP-4 ticket and account checks;
Chase/TWAP remain unsupported for outcomes. No orders are sent by the
integration or its tests.

References verified 2026-09-17:

- [Skew market documentation](https://docs.skew.trade/markets/overview)
- [Skew BTC Hourly terms and identity](https://docs.skew.trade/markets/btc-hourly)
- [Hyperliquid outcome asset IDs](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/asset-ids)

Regression fixtures in `api/exchange_symbols/outcomes/tests/fixtures/skew.json`
contain public `outcomeMeta` rows for BTC, Nasdaq-100, and S&P 500. Tests cover
labels, expiry, venue cache compatibility, both side assets, buy/sell/cancel
preparation, and price/size/mid validation without signing or sending orders.

### Contract verification and lifecycle

Outcome discovery fetches `outcomeMeta` and `outcomeTemplates` together from
Hyperliquid. `outcomes/templates.rs` validates template parameters and renders
titles, sides, and full resolution rules from that registry. Supported lifecycle
families include binary, touch, and scalar prices, sports contests and tournaments,
IPO confirmation, AI model comparisons, and policy rate decisions. Named
outcomes must reference a valid question with the expected parent template;
they inherit its deadline and rules. Legacy price-binary and price-bucket
metadata remains supported with validated bounds and expiry.

`OutcomeSymbolInfo.contract` stores resolved `OutcomeContract` terms. Missing
fields in older caches default safely; the `verified` flag is runtime-only and
never survives serialization. A cached or failed metadata refresh preserves
labels, rules, and cancellation access but blocks placement and modification
until a live refresh verifies the contract. Unknown templates, invalid
parameters, inconsistent side labels, and unsupported/missing quote tokens are
also blocked. Unknown quote metadata never falls back to USDC for the ticket's
available balance or percentage sizing. Duplicate/overflowing IDs, nonbinary side counts, and ambiguous
question membership reject the outcome metadata family.

Shared order preparation checks the actual clock against the contract expiry
or resolution deadline. Views use the existing frame clock. Sports start times
and scheduled policy decisions are not trading cutoffs: their explicit
resolution/decision deadlines are used instead. Settled named outcomes and
fallback settlement tokens cannot be traded. Early settlement still depends on
fresh exchange metadata; the client does not infer an oracle decision from a
live price. Removed markets retain historical display labels, and cancellation
can recover the asset from a strictly validated canonical `#` side key.

The Outcomes pane and ticket expose expandable **Contract rules**, routed by
`OutcomeRulesToggled` through the market update module. Expansion is runtime
state keyed by outcome ID. Sports and other custom side names are preserved;
the second side is only described as a negation when its name is No. Scalar
contracts show quote-token prices and omit probability bars. Order pricing
uses exact outcome coin mids, never display-ticker or underlying-perp aliases.

The ticket shows published protocol `feeScale` and `deployerFeeScale`, or
explicitly unavailable values. Cached scales are not presented as live terms.
These scales are not a dollar fee estimate: settlement fees depend on closing
and settlement behavior, and HIP-4 has no maker rebates. Split, merge, and
negate operations remain outside the supported ticket workflows.

The public `hip4.json` and `templates.json` fixtures were captured on 2026-09-17
from the mainnet info endpoints, excluding deployer addresses. Regression tests
cover current template rendering, parent validation, lifecycle boundaries,
cache/failure recovery, fee metadata, order gates, and cancellation without
signing or submitting trades. Protocol references:

- [Hyperliquid outcomes](https://hyperliquid.gitbook.io/hyperliquid-docs/hyperliquid-improvement-proposals-hips/hip-4-outcome-markets)
- [Hyperliquid fees](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/fees)

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
