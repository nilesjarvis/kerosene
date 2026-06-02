# Journal Chart Snapshots Plan

Last updated: 2026-06-02.

## Goal

Enrich journal trade cards with a compact candlestick snapshot for the traded
asset. The snapshot should show trade context, buy/sell fills, and derived move
metrics so a trader can quickly understand what happened during the position.

The feature should be read-only from the journal: it should not place orders,
mutate chart state, or require an interactive chart instance.

## Current Fit

The existing journal and chart systems already provide most of the raw pieces:

- `src/journal/state.rs` stores full `raw_fills` and aggregated `trades`.
- `src/journal/aggregation/model.rs` stores trade start/end time, coin, side,
  max position, fees, PnL, fill count, and average entry price.
- `src/api/candles.rs` already fetches Hyperliquid `candleSnapshot` data and
  supports chart backfill through the configured source.
- `src/timeframe.rs` already models `1m`, `3m`, `5m`, and broader intervals.
- `src/chart_state/overlays/trades.rs` already maps fills into buy/sell marker
  data for chart overlays.

The main missing piece is per-trade fill attribution. `AggregatedTrade` records
counts and totals, but it does not retain the specific fills that belong to the
trade. Journal snapshots need those fills to draw markers and compute close/exit
metrics accurately.

## Proposed User Experience

Add a collapsible snapshot section to each journal trade card.

When collapsed, the existing card remains compact. When expanded, show:

- A mini candlestick chart for the trade window.
- Buy and sell markers at fill price/time.
- Open and close markers or subtle vertical guides.
- A small metrics row with timeframe, raw asset move, directional move, max
  adverse excursion, max favorable excursion, and asset peak-to-trough drawdown.
- A loading or unavailable state when candles are still fetching or no candle
  source has data for the requested window.

Snapshots should be loaded lazily when a trade is expanded or selected, not for
every visible card.

## Data Model Additions

Add journal-owned snapshot state rather than reusing live chart instance state.
Suggested model:

```rust
pub struct JournalTradeSnapshot {
    pub trade_id: String,
    pub coin: String,
    pub timeframe: Timeframe,
    pub start_ms: u64,
    pub end_ms: u64,
    pub candles: Vec<Candle>,
    pub markers: Vec<TradeMarker>,
    pub metrics: JournalTradeSnapshotMetrics,
    pub status: JournalTradeSnapshotStatus,
}
```

Suggested journal state fields:

```rust
pub expanded_snapshot_trade_id: Option<String>,
pub snapshot_requests: HashMap<String, JournalTradeSnapshotRequest>,
pub snapshots: HashMap<String, JournalTradeSnapshot>,
```

If multiple cards may be expanded at once, replace `expanded_snapshot_trade_id`
with a `HashSet<String>`.

## Per-Trade Fill Attribution

Add an aggregation variant that can return fill identities or lightweight fill
copies per trade.

Preferred direction:

- Keep the existing `AggregatedTrade` public shape stable if possible.
- Add a separate `AggregatedTradeFills` or `JournalTradeDetails` map keyed by
  `trade.id`.
- Reuse the current same-timestamp normalization path so attribution matches
  existing journal trade boundaries.
- Include flip attribution carefully: the closing portion belongs to the old
  trade, while the opening remainder belongs to the new flipped trade.

This keeps notes and summary behavior stable while giving snapshots the richer
data they need.

## Candle Planning

`1m` candles should be supported. Native Hyperliquid `1m` candles are available
for recent windows, but exploratory checks showed older native windows can return
empty `1m` data while broader intervals still return data. The planner should
therefore choose the desired interval by duration, then retry upward if the
selected interval returns no candles.

Suggested interval ladder:

```text
1m -> 3m -> 5m -> 15m -> 30m -> 1h -> 2h -> 4h -> 8h -> 12h -> 1d -> 3d -> 1w
```

Suggested selection rule:

- Start with the finest interval that keeps the padded window under roughly
  `160` candles.
- For sub-hour trades, this usually means `1m`.
- For multi-hour trades, move to `3m`, `5m`, or `15m`.
- For multi-day trades, use hourly or multi-hour candles.
- If the first request returns empty, retry the next coarser interval.
- Fetch through `fetch_chart_backfill_candles` so the configured backfill source
  can provide finer older history when available.

Suggested padding:

```text
padding = max(trade_duration * 0.25, 6 * interval_duration, 30 minutes)
```

For open trades, use the current time as the right edge.

## Metrics

Compute metrics from candles that overlap the trade interval, not only candles
whose open time falls inside the trade. This matters for very short trades where
one candle may cover the full position.

Suggested metrics:

- Raw asset move: `(exit_price - entry_price) / entry_price`.
- Directional move: raw move for longs, inverted raw move for shorts.
- Max adverse excursion:
  - Long: lowest overlapping low relative to entry.
  - Short: entry relative to highest overlapping high.
- Max favorable excursion:
  - Long: highest overlapping high relative to entry.
  - Short: entry relative to lowest overlapping low.
- Asset peak-to-trough drawdown across overlapping candles.
- Candle timeframe and candle count.

Use fill VWAPs when possible:

- Entry: existing `avg_entry_price` or VWAP of fills that increased exposure.
- Exit: VWAP of closing fills for closed trades.
- Open trades: last overlapping candle close or latest mid/reference price.

## Rendering Approach

Start with a purpose-built journal snapshot canvas rather than embedding the full
interactive `CandlestickChart`.

Reasons:

- The journal snapshot is read-only and compact.
- It avoids chart interaction state, order overlays, funding panels, heatmaps,
  and quick-order behavior.
- It can use the same colors and marker semantics as chart overlays while
  remaining scoped to journal cards.

The canvas should render:

- Grid and axes with minimal labels.
- Candlestick bodies and wicks.
- Buy dots in success color and sell dots in danger color.
- Optional entry/exit guide lines.
- Empty/loading/error states sized consistently with the card.

## Messages And Update Flow

Suggested messages:

```rust
JournalSnapshotToggle(String)
JournalSnapshotLoaded {
    trade_id: String,
    request: JournalTradeSnapshotRequest,
    result: Result<Vec<Candle>, String>,
}
```

Update flow:

1. User toggles a card snapshot.
2. If cached snapshot exists, show it.
3. If missing or stale, build a candle request from the trade and attributed
   fills.
4. Fetch candles with `Task::perform`.
5. On success, compute metrics and store the snapshot.
6. On empty candles, retry the next coarser interval.
7. On final failure, show a compact unavailable state.

## Testing Plan

Add focused tests near journal code:

- Timeframe planner chooses `1m` for short trades.
- Planner chooses coarser intervals for longer trades.
- Empty candle responses retry the next interval.
- Overlapping candle selection includes a candle that spans a sub-minute trade.
- Long and short directional metrics are correct.
- Max adverse and max favorable calculations are direction-aware.
- Fill attribution preserves same-timestamp chain ordering.
- Flip trades attribute closing and opening portions to the correct snapshots.

UI-only rendering can be covered by compile checks initially, with canvas logic
tested through pure layout/metric helpers.

## Sample Probe Notes

A sample wallet provided in the working thread was used only for read-only API
exploration and is intentionally not recorded here.

Observed from the first 2,000 fills in that sample:

- The journal aggregation produced closed HYPE, BTC, and ETH trades.
- Same-timestamp position-chain ordering was important and produced clean
  aggregation when mirrored correctly.
- Recent native Hyperliquid `1m` candles returned data for HYPE and BTC.
- Older native Hyperliquid windows tested around 2026-05-20 returned no `1m`
  candles, while `5m` and broader intervals returned data.

The implementation should support `1m` first and fall back only when the selected
source has no data for that interval/window.
