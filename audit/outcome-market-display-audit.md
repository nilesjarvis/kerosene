# Outcome (HIP-4) Market Display Audit

Last audited: 2026-06-11.

This audit covers how outcome/HIP-4 markets resolve and render human-readable
names (and whole-contract sizes) across every widget, why tickers sometimes
showed raw keys (`#950`, `+950`) or the synthetic internal ticker
(`OUT95-YES`), and the remediation applied. The audit swept nine widget areas
with adversarial verification of every finding: 67 raw findings, 48 confirmed,
19 refuted as unreachable for outcome coins, plus a completeness pass that
recovered 3 dropped cross-scope defects.

## System Shape

Coin key encodings (`src/api/exchange_symbols/model.rs:44`):

- Main perp dex: `BTC`, `HYPE`
- HIP-3 dexes: `xyz:NVDA`, `km:US500`
- Spot pairs: `@107` (HYPE/USDC)
- Outcome trade coins: `#950` (encoding = outcome_id * 10 + side_index)
- Outcome balance coins: `+950` (spot-style holdings of the same contract)

Outcome `ExchangeSymbol`s are built in `src/api/exchange_symbols/outcomes.rs`:
`ticker` is the synthetic `OUT{id}-{SIDE}` (never meant to be user-visible),
`display_name` is precomputed from `OutcomeSymbolInfo::display_label()`
(`"YES: Will BTC close green?"` style, built in
`src/api/exchange_symbols/model/outcome_labels.rs`).

Canonical display helpers on `TradingTerminal`
(`src/order_execution/symbols/display.rs`, `outcome.rs`):

- `display_name_for_symbol(coin)` — the general resolver. Order: loaded
  `exchange_symbols` → persisted outcome-label cache (new, handles `#` and `+`
  keys) → `coin.split(':').nth(1)` fallback.
- `exchange_symbol_display_name(sym)` — when the symbol is already resolved.
- `display_coin_for_journal(coin)` — fills/journal: maps `@`/`#` keys, leaves
  perp tickers untouched.
- `display_coin_for_spot_balance(coin)` — balance rows: spot pair names and
  `"{label} (+NNN)"` for outcome balances.
- `display_size_for_symbol(coin, size)` — whole contracts (`{:.0}`) for
  outcome coins.
- `is_outcome_coin(coin)` — symbol lookup or `#` prefix, works pre-load.

## Root Causes

The reported bug — "tickers sometimes do not load the proper name in other
widgets" — had five compounding causes:

1. **Boot-only symbol fetch with silently swallowed failures.**
   `fetch_exchange_symbols` ran exactly once (`src/app_boot/boot.rs:131`) and
   discarded `spotMeta`/`outcomeMeta` errors with `if let Ok(...)`
   (`src/api/exchange_symbols.rs:34-40`). One transient failure at boot meant
   zero outcome symbols for the whole session — every outcome widget degraded
   to raw keys with no error and no retry. A failed full fetch likewise never
   retried.

2. **No refresh, while HIP-4 churns intraday.** Hourly/daily questions are
   listed continuously; any outcome market created after boot was permanently
   absent from `exchange_symbols`, so its coin rendered as `#NNN` everywhere
   until restart (no `exchange_symbols` refresh existed in
   `src/subscription_state/timers/`).

3. **Expired markets lose labels forever.** Settled questions drop out of
   `outcomeMeta`, but their `#NNN` coins remain in fills, journal entries, and
   balances. For history widgets the missing-symbol case was the *common*
   case, and the fallback is the raw key.

4. **Snapshot display fields that never heal.** Labels were resolved once and
   cached into state: `ChartInstance.symbol_display` (raw on runtime layout
   load, `src/layout_persistence/instances.rs:43`), spaghetti `Series.display`
   (never re-resolved by `apply_symbols_loaded` at all), and stale copies of
   `inst.symbol_display` propagated into the global `active_symbol_display`
   on pane click / quick-order open.

5. **Per-widget bypasses.** ~35 sites rendered `coin`, `sym.ticker`
   (`OUT95-YES`), or hand-rolled `split(':')` fallbacks instead of the
   canonical helpers, and several formatted whole-contract outcome sizes with
   fractional formatters.

## Remediation Implemented

### Symbol load lifecycle (systemic)

- `fetch_exchange_symbols` now returns `ExchangeSymbolsPayload { symbols,
  spot_meta_failed, outcome_meta_failed }` — partial failures are explicit
  instead of silent (`src/api/exchange_symbols.rs`).
