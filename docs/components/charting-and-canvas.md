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
| Screenshots | `src/chart_screenshot/` | Screenshot UI, capture bounds, bitmap primitives, labels, PNG export. |
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
- SEC earnings marker state
- funding fetch state
- macro indicator config
- header metric display modes

`TradingTerminal` stores chart instances in `charts: HashMap<ChartId,
ChartInstance>` and allocates IDs through `alloc_chart_id`.

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
  -> api::fetch_chart_backfill_candles
  -> Message::ChartCandlesLoaded
  -> stale request guard checks exact request
  -> candles normalized and merged
  -> shared candle cache updated
  -> chart cache invalidated
  -> overlays and funding/heatmap/liquidations may refresh
```

`CandleFetchRequest` includes chart ID, runtime chart-incarnation generation,
symbol, timeframe, source/provider generations, time range, and attempt number.
Result handling first requires the current incarnation and provider owners,
then compares the incoming request with the currently stored request before
applying it. Runtime layout restoration advances the incarnation before it
reconstructs persisted chart IDs, so a surviving task from the prior layout
cannot consume an otherwise identical primary or comparison request on the
replacement chart.

Hourly, daily, weekly, and monthly macro-candle batches carry that same outer
incarnation plus their per-chart batch ID. This keeps macro history owned by the
replacement chart even though a reconstructed `ChartInstance` starts its local
batch sequence again. All valid merge, retry, cache, backfill, and macro
indicator behavior is unchanged.

The backfill source comes from `ReadDataProvider`:

- Hyperliquid for default reads.
- Hydromancer when selected and an API key is available.
- Fallback behavior when Hydromancer is selected without a usable key.

## Shared Candle Cache

`chart_state/candles/cache.rs` stores candle series by `(symbol, Timeframe)`.
It is reused by regular charts and spaghetti charts. The cache is bounded to
avoid unbounded memory growth.

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

`Message::ChartWsCandleUpdate` applies updates to matching loaded chart
instances. It updates the current series, triggers price flashes, invalidates
render caches, and can schedule funding refreshes when macro panels need them.

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

Asset context streams supply mark/oracle/mid/open-interest/funding-like metadata
for chart headers and overlays. `ChartWsAssetCtxUpdate` applies matching
contexts to chart instances unless the symbol is hidden.

Header metric display modes can show values as raw or USD notional depending on
the market and user preference.

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

Interaction messages should carry chart ID and surface ID so detached windows
and inline panes do not fight over state.

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
- clicked through `OpenChartEarningsFiling` to open the public SEC filing

Macro indicators are configured per chart and include candle/funding-derived
series. The candle-backed moving averages support active-timeframe, 1-hour,
daily, weekly, and monthly source series. Their menu and active badges live in
`chart_views/indicator_menu/` and `chart_views/indicator_badges/`.

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
the chart instance if the chart is still present in the main pane grid.

## Chart Screenshots

`chart_screenshot/` exports a chart to PNG. It handles:

- screenshot menu and settings
- chart bounds and sizing
- offscreen capture model
- bitmap primitives and glyphs
- optional labels
- file save/copy behavior
- privacy-sensitive display flags

Screenshot settings are persisted, but generated images are output artifacts
and should not include secrets.

## Spaghetti Charts

Spaghetti charts are comparison charts with their own state and canvas engine.
They support:

- normalized comparison mode
- pair-ratio mode
- multiple selected symbols
- per-symbol candle fetches
- session anchoring
- style controls
- zoom/scroll/Y-scale interaction

Key modules:

- `spaghetti_state.rs`
- `spaghetti/model.rs`
- `spaghetti/normalized/`
- `spaghetti/ratio/`
- `spaghetti_update/`
- `spaghetti_views/`

Spaghetti data uses the shared candle backfill infrastructure where practical
but keeps its own chart instance map and canvas cache.

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
