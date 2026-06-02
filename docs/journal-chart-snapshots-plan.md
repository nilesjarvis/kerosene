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
- `src/chart_state/overlays/trades.rs` already maps account-data fills into
  buy/sell marker data for chart overlays.

The main missing piece is per-trade fill attribution. `AggregatedTrade` records
counts and totals, but it does not retain the specific fills that belong to the
trade. Journal snapshots need those fills to draw markers and compute close/exit
metrics accurately.

The existing chart trade marker helper is useful for marker semantics, but it is
not a drop-in dependency for journal snapshots because it consumes
`crate::account::UserFill`. Journal snapshots should parse markers from
journal-owned `crate::api::UserFill` attribution fragments instead.

## Proposed User Experience

Add a collapsible snapshot section inside each journal trade card in the journal
window UI. Each trade card should expose its own snapshot toggle; expanding one
card must not require opening or mutating an interactive chart pane.

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

Multiple trade cards may be expanded at once. This avoids surprising collapses
when a trader compares nearby trades in the journal.

## Data Model Additions

Add journal-owned snapshot state rather than reusing live chart instance state.
Suggested models:

```rust
pub struct JournalTradeDetails {
    pub trade_id: String,
    pub coin: String,
    pub attributed_fills: Vec<JournalAttributedFill>,
}

pub struct JournalAttributedFill {
    pub identity: FillIdentity,
    pub time_ms: u64,
    pub price: f64,
    pub raw_size: f64,
    pub attributed_size: f64,
    pub side: String,
    pub role: JournalAttributedFillRole,
    pub fee: f64,
    pub closed_pnl: f64,
}

pub enum JournalAttributedFillRole {
    Increase,
    Reduce,
    FlipClose,
    FlipOpen,
    Settlement,
}

pub struct JournalTradeSnapshotRequest {
    pub account_key: Option<String>,
    pub address: String,
    pub trade_id: String,
    pub coin: String,
    pub source: ChartBackfillSource,
    pub timeframe: Timeframe,
    pub ladder_index: usize,
    pub start_ms: u64,
    pub end_ms: u64,
}

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
pub trade_details: HashMap<String, JournalTradeDetails>,
pub expanded_snapshot_trade_ids: HashSet<String>,
pub snapshot_requests: HashMap<String, JournalTradeSnapshotRequest>,
pub snapshots: HashMap<String, JournalTradeSnapshot>,
```

These fields must be account-scoped. Mirror them into `JournalAccountState`, and
include them in active-account snapshot/restore paths so account switches do not
leak cached candles, expanded card state, or in-flight requests across wallets.
Clear or invalidate snapshot state when the active account data is cleared, the
journal is force-refreshed, the chart backfill source changes, or an in-flight
request no longer matches the active account/source.

## Per-Trade Fill Attribution

Add an aggregation variant that returns attributed fill fragments per trade.
Identities alone are not sufficient because a flip can split one raw fill across
two trades.

Preferred direction:

- Keep the existing `AggregatedTrade` public shape stable if possible.
- Add a separate `AggregatedTradeFills` or `JournalTradeDetails` map keyed by
  `trade.id`.
- Reuse the current same-timestamp normalization path so attribution matches
  existing journal trade boundaries.
- Include flip attribution carefully: the closing portion belongs to the old
  trade, while the opening remainder belongs to the new flipped trade.
- Store attributed sizes and roles so snapshot markers and VWAP metrics can use
  the same split math as aggregation totals.

This keeps notes and summary behavior stable while giving snapshots the richer
data they need.

The first implementation should prioritize perp trades with complete opening
basis. Spot, outcome, and partial-history trades can render an unavailable or
simplified state until their semantics are intentionally designed.

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
- Treat an empty successful candle response as retryable only within the journal
  snapshot ladder. Existing interactive chart candle loading treats empty data as
  an unavailable chart state and should not be changed for this feature.

Suggested padding:

```text
padding = max(trade_duration * 0.25, 6 * interval_duration, 30 minutes)
```

For open trades, use the current time as the right edge.

Snapshot requests should carry the account key, address, selected source, time
range, timeframe, and ladder index. Loaded responses must be ignored if any of
those request fields are stale by the time the async task completes.

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

- Entry: existing `avg_entry_price` or VWAP of attributed fills that increased
  exposure.
- Exit: VWAP of attributed closing fills for closed trades.
- Open trades: last overlapping candle close or latest mid/reference price.

Flip fills should contribute their attributed closing size to the closing
trade's exit VWAP and their attributed opening size to the new trade's entry
VWAP.

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

Place the canvas directly inside the expanded trade card body below the existing
details row and before notes/editor content. The collapsed state should preserve
the current card density, with only a compact snapshot toggle added to the
details/actions row.

## Messages And Update Flow

Suggested messages:

```rust
JournalSnapshotToggle(String)
JournalSnapshotLoaded {
    account_key: Option<String>,
    address: String,
    request: JournalTradeSnapshotRequest,
    result: Result<Vec<Candle>, String>,
}
```

Update flow:

1. User toggles a card snapshot.
2. Add or remove the trade ID from `expanded_snapshot_trade_ids`.
3. If collapsed, keep cached snapshot data but do not fetch.
4. If cached snapshot exists and matches the current account/source, show it.
5. If missing or stale, build a candle request from the trade and attributed
   fills.
6. Store the request in `snapshot_requests` and fetch candles with
   `Task::perform`.
7. On response, verify account key, address, source, request fields, and active
   in-flight request before mutating journal state.
8. On success with candles, compute metrics and store the snapshot.
9. On success with empty candles, retry the next coarser interval from the
   snapshot ladder.
10. On final failure or exhausted ladder, show a compact unavailable state.

## Implementation Milestones

1. Add aggregation detail output and tests for attributed fill fragments.
2. Add snapshot planning, candle overlap, VWAP, and metric helpers with focused
   pure tests.
3. Add account-scoped snapshot state, messages, stale-response guards, and lazy
   candle fetch flow.
4. Add the collapsible per-trade-card UI with loading, unavailable, and loaded
   states.
5. Add the journal snapshot canvas and compile-level UI validation.

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
- Snapshot state is saved/restored per active journal account.
- Stale snapshot responses are ignored after account/source/request changes.
- Collapsing a snapshot does not trigger a fetch, and re-expanding can reuse a
  matching cached snapshot.

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