- `apply_symbols_loaded` (`src/market_update/symbols.rs`) merges on partial
  failure (previously loaded spot/outcome symbols are kept so labels never
  regress mid-session), early-exits when the symbol set is unchanged
  (`ExchangeSymbol` now derives `PartialEq`), surfaces a status when outcome
  metadata is missing, and stays quiet on background refresh failures.
- New `Message::ExchangeSymbolsRefreshTick` fires every 120s
  (`src/subscription_state/timers/app.rs`), guarded by an in-flight flag.
  This picks up newly listed HIP-4 questions within two minutes and retries
  failed/partial boot loads automatically.

### Persistent outcome label cache (systemic)

- `outcome_display_labels: HashMap<String, String>` on `TradingTerminal`,
  persisted in config (`src/config/schema.rs`, serialization test in
  `src/config/tests/serialization/preferences/search.rs`). Every symbols load
  records `#NNN → label` (`record_outcome_display_labels`).
- `display_name_for_symbol` and `display_coin_for_spot_balance` fall back to
  the cache for `#`/`+` keys, so fills, journal, balances, and history on
  expired or not-yet-loaded markets keep their names across restarts.

### Snapshot healing (systemic)

- Runtime layout restore resolves chart and spaghetti labels through
  `display_name_for_symbol` (`src/layout_persistence/instances.rs`).
- `apply_symbols_loaded` now also re-resolves spaghetti `Series.display`
  (and invalidates the canvas cache) alongside charts, the active symbol,
  watchlist caches, and feed mention resolvers.
- Pane click and quick-order open re-resolve the display instead of copying
  the chart's cached label into `active_symbol_display`
  (`src/pane_interaction_update.rs`, `src/order_update/quick_order/open.rs`).
- Outcome positions are synthesized from `+NNN` balances unconditionally
  (`src/account_positions.rs`) — holdings no longer vanish from the Positions
  tab when the symbol lookup misses (expired market, failed meta fetch, or
  fallback-settlement contract).

### Per-widget fixes (all confirmed findings)

Account & wallet:

- Trade History symbol column → `display_coin_for_journal`
  (`src/account_views/history_tables/trades/row.rs`).
- Fill toasts resolve label + whole-contract size at the push site
  (`src/account_update/stream.rs`, `stream/fills.rs`).
- Optimistic "opening position" row label/size resolved at the call site
  (`src/account_views/positions.rs`).
- Positions table renders the market label instead of the synthetic
  `OUT95-YES` ticker (`src/account_views/positions/table/position_row.rs`).
- Wallet tracker open orders: resolved labels + whole-contract sizes
  (`src/wallet_views/orders.rs`, `orders/row.rs`).
- Wallet tracker spot amounts: `{:.0}` for `+NNN` balances
  (`src/wallet_views/spot/row.rs`).

Feeds & search:

- Tracked-trade coin cells and alert toasts → `display_coin_for_journal`
  (`src/feed_views/tracked_trades/rows/cells.rs`,
  `src/feed_state/tracked_trades/alerts.rs`).
- Telegram and X ticker-impact chips show the market label instead of
  `OUT{id}-SIDE` (`src/feed_views/telegram.rs`, `src/feed_views/x.rs`).
- Generic `outcome`/`prediction` keyword seeds no longer alias every outcome
  market — posts containing those words no longer flood mention chips
  (`src/symbol_mentions.rs`).
- Liquidation alert toast and feed rows resolve `@`/`#` coins
  (`src/feed_update/liquidations.rs`, `src/feed_views/liquidations/rows/`).

Orders:

- Move-order, no-mid, price-band, one-shot placement status, NUKE
  status-unknown, tradability/risk-hidden, and Alfred submit statuses all lead
  with the resolved label (`src/order_execution/quick_order/move_order.rs`,
  `core.rs`, `symbols/market/mids.rs`, `active_symbol.rs`,
  `src/order_update/results.rs`, `src/alfred_update/submit.rs`).
- Chase banner label + whole-contract sizes
  (`src/order_views/actions/chase.rs`); TWAP rows render the persisted
  `display_coin` (`src/order_views/advanced/rows.rs`).
- USD-mode size hint uses the resolved display and whole contracts
  (`src/order_views/inputs/size/calculations.rs`).
- Chase history archives the *resolved* `display_coin` instead of freezing the
  raw key on disk, mirroring the TWAP path
  (`src/advanced_order_history/snapshots.rs`).

Charts & windows:

- Empty-candle error overlay uses `symbol_display`
  (`src/chart_update/candles/loaded.rs`).
