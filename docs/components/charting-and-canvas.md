# Charting And Canvas

Kerosene's charting system combines per-chart runtime state, historical REST
backfills, websocket candle updates, iced canvas rendering, viewport
interaction, trading overlays, optional liquidation/heatmap/funding/earnings
data, screenshots, and comparison charts.

## Component Map

| Component | Key files | Responsibility |
| --- | --- | --- |
| Chart instance state | `src/chart_state/` | Per-chart symbol/timeframe/model, fetch state, editor state, quick order, annotations, overlays. |
| Chart update flow | `src/chart_update/` | Candle loads, timeframe changes, websocket updates, editor, detached charts, earnings, macro indicators, HUD. |
| Chart views | `src/chart_views/` | Header, toolbar, editor, indicator menu, canvas surface composition. |
| Canvas engine | `src/chart/` | `CandlestickChart`, canvas program, data model, geometry, viewport, interaction, overlays, drawing layers. |
| Screenshots | `src/chart_screenshot/` | Screenshot UI, canvas snapshots, offscreen rendering, PNG export. |
| Spaghetti charts | `src/spaghetti_state.rs`, `src/spaghetti/`, `src/spaghetti_update/`, `src/spaghetti_views/` | Normalized comparison and pair-ratio charts. |
| Spread chart | `src/spread_chart/` | Compact order-book spread history canvas. |

## ChartInstance

`ChartInstance` in `chart_state/model.rs` is the runtime owner for one chart:

- ID, symbol, display name, timeframe
- `CandlestickChart`
- latest asset context
- symbol editor state
- collapsed header state
- right-click quick-order form state
- persisted annotations
- liquidation level overlay state
- historical heatmap state
- candle fetch request and non-blocking fetch error
- candle provider-verification and websocket-observation timestamps
- bounded websocket updates captured during an in-flight history request
- explicit candle interval-discontinuity state
- SEC earnings marker state
- funding fetch state
- macro indicator config
- header metric display modes

`TradingTerminal` stores chart instances in `charts: HashMap<ChartId,
ChartInstance>` and allocates IDs through `alloc_chart_id`.

Startup and saved-layout restoration share `ChartInstance::from_config` in
`chart_state/model/restoration.rs` for persisted settings and annotations.
Comparison charts use `SpaghettiChartInstance::from_config`. The callers retain
symbol resolution, visibility filtering, and data request scheduling, which
differ between startup and runtime layout changes.

## Chart Surfaces

One chart can be rendered in different surfaces:

- inline pane surface
- detached chart window surface

`ChartSurfaceId` disambiguates surface-specific viewport and interaction state.
This lets a detached window share the chart's data while tracking its own
visible range and canvas interaction details.

## Candle Backfill Flow

Historical candles are requested through `chart_update/candles/`.

```text
symbol/timeframe/reload change
  -> queue_candle_fetch_for
  -> CandleFetchRequest stored on ChartInstance
  -> full visible lookback fetched with NetworkOnly policy
  -> Message::ChartCandlesLoaded
  -> stale request guard checks exact request
  -> refresh result replaces unverified/cache-backed visible history
  -> websocket updates received during the request replayed last
  -> shared candle cache updated
  -> chart cache invalidated
  -> overlays and funding/heatmap/liquidations may refresh
```

`CandleFetchRequest` includes chart ID, symbol, timeframe, source, time range,
and attempt number. Result handling compares the incoming request with the
currently stored request before applying it.

Refresh requests never use a cache hit as proof of freshness. They revalidate
the full visible lookback against the selected provider. `BackfillOlder`
pagination may use a complete, finalized cache page and merges that page into
the existing history.

At application boot, an optional disk-cache read and the provider refresh run
as independent tasks:

```text
first frame
  -> websocket subscription active immediately
  -> async disk result -> Message::ChartCachedCandlesLoaded
  -> exact request/source/generation guard
  -> cached history may render as "verifying", never as verified
  -> provider result replaces cached history and certifies freshness
```

A late disk result is ignored after its request is replaced or completed. If a
live bucket arrived first, the cached series is installed underneath it so the
disk task cannot regress the mutable tail.

The backfill source comes from `ReadDataProvider`:

- Hyperliquid for default reads.
- Hydromancer when selected and an API key is available.
- Fallback behavior when Hydromancer is selected without a usable key.

## Shared Candle Cache

