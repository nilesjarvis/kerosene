# Volatility Metrics Integration

**Goal:** Add volatility context to Kerosene so a trader can compare moves across assets, size risk, and identify stretched or unusually active markets without leaving the app.

**Recommended first version:** expose three compact metrics first:

1. `RV24h` — current 24-hour realized volatility.
2. `NATR 1h` — normalized ATR for near-term range and stop sizing.
3. `ATR Dist` — signed distance from EMA20 in ATR units, e.g. `+2.4A`.

Add `Vol %ile` later once Kerosene has a stable historical volatility baseline/cache.

---

## Why Volatility Belongs In Kerosene

Raw percentage moves are hard to compare across assets. A 3% move in BTC, HYPE, and a thin HIP-3 perp can mean very different things. Volatility-normalized values answer better trading questions:

- Is this asset moving unusually much for itself?
- Is the current candle/day hot or quiet versus recent history?
- Is price stretched enough from its mean to care about fading it?
- How wide should stops or invalidation levels be?
- Which watchlist names are experiencing volatility expansion or compression?

For HYPE/alts mean-reversion, the most actionable readout is usually:

> Price is `+2.4 ATR` from the 1h EMA20 while `RV24h` is in a hot regime.

That says more than “up 3%” because it combines stretch and regime.

---

## Metrics To Compute

### 1. Close-To-Close Log Returns

Use log returns as the base series:

```text
r_t = ln(close_t / close_{t-1})
```

Ignore candles with missing, non-finite, or non-positive closes.

### 2. Realized Volatility

For a rolling window of `N` returns:

```text
sigma_N = stdev(r over N periods)
rv_window = sigma_N * sqrt(N) * 100
```

For trading UI, prefer non-annualized window values such as `RV24h` because they describe the range relevant to current positioning. If an annualized comparison is needed:

```text
rv_annualized = sigma_N * sqrt(periods_per_year) * 100
```

For crypto:

```text
daily periods per year = 365
hourly periods per year = 24 * 365
5m periods per year = 12 * 24 * 365
```

### 3. ATR And Normalized ATR

True range:

```text
TR_t = max(
  high_t - low_t,
  abs(high_t - close_{t-1}),
  abs(low_t - close_{t-1})
)
```

Average true range:

```text
ATR_N = average(TR over N periods)
```

Normalize for cross-asset display:

```text
NATR_N = ATR_N / close_t * 100
```

`NATR` is better than raw ATR in watchlists because it lets BTC, HYPE, and lower-priced alts share one comparable column.

### 4. ATR Distance From EMA20

Use ATR to express current stretch from a moving average:

```text
EMA20 distance = (last_price - EMA20) / ATR
```

Display as:

```text
+2.4A
-1.8A
```

This is the highest-value metric for mean-reversion scanning because it tells the user how far price is from its local mean in volatility units.

### 5. Volatility Percentile

Once enough history is cached, rank current volatility against the asset's own past distribution:

```text
Vol %ile = percentile_rank(current RV24h, trailing RV24h history)
```

Example:

```text
Current RV24h is in the 88th percentile of the last 180 days.
```

This should be a later addition because it needs a longer baseline and careful caching.

---

## Recommended Display Values

| Metric | Display | Meaning |
| --- | --- | --- |
| `RV24h` | `6.4%` | Realized volatility over the current 24h window. |
| `RV7d` | `18.2%` | Short-term baseline volatility. Optional in first version. |
| `NATR 1h` | `1.1%` | Typical 1h range as a percentage of price. |
| `NATR 4h` | `2.7%` | Wider swing/risk range. Optional in first version. |
| `ATR Dist` | `+2.4A` | Price stretch from EMA20 measured in ATR units. |
| `Vol %ile` | `88` | Current vol regime versus historical self. Later version. |

Formatting rules:

- Percent values: one decimal by default, two decimals for small values.
- Percentile: integer `0`-`100`.
- ATR distance: signed one decimal plus `A`, e.g. `+2.4A`.
- Missing/stale values: `-`.
- Keep chart/watchlist values non-annualized unless a dedicated analytics view explicitly asks for annualized volatility.

---

## App Integration Surfaces

### 1. Chart Header Metrics

Relevant files:

- `src/chart_views/header/metrics.rs`
- `src/chart_views/header/metrics/columns.rs`
- `src/chart_views/header/metrics/columns/formatting.rs`

Add compact header metrics for the active symbol:

```text
RV24h 6.4%   NATR 1h 1.1%   +2.4A
```

This is the best always-visible placement for the active chart. It should sit beside existing context like volume, mark/oracle, funding, and open interest.

Suggested responsive behavior:

- Wide chart: show `RV24h`, `NATR 1h`, and `ATR Dist`.
- Medium chart: show `RV24h` and `ATR Dist`.
- Narrow chart: show only `ATR Dist` or hide all volatility columns.

### 2. Live Watchlist Columns

Relevant files:

- `src/config/live_watchlist.rs`
- `src/market_state/types/live_watchlist.rs`
- `src/market_state/live_watchlist/rows.rs`
- `src/market_state/live_watchlist/columns.rs`
- `src/market_views/live_watchlist/rows/view/cells.rs`
- `src/market_views/live_watchlist/controls/header.rs`
- `src/market_views/live_watchlist/controls/columns.rs`

Add optional configurable columns:

```text
RV24h
NATR
ATR Dist
Vol %ile   # later
```

Example row:

```text
HYPE   42.10   +3.2%   RV24h 6.4%   NATR 1.1%   +2.4A
```

These should be sortable so the user can scan for:

- Highest volatility expansion.
- Lowest volatility compression.
- Largest positive or negative ATR stretch.
- High-volume names with unusually hot volatility.

Persist column visibility and sort selection through the existing live-watchlist config flow.

### 3. Screener Columns

Relevant files:

- `src/screener_state.rs`
- `src/screener_views.rs`
- `src/screener_update.rs`

The screener currently ranks by price, 24h/1h/15m change, volume, and funding. Add a compact volatility column set:

```text
Vol
Stretch
```

Possible layout:

```text
Ticker   Price   24h    1h    15m   Volume   Funding   Vol    Stretch
HYPE     42.10   +5.2   +1.8  +0.6  $183M    +0.012%   6.4%   +2.4A
```

For a first pass, prefer one column named `Vol` mapped to `RV24h`, plus a `Stretch` column mapped to `ATR Dist` if there is enough width.

### 4. Chart Canvas Badge

A small chart overlay or badge can summarize volatility without adding more header width pressure:

```text
RV24h 6.4% | NATR 1h 1.1% | +2.4A
```

This should be display-only and derived from cached state. Do not compute volatility inside canvas drawing code.

### 5. Order Ticket / Risk Helper

Once `NATR` and `ATR` are available, order-entry surfaces can show practical risk context:

```text
1 ATR move: $0.46
2 ATR stop: $0.92
Current NATR 1h: 1.1%
```

This is optional after the read-only market metrics are stable.

---

## Data Model Proposal

Add a shared volatility model rather than computing directly inside views.

Possible location:

- `src/market_state/volatility.rs`
- or `src/volatility.rs` if it will be reused broadly outside market state.

Suggested shape:

```rust
pub(crate) struct VolatilityMetrics {
    pub(crate) rv_24h: Option<f64>,
    pub(crate) rv_7d: Option<f64>,
    pub(crate) natr_1h: Option<f64>,
    pub(crate) natr_4h: Option<f64>,
    pub(crate) atr_distance_ema20: Option<f64>,
    pub(crate) vol_percentile: Option<f64>,
    pub(crate) updated_at_ms: u64,
}
```

Cache by symbol key:

```rust
HashMap<String, VolatilityMetrics>
```

Track loading/request state similarly to existing watchlist and screener refresh paths:

- request id
- requested symbols
- loading flag
- refresh pending flag
- last fetch timestamp

Views should only read `Option<f64>` values and render `-` when unavailable.

---

## Candle Data Requirements

Relevant files:

- `src/api/candles.rs`
- `src/api/candles/model.rs`
- `src/api/watchlist/history.rs`

Use OHLC candles for all volatility metrics:

- close-to-close RV needs `close`.
- ATR/NATR needs `high`, `low`, and previous `close`.
- EMA20 distance needs `close` and ATR over the same or compatible timeframe.

Do not overload the existing short watchlist history fetch if it only fetches enough data for 5m/30m/1h percent changes. Volatility windows need a dedicated refresh/caching path so watchlist responsiveness does not depend on heavier history requests.

Suggested candle windows:

| Metric | Candle timeframe | Minimum data |
| --- | --- | --- |
| `RV24h` | 5m or 15m | 24h of candles |
| `RV7d` | 1h | 7d of candles |
| `NATR 1h` | 1m/5m or 1h, depending interpretation | enough for ATR period |
| `NATR 4h` | 15m/1h | enough for ATR period |
| `ATR Dist` | 1h | EMA20 + ATR period |
| `Vol %ile` | 1h or 1d derived windows | long trailing baseline |

For the first version, use 1h candles for `ATR Dist` and 5m or 15m candles for `RV24h`.

---

## Refresh And Performance Policy

Volatility should update slower than live mids/books.

Suggested cadence:

- Active chart symbol: every 30-60 seconds.
- Visible live watchlist symbols: every 60-120 seconds.
- Screener universe: batched, top-volume or visible symbols first.

Guidelines:

- Batch requests to avoid hammering the API.
- Prioritize visible rows and the active chart.
- Keep stale values visible until replaced, with optional subtle stale styling later.
- Never block chart or watchlist rendering on volatility fetches.
- Keep all calculations outside view functions.

---

## Color Semantics

Use color carefully so volatility does not imply trade direction by itself.

Suggested rules:

- `RV24h` / `NATR`: neutral text by default.
- `Vol %ile < 30`: muted/quiet.
- `Vol %ile 30-70`: neutral.
- `Vol %ile > 70`: warning/hot.
- `Vol %ile > 85`: danger/extreme.
- `abs(ATR Dist) >= 2.0`: warning.
- `abs(ATR Dist) >= 3.0`: danger/extreme.

For `ATR Dist`, avoid using green/red purely because the value is positive/negative; a positive stretch can be a fade candidate, not necessarily bullish.

---

## Implementation Checklist

### Step 1: Add pure volatility calculations

Files:

- Create: `src/market_state/volatility.rs` or `src/volatility.rs`
- Test near the module with `#[cfg(test)]`

Add pure functions for:

- log returns
- realized volatility
- ATR
- NATR
- EMA20
- ATR distance from EMA20

Validation:

```bash
cargo test volatility
```

### Step 2: Add cached volatility state

Files likely involved:

- `src/app_state.rs`
- `src/market_state.rs`
- `src/message.rs`
- `src/app_update/routing.rs`
- a new or existing market update module

Add cached per-symbol metrics plus request lifecycle fields.

Validation:

```bash
cargo check
```

### Step 3: Add refresh tasks

Files likely involved:

- `src/api/candles.rs`
- `src/subscription_state/timers.rs`
- `src/subscription_state/market.rs`
- market update modules under `src/market_update*`

Fetch candle windows in batches, compute metrics in update/task code, and store results in cached state.

Validation:

```bash
cargo check
cargo test volatility
```

### Step 4: Render chart header metrics

Files likely involved:

- `src/chart_views/header/metrics.rs`
- `src/chart_views/header/metrics/columns.rs`
- `src/chart_views/header/metrics/columns/formatting.rs`

Add formatting helpers and responsive visibility rules.

Validation:

```bash
cargo test chart_views::header
cargo check
```

### Step 5: Add live watchlist columns

Files likely involved:

- `src/config/live_watchlist.rs`
- `src/market_state/types/live_watchlist.rs`
- `src/market_state/live_watchlist/rows.rs`
- `src/market_state/live_watchlist/columns.rs`
- `src/market_views/live_watchlist/rows/view/cells.rs`
- `src/market_views/live_watchlist/controls/header.rs`
- `src/market_views/live_watchlist/controls/columns.rs`

Add column enum variants, labels, widths, row values, sorting, cells, and column-picker entries.

Validation:

```bash
cargo test live_watchlist
cargo check
```

### Step 6: Add screener columns

Files likely involved:

- `src/screener_state.rs`
- `src/screener_views.rs`
- `src/screener_update.rs`

Add optional `Vol` and `Stretch` columns with sort support.

Validation:

```bash
cargo test screener
cargo check
```

### Step 7: Run final validation

```bash
cargo fmt -- --check
cargo test volatility
cargo check
```

For a broader change that touches persisted live-watchlist config, also run relevant config serialization tests if available.

---

## Suggested Rollout

1. Build the pure calculation module and tests.
2. Add active-chart volatility metrics first.
3. Add live-watchlist columns next.
4. Add screener ranking after batching/caching is stable.
5. Add percentile/regime metrics after longer history storage is designed.
6. Add order-ticket risk helpers last.

This keeps the first implementation small while still delivering the most useful trading context quickly.