- On-chart order labels format outcome sizes as whole contracts
  (`src/chart/order_labels.rs`).
- Crosshair volume readout matches the header's whole-contract formatting via
  a `whole_unit_volume` flag on `CandlestickChart` (`src/chart/crosshair.rs`).
- Chart editor rejection statuses resolve the name
  (`src/chart_update/editor/symbol.rs`).
- Spaghetti right-edge labels ellipsize to the axis width so long outcome
  labels stay distinguishable (`src/spaghetti/normalized/series/labels.rs`).
- PnL card OS window title resolves the coin (`src/main_view/windows.rs`).

Market widgets:

- Ticker tape: compact `side_condition_short_label()` for outcome favourites,
  canonical fallback for unloaded keys (`src/market_views/ticker_tape.rs`).
- Watchlist exchange column uses `symbol_search_exchange_label`
  (`src/market_views/watchlist/rows/item.rs`).
- Live watchlist rows resolve through the canonical helper
  (`src/market_state/live_watchlist/rows.rs`); its autocomplete now matches
  display names and keywords, so question text is searchable
  (`src/market_views/live_watchlist/controls/autocomplete.rs`).
- Order book Fixed-mode title → `display_name_for_symbol`
  (`src/market_views/order_book/controls/chrome.rs`).
- Depth list and DOM ladder format outcome level sizes/totals as whole
  contracts via `BookRowData.whole_contracts` / `format_book_size`
  (`src/helpers/order_book/row.rs`, `row/cells.rs`).

PnL card & journal:

- PnL card metrics resolve positions via `account_positions_with_outcomes()`,
  fixing "Position is no longer open" on outcome holdings, and the card ticker
  is the resolved label (`src/pnl_card/metrics.rs`, `update/window.rs`).
- Journal trade-card max-position uses whole contracts for outcome coins
  (`src/journal_views/trade_card.rs`).

## Verified Clean (no change needed)

- Outcomes pane, screener rows, symbol search, order book settings picker,
  session data header, journal asset rows — already canonical.
- Open-orders rows (`src/account_views/orders/row.rs`) and balances rows —
  already canonical for outcome coins.
- Layout/config persistence stores raw *keys* only (display resolved at render
  time); the single persisted display label was the chase history entry, fixed
  above.
- WS subscription plumbing passes `#NNN` keys verbatim with no mangling;
  hyperdash integrations are perp-gated and cannot receive outcome coins.
- 19 findings refuted as unreachable for outcome coins (e.g. NUKE skip lists,
  chase placement statuses, funding rows — outcome holdings never appear in
  clearinghouse `asset_positions`; chase/TWAP placement blocks outcome
  markets). Several still show raw *HIP-3/spot* keys on defensive paths; not
  outcome defects, left as noted.

## Known Remaining Items

- `alfred_state/position_close.rs:72` renders the raw position coin, but the
  resolver reads only `clearinghouse.asset_positions` (line 96), which cannot
  contain outcome coins today. If Alfred close ever consumes
  `account_positions_with_outcomes()`, resolve the title through
  `display_name_for_symbol`.
- Portfolio *summary* PnL card aggregates clearinghouse positions only;
  widening it to synthesized outcome positions is a deliberate scope cut.
- Depth-chart canvas size labels (`src/depth_chart/rendering.rs`) still use
  the generic `format_size`; the order book list/DOM were fixed.
- The journal filter has All/Perp/Spot but no Outcome category — a
  categorization gap, not a label bug.
- `JournalFilter`-style chase fill summaries (`account_update/stream/fills.rs`
  chase paths) embed raw coins but are unreachable for outcome coins (chase
  blocks outcome markets).

## Validation

- `cargo fmt` clean; `cargo check --all-targets` clean.
- `cargo test`: 1848 passed, 0 failed (≈45 new regression tests across the
  fixed areas, including: payload merge on failed `outcomeMeta`, label-cache
  fallback for `#`/`+` keys after expiry, spaghetti healing on symbols load,
  quiet background refresh failure, config round-trip + legacy default for
  `outcome_display_labels`, whole-contract formatting in book rows / chart
  order labels / wallet rows / toasts, ticker tape & impact chips label
  resolution, PnL card metrics from a `+NNN` balance).
- `timeout 20s xvfb-run -a cargo run`: window starts, no panic.
- `cargo clippy --all-targets --all-features -- -D warnings`: one pre-existing
  failure in `src/chart/interaction/press.rs:198` (`if_same_then_else`) from
  unrelated in-progress session-panel work in the working tree; everything
  else clean.