`api/candles/normalize.rs` validates and stably sorts candles, then deduplicates
in place so the last valid input for each timestamp wins. Trailing-run helpers
search backward for the final discontinuity, using the same exact or tolerant
spacing rules as cache containment.

`chart_state/candles/cache.rs` stores the bounded in-memory LRU by
`(ChartBackfillSource, symbol, Timeframe)`, so data from different providers
cannot overwrite or satisfy one another.

The persistent cache keeps its public entry points in `api_cache.rs`, with
candle policy in `api_cache/candles.rs`, queued writes in `api_cache/writer.rs`,
and JSON envelopes/path handling/atomic file replacement in
`api_cache/storage.rs`. The writer identifies superseded saves in a single
reverse pass, then executes retained jobs in their original order. Merges and
removals always run.

Candle snapshots use their own versioned namespace, persist only buckets that
were closed when the write was queued, and record coverage through the final
close time. A snapshot without complete coverage cannot satisfy a range request.

Continuous-market cache reads require exact interval spacing and return only
the trailing exact run after a discontinuity. Sparse spot/outcome and
calendar-month data use the existing tolerant containment rule because missing
trade buckets or variable calendar spans can be legitimate. Cached OHLC values
are never rewritten, and the cache-containment logic does not synthesize bridge
candles.

Filesystem cache reads do not run inside interactive update handlers. Cold
startup hydration uses a blocking worker task; symbol/timeframe changes can use
only the in-memory LRU for immediate display while their network refresh runs.
Spaghetti charts fetch from the network before marking a series loaded because
they do not currently expose the regular chart's cache-verification status.
Detached spaghetti charts reuse any already-loaded cloned history and fetch only
series that were still unloaded when detached. Their REST completions and
websocket subscription identities carry a runtime instance epoch so queued work
from a replaced layout cannot mutate a new chart that reuses the same numeric ID.

Cache invalidation matters when:

- symbol changes
- timeframe changes
- backfill source changes
- hidden/muted symbols are applied
- data is reloaded

## Websocket Candle Updates

Chart candle websocket subscriptions are assembled under
`subscription_state/market/chart.rs`. Streams are keyed by chart ID, symbol, and
interval, with deduplication where possible.

Subscriptions start while REST history is still loading, so cold provider
latency does not prevent a current live bucket from arriving. A websocket
update applies to every matching chart instance, triggers price flashes,
invalidates render caches, and can schedule funding refreshes when macro panels
need them.

Primary and secondary series share the tail-update routine in
`chart/data/candles.rs`: a valid candle replaces the same timestamp or appends a
newer bucket, while an older timestamp is rejected. Appending trims the oldest
history to the chart limit; replacement leaves history length unchanged. Only
applied updates clear the render cache, and a missing secondary series rejects
the update. The callers retain chart status and websocket reconciliation policy.

Backward, skipped, or misaligned buckets on continuous markets are not blindly
appended; they trigger a network-only reconciliation. Naturally sparse markets
reconcile once and then use a short backoff to avoid reload churn, while
surfacing their interval gap. Live buckets received during REST are deduplicated
by open time in a bounded buffer and replayed after the response so an older
snapshot cannot overwrite them. Persistent snapshots are queued on bucket
rollover, not on every mutable-tail update. If duplicate panes share a candle
key, the most recently provider-verified pane is the cache representative.

The toolbar and chart surface distinguish:

- loading and verifying history
- cached but unverified history
- live tail while history is still being verified
- refresh errors/stale candles
- interval discontinuities present in a successful provider response
- continuous-market tails that have aged past their close and update grace

## Funding Data

Funding state is split between:

- `chart_state/funding/`
- `chart_update/candles/loaded.rs`
- `chart_update/macro_indicators.rs`
- `chart/candle_layer/funding/`

Funding fetch requests include chart ID, symbol, coin, range, and mode
(`Snapshot` or `Incremental`). Funding panels have their own range and chrome,
and can be resized through chart messages.

## Asset Context And Header Metrics

Chart headers and overlays always use Hyperliquid's native `activeAssetCtx`
stream for mark/oracle/mid prices, open interest, and current funding, including
when Hydromancer is the selected read provider. Hydromancer's documented
[`activeAssetCtx` payload](https://docs.hydromancer.xyz/readme/websocket/prices-data-and-asset-context/activeassetctx)
omits funding and `prevDayPx`; accepting those pushes as complete context kept
funding blank and prevented the missing-context REST fallback from running.
Candles still use the selected read provider. Context subscriptions remain
deduplicated by symbol and scoped to the current provider generation.
`ChartWsAssetCtxUpdate` applies matching contexts to every chart instance unless
the symbol is hidden.

Spot chart REST fallback batches requested symbols through
`api/chart_asset_context/spot.rs`. Each response builds one borrowed lookup for
universe symbols/aliases and keyed contexts, then returns available contexts in
request order with duplicate requests removed. This preserves first-match
universe lookup, last-match duplicate context coins, and positional fallback
only for responses without keyed contexts. Watchlist context parsing keeps its
separate alias and duplicate-selection rules.

The header's `24h Chg` compares the displayed latest candle close with a
24-hour reference: `(last - previous) / previous * 100`. It prefers the
exchange context's valid `prevDayPx`, including context already available in
another chart of the same symbol. When metadata is missing or expires, it can
derive the reference from completed minute-level candles at or before
`chart.clock_now_ms() - 24 hours`. It never uses the first loaded candle,
interpolates an hourly/daily candle, or reads the close of a candle that extends
past the cutoff. A reference must be less than one minute before the cutoff.
The candle-derived value is marked `≈`, with its exact reference timestamp in
the tooltip. Missing coverage, invalid prices, and nonfinite calculations
continue to display `-`.

`chart_state/price_change.rs` owns the calculation and shared history in
`TradingTerminal::chart_price_change_history`. Verified 1-second/1-minute
history already loaded in a chart can supply every chart of that symbol,
regardless of timeframe. Otherwise, the status tick calls
`queue_chart_price_change_history` in `chart_update/price_change.rs` to request
a small 1-minute candle slice around yesterday's cutoff through the existing
chart data provider. The slice includes the following hour of historical
prices, so the rolling reference advances locally; a refresh is queued five
minutes before coverage runs out. One request serves all charts for a symbol.
`ChartPriceChangeHistoryLoaded` applies normalized results only to the matching
pending request and provider/key generation. Failed or incomplete requests
preserve usable history and back off for one to five minutes. History is
removed when the symbol no longer has an open, unhidden chart.

Header metric display modes can show values as raw or USD notional depending on
the market and user preference.

Expanded perpetual chart headers always include the current hourly funding
rate and countdown beside the symbol, including in narrow panes. The header
wraps metrics when needed. Market identity determines whether funding applies;
missing context shows `-` rather than hiding funding or selecting spot metrics.
The current rate comes from asset context independently of the optional funding
history indicator and its Hydromancer integration.

The opt-in live regression test exercises the production subscription through
header layout for a native perp and a HIP-3 perp with Hydromancer selected:
`cargo test --bin kerosene live_native_funding_reaches_chart_header_with_hydromancer_selected -- --ignored --nocapture`.
It uses public Hyperliquid data and needs no credentials.

## Canvas Rendering

The chart canvas is implemented under `src/chart/`.

Important concepts:

- `CandlestickChart` is the chart model.
- `chart/program.rs` implements iced `canvas::Program<Message>`.
- `ChartState` is iced widget-local state for cursor, scroll, zoom, Y-scale,
  drag state, drawing anchors, HUD controls, measurement, and reset epochs.
- Drawing is split into bounded visible ranges and overlay layers.

Canvas rendering computes:

- visible candle range
- price range and viewport transforms
- volume and funding ranges
- grid/axis labels
- candle and liquidity layers
- overlays for orders, positions, trades, annotations, crosshair, badges,
  countdown, and quick-order/HUD states

Expensive geometry is cached and invalidated when data, viewport, theme, scale,
or overlay-affecting state changes.

## Interaction

Chart interaction modules live in `chart/interaction/` and `chart/viewport/`.
They handle:

- scroll and zoom
- crosshair movement
- Y-scale drag
- drawing tools
- order-line hit testing and drag-to-move
- right-click quick-order placement
- HUD order controls
- range measurement
- reset-view behavior

Gaming HUD order modes are key-bound: `L` selects Limit, `M` selects Market,
and `H` selects Chase. Market and Chase use the `Y`/`X` side selector; Limit
infers its side from the clicked price. An armed Chase click routes through the
normal client-side Chase lifecycle for the chart's symbol.

HUD Limit and Market clicks use the entered coin quantity and do not wait for
background account reconciliation, including post-trade refreshes or refresh
rate-limit backoff. Market clicks still serialize pending trading requests and
unresolved order-status checks; Limit clicks retain their bounded concurrent
placement path. Signing-account identity, fresh prices, shared order preflight,
and exchange-enforced margin/balance and reduce-only checks still apply. Account
refreshes continue for display and account-dependent actions such as close/NUKE
and percentage sizing.

Interaction messages should carry chart ID and surface ID so detached windows
and inline panes do not fight over state.

### Freehand pen

The **Draw** toolbar includes a pen icon. Hold the left mouse button over the
price plot, draw, and release to save a stroke. The pen stays active for further
strokes; Escape or right-click cancels an unfinished stroke and exits drawing
mode. Switching tools or losing window focus abandons an unfinished stroke.
Select moves an entire stroke and exposes the usual color, width, line style,
lock, and visibility controls; Eraser or Select + Delete removes it.

Pen samples use timestamp/price coordinates, preserving their traversal order
when the mouse doubles back. The existing annotation persistence stores them
as `type: "pen"` with `anchors`. Older annotation formats are unchanged. Mouse
jitter below two chart pixels is filtered and long strokes are decimated to a
maximum of 4,096 samples. Rendering clips each segment to the price plot and
uses the existing fisheye projection. Drawing captures chart clicks before
order interactions and suppresses wheel zoom while the pen is held.

## Quick Trade Panel

`Quick Trade` is a per-chart option in the indicator menu. When enabled it
adds a compact section beneath the chart canvas, following the visual hierarchy
of the funding/session sections without mixing order controls into canvas draw
code. The strip contains user-defined BUY/SELL market actions, supports both
USD-notional and coin-denominated quantities, and disables action buttons while
another serialized trading request is pending.

Actions are configured in a dedicated window opened from the strip. The editor
validates positive finite quantities, caps each chart at 12 actions, and saves
the definitions with that chart's layout config. A click in the chart strip is
an immediate market-order intent: the request carries chart ID, surface ID,
symbol, action index, and an action snapshot so symbol, surface, or configuration
changes fail closed before signing.

Quick Trade execution routes through `order_update/quick_trade.rs` and the
shared prepared-order boundary. USD sizes use the fresh mid as their notional
reference, submitted prices use the configured market slippage, actions do not
inherit the main ticket's reduce-only toggle, and outcome markets remain
unsupported from this chart control.

## Trading Overlays

Charts show trading/account state through overlays:

- current price badges
- open order lines
- position entry/size markers
- trade markers
- quick-order cards
- HUD order animation
- cancel hover animation
- move-order drag handling

Overlay data is synchronized after account data loads, websocket user-data
updates, order result handling, and candle loads. Order actions still route
through `order_update` and `order_execution`; chart overlays do not place
orders directly.

`chart_state/overlays.rs` coordinates position, trade-marker, and reference-price
updates. Order-line assembly lives in `chart_state/overlays/orders.rs`, and
fill-to-marker mapping lives in `chart_state/overlays/trades.rs`. Synchronization
borrows chart symbols and pending indicators, and iterates Chase overlays without
an intermediate vector. Overlay rows still own the strings they need.
Confirmed orders, Chase replacements, and pending decorations retain their order
and distinct account/numeric matching rules; trade markers retain stable time order.

Drawing lives in `chart/overlays/`. Order rendering prepares visible orders once,
then draws lines and price badges, label connectors, and labels in separate passes.
The last pass consumes the prepared labels without copying their strings. Current
price and liquidation overlays reuse their line style for badge connectors; order
styles select side colors and animation settings independently. Synthetic rendering
tests cover pending/dragged orders, stacked badges, fisheye effects, and privacy.

Order-label drawing and hit testing share the same stacking geometry. A single
packing routine handles the full label area and the bands above/below a position
label, including reserved-region avoidance and crowded-edge adjustments.
Right-axis badges retain their separate variable-height and fixed-position rules;
each band sorts its anchors once before packing.

`chart/interaction/drag.rs` ends each active gesture through a shared drag reset,
then performs gesture-specific payload cleanup and publication. Panning clears the
candle cache on release so the next frame restores full heatmap detail. Annotation
previews copy their original snapshot only after coordinate inputs are available;
selecting a locked annotation leaves its drag snapshots untouched.

Right-click handling shares the quick-order action builder for opening and
replacing a card. The caller retains click priority and falls through when price
inputs are unavailable. HUD size editing shares character insertion after its
digit/decimal checks; filtering, replacement, and length limits remain in that
order. HUD tests live in `chart/interaction/hud/tests.rs` and its `tests/`
subdirectory.

## Liquidations And Heatmap

Chart liquidation data comes from HyperDash update modules:

- `hyperdash_update/liquidations.rs`
- `hyperdash_update/heatmap.rs`
- `chart_state/heatmap/`
- `chart/candle_layer/liquidity/`
- `chart/tooltips/liquidations.rs`
- `chart/tooltips/heatmap.rs`

Liquidation levels and heatmap requests use request keys and pending maps to
deduplicate identical fetches and fan results out to matching charts. Viewport
changes can trigger heatmap refreshes when the visible time/price range changes.

HyperDash API keys are secret-bearing and should only be used in update/task
boundaries.

## Earnings Markers And Macro Indicators

SEC earnings markers are optional chart overlays:

- toggled by `ToggleChartEarningsMarkers`
- fetched through `api::fetch_sec_earnings_events`
- rendered as labeled chart markers and hover tooltips
- lazily summarize hovered filings through `api::fetch_sec_filing_summary`,
  using SEC complete-submission text and earnings exhibits such as `EX-99.1`
- supplement hover summaries with standardized revenue, diluted EPS, and net
  income from the nearest associated 10-Q/10-K in the SEC Company Facts XBRL
  API, including comparable year-over-year changes when available
- clicked through `OpenChartEarningsFiling` to open the public SEC filing

Macro indicators are configured per chart and include candle/funding-derived
series. The candle-backed moving averages support active-timeframe, 1-hour,
daily, weekly, and monthly source series. Their menu and active badges live in
`chart_views/indicator_menu/` and `chart_views/indicator_badges/`.
Each EMA/SMA slot has an editable period (1–5000 candles), applied immediately
through `ChartMovingAveragePeriodChanged`. Periods are independent per chart,
source timeframe, and average type. The optional
`macro_indicators.moving_average_periods` map stores overrides under the existing
stable slot IDs (for example, `tf_ema_50` can use period 21). Missing or invalid
periods retain the original 50/200/20/12 defaults, preserving old layouts.
Temporary empty/zero edits keep the last valid period and are not persisted.
Canvas labels and active badges display the effective period; detached charts
and restored layouts retain it. Custom averages request additional warm-up
history, bounded to 5000 candles and subject to the source's available history.


`chart_indicator.rs` is the shared stable-ID registry used by the chart UI and
the Assistant workspace contract. Assistant actions set explicit enabled states
instead of toggling strings, so retries are idempotent. The advertised catalog
is restricted to reversible visual indicators on open candlestick charts;
presentation labels and Quick Trade controls are intentionally excluded. The
Rust host validates the complete batch and any integration dependency before it
changes a chart, then synchronizes render state and schedules layout/config
persistence through the normal chart path. Per-chart workspace snapshots expose
the effective `moving_average_periods`; stable IDs identify the original slots,
and action-result labels reflect any custom period.

The Assistant drawing bridge uses the existing annotation model rather than
simulating clicks in the drawing toolbar. `agent_snapshot.rs` exposes bounded
annotation geometry, style, selection, and coverage; `agent_workspace.rs`
accepts atomic create operations for the catalogued geometric shapes and remove
operations for persisted drawings. Pen creation is mouse-driven; snapshots expose
up to 64 pen points with an explicit total and truncation flag. New shapes receive
normal per-chart annotation IDs, are mirrored
into the canvas, and follow the existing config-persistence path. Exact
geometry/style retries are idempotent. Removal requires an exact current ID and
respects the annotation lock. Geometry/style edits remain user-driven through
Select mode, which keeps the first mutation surface small and deterministic.

Annotation drawing and hit testing iterate `AnnotationKind::anchor_points`
without allocating a handle list; fixed pairs are copied values and Fibonacci
anchors borrow their stored slice. Rendering also borrows selected/live annotations
and computes Fibonacci bounds directly. Hit testing checks topmost annotations
first and each annotation's handles before its body. Locked annotations remain
selectable, while the editing handlers enforce their locks.

## Detached Charts

Detached charts are opened from chart controls and rendered by
`main_view/windows.rs`.

They reuse:

- the same `ChartInstance`
- chart theme/preference sync
- candle/funding/overlay state

They maintain:

- detached window ID
- detached surface ID
- detached viewport state
- detached window geometry

Closing a detached chart must remove the detached window state without deleting
the chart instance if the chart is still present in any main or Canvas pane
grid.

## Chart Screenshots

`chart_screenshot/` exports the visible chart canvas to PNG. The camera action
freezes chart data, theme, privacy settings, and the capture timestamp. A thin
`ScreenshotCanvas` wrapper exposes the clicked surface's actual widget-local
`ChartState` and current logical size through a read-only iced operation. The
operation finishes before opening/focusing the preview window. Docked and
detached surfaces have separate IDs; a missing surface produces an error rather
than falling back to another chart or a cached heatmap viewport.

Rendering uses the same chart drawing code at the **original logical size**.
Only the offscreen renderer's pixel density increases: normally 3×, with at least
1920 pixels on the longest edge for small panes, subject to an 8192-pixel edge
and 12,582,912-pixel area limit. Fractional bounds are retained until final pixel
rounding, and oversized sources also obey the limits. This preserves candle
spacing, pan/zoom, blank space, inverted/manual price scales, funding/session
panel proportions, annotations, and indicator geometry across resolutions.
The snapshot retains the reset epoch and HUD price-follow state, so export uses
the same effective view as the live canvas after symbol/timeframe resets.

Pointer/drag affordances and widget menus are omitted. There is no extra ticker
badge painted over candles; symbol/timeframe remain in preview metadata and the
filename. The existing position-price and positions/orders privacy flags apply
only to the frozen export, and remain persisted with compatible defaults.
Copy Image and Save PNG use the same rendered pixels shown in the preview.
Late results from a closed or superseded request are ignored.

## Spaghetti Charts

Spaghetti charts are comparison charts with their own state and canvas engine.
They support:

- normalized comparison mode
- pair-ratio mode
- multiple selected symbols
- optional linkage to a named watchlist preset in comparison mode
- per-symbol candle fetches
- session anchoring
- style controls
- zoom/scroll/Y-scale interaction

Key modules:

- `spaghetti_state.rs`
- `spaghetti/model.rs`
- `spaghetti/axes.rs`
- `spaghetti/crosshair.rs`
- `spaghetti/normalized/`
- `spaghetti/ratio/`
- `spaghetti_update/`
- `spaghetti_views/`

Both render modes receive the same frame context from `spaghetti.rs` and share
grid lines, the value-axis border, relative-time labels, and session-start
markers in `spaghetti/axes.rs`. They also use the same background-frame setup
and crosshair overlay, with each mode supplying its optional hover-value label.
Each mode retains its value calculations and formatting; the zero-percent
baseline and positive-range guard for hover labels belong to normalized rendering.

Spaghetti data uses the shared candle backfill infrastructure where practical
but keeps its own chart instance map and canvas cache.
Selecting a watchlist preset from the comparison editor replaces the chart
series with that preset's assets and keeps the chart synchronized as the preset
changes. Symbol edits made while linked update the shared preset; **Unlink**
returns the chart to an independent symbol list. Pair-ratio charts remain
independent because they require exactly two symbols.

## Spread Chart

`spread_chart/` renders compact bid/ask spread history inside order-book views.
It has a small canvas program, hover readout, and resize interactions. The order
book view owns when spread samples are pushed and when the spread chart is
visible.

## Tests To Check

Use focused tests in these areas:

- `src/chart_state/model/tests/**`
- `src/chart_state/candles/cache/tests/**`
- `src/chart_state/heatmap/request/tests/**`
- `src/chart_update/candles/ws/tests.rs`
- `src/chart_update/detached/tests/**`
- `src/chart/tests/**`
- `src/chart/geometry/tests.rs`
- `src/chart/viewport/**/tests.rs`
- `src/chart/overlays/**/tests.rs`
- `src/chart/price_badges/tests/**`
- `src/chart_screenshot/tests/**`
- `src/spaghetti/**/tests`
- `src/spread_chart/**/tests`

For chart rendering changes, run targeted tests plus `cargo check`. For major
canvas or screenshot changes, also run the GUI smoke test when practical.
