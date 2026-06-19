# Trading Journal

The Trading Journal turns raw Hyperliquid fills into position-level trade cards. It is meant to answer practical questions such as:

- Which trades were opened, closed, or still active?
- How many fills made up the trade?
- What was the maximum position size?
- What realized PnL and fees came from the grouped fills?
- Which trades may be incomplete because the opening history was not loaded?

The journal is reconstructed locally from fill history. Hyperliquid provides individual fills, not user-authored "trade" records, so Kerosene has to infer trade boundaries from position transitions.

## User-facing behavior

Open the Trading Journal from the account tabs or add-widget menu after connecting an account. Kerosene loads the connected wallet's fill history, groups fills into trades, and renders a scrollable list of trade cards.

Each card shows:

- Asset.
- Trade status: `OPEN`, `CLOSED`, or `FILLED` for spot/outcome executions.
- Open time and duration.
- Direction: long, short, spot, or outcome.
- Maximum position size.
- Fill count.
- Realized PnL.
- Fees.
- Optional reflections: entry thesis, exit reflection, cause of error, and tags.

The summary header aggregates the currently visible cards:

- Total realized PnL.
- Closed win rate for complete perpetual trades.
- Cumulative fees.
- Most-traded assets.

Muted tickers are hidden from the journal. The filter can show all trades, only perpetual trades, or spot trades. Sorting can be by time or realized PnL.

## Data loading and cache

Journal fills are fetched with Hyperliquid's `userFillsByTime` info request:

```json
{
  "type": "userFillsByTime",
  "user": "<wallet>",
  "startTime": 0,
  "endTime": "<now>",
  "aggregateByTime": false
}
```

Kerosene requests unaggregated fills because aggregation has to preserve individual execution size, side, fee, PnL, order id, transaction hash, and `startPosition`.

Large histories are paginated forward when a page returns the API cap. Kerosene
keeps the first request's `endTime` as a fixed sync watermark, then advances the
next page's `startTime` to the newest loaded fill timestamp. Page boundaries are
inclusive, so duplicate boundary fills are deduplicated locally rather than
skipping same-millisecond executions.

Loaded fills are cached per wallet in the platform config directory:

- Linux: `~/.config/kerosene/journal_cache_<address>.json`
- macOS: `~/Library/Application Support/kerosene/journal_cache_<address>.json`
- Windows: `%APPDATA%\kerosene\journal_cache_<address>.json`

On normal journal open, Kerosene can show cached data immediately and then fetch fills since the newest cached fill. A manual full refresh starts from the beginning of history.

If `clearinghouseState` reports a current open perp position that cannot be
reconstructed from loaded fills, the journal adds a partial open trade from the
current account snapshot. These fallback cards are marked partial because the
opening fills, fees, and exact open time are outside the loaded fill history.

## Fill normalization

Before aggregation, fills are normalized in `src/journal/aggregation/identity.rs`.

Normalization does three things:

1. Sort fills into a deterministic chronological order.
2. Deduplicate exact fills using a composite identity.
3. Repair same-millisecond execution order by chaining positions.

The composite fill identity includes:

- `time`
- `tid`
- `oid`
- `hash`
- `coin`
- `side`
- `px`
- `sz`

This avoids dropping legitimate fills that happen to share only one identifier.

### Same-millisecond position chains

Some high-activity executions arrive with many fills sharing the same millisecond. `tid` order is not guaranteed to match execution order inside that millisecond. If Kerosene orders those fills by `tid`, the local position reconstruction can become discontinuous and invent false flips.

For fills with the same `(time, coin)`, Kerosene now derives an edge for each fill:

```text
start = fill.startPosition
end   = start + signed_size
```

Signed size is positive for side `B` and negative for side `A`. Settlement fills keep `end = start`.

Kerosene then orders the group by following the position chain:

```text
fill A end position == fill B start position
```

within a small epsilon. This restores the exchange's effective position order for same-millisecond open and close clusters. If a group cannot be parsed or has no clear chain head, Kerosene falls back to the deterministic sorted order.

## Perpetual aggregation

Perpetual aggregation lives in `src/journal/aggregation.rs`.

The aggregator walks normalized fills from oldest to newest and keeps one current trade per coin. For each fill, it parses:

- Size.
- Price.
- Fee.
- Closed PnL.
- API `startPosition`.
- Direction and side.

The signed fill size determines the transition:

```text
new_position = start_position + signed_size
```

except for settlement fills, which do not change the position.

### Trade boundaries

The journal infers trade boundaries from position transitions:

- Opening or increasing a position keeps the current trade open.
- Reducing but not closing a position keeps the current trade open.
- Reaching zero closes the current trade.
- Crossing through zero closes the old trade and starts a new flipped trade with the remainder.
- Settlement fills add fee/PnL to the current trade without changing position size.

The trade tracks:

- `start_time`: first fill in the inferred trade.
- `end_time`: close fill time, if closed.
- `max_position`: largest absolute position seen while the trade was active.
- `volume`: notional volume from fills attributed to the trade.
- `fee`: summed fee.
- `pnl`: summed realized closed PnL.
- `fill_count`: number of fills.
- `avg_entry_price`: weighted average of fills that increased exposure.
- `basis_complete`: whether the loaded history includes the trade's opening basis.

### Same-timestamp start-position safety

Kerosene also tracks the locally reconstructed position for each coin. When the next fill has the same timestamp, the local position is only used if it matches the fill's API `startPosition` within epsilon.

If it does not match, Kerosene trusts the API `startPosition` and increments a diagnostic counter. This prevents a bad local chain from creating a false flip or false trade split.

The user-facing warning says that same-timestamp fills used API `startPosition` because local position tracking was discontinuous.

## Spot and outcome aggregation

Spot coins start with `@`. Outcome markets start with `#`.

These assets do not behave like open margin positions in the journal, so Kerosene does not infer open/close lifecycles for them. Instead, spot and outcome fills are grouped by order id:

- Status is `FILLED`.
- Direction label is `Spot` or `Outcome`.
- Max position is the net signed size for that order.
- Average entry price is weighted by fill size.
- Fees and notional volume are summed.

Spot/outcome trades are excluded from closed perpetual win-rate calculations.

## Partial history

A perpetual trade is marked partial when the first loaded fill starts from a non-zero position. This means Kerosene sees a close, reduce, or increase for a position whose opening fill is outside the loaded history.

Partial trades still show their visible realized PnL and fees, but they are excluded from closed win-rate calculations because the full basis is unknown.

Common causes:

- The wallet had positions before the earliest loaded fill.
- The API did not return complete historical fills.
- A cache was manually edited or truncated.

The card displays:

```text
Partial history: opening fills are outside the loaded data.
```

## Notes and stable IDs

Journal notes are keyed by trade id. New trade ids are derived from stable fill data rather than just timestamp-based legacy ids.

To avoid orphaning old notes after aggregation improvements, each trade also carries legacy note ids:

- Time-based legacy ids, such as `<coin>_<timestamp>`.
- Stable ids for fills that belong to the trade.
- Flip variants for trades created by a position flip.

When rendering a card, Kerosene first looks for a note under the current trade id. If none exists, it checks the legacy ids and migrates on save.

## Diagnostics

Aggregation diagnostics are collected alongside the trade list:

- `skipped_fill_count`: fills whose numeric fields could not be parsed.
- `incomplete_trade_count`: trades marked partial because opening basis is missing.
- `same_timestamp_position_mismatch_count`: same-millisecond fills where local position tracking did not match API `startPosition`.

If any diagnostic is non-zero, the journal shows a data-quality warning above the trade list.

## Implementation map

Key files:

- `src/api/user_fills.rs`: fetches unaggregated fills and computes pagination requests.
- `src/journal/cache.rs`: reads and writes the per-wallet fill cache.
- `src/journal/aggregation/identity.rs`: normalizes, deduplicates, and same-timestamp position-chain sorts fills.
- `src/journal/aggregation/position.rs`: computes signed size, start-position resolution, close detection, and flip detection.
- `src/journal/aggregation/builders.rs`: creates and updates `AggregatedTrade` records.
- `src/journal/aggregation/model.rs`: trade and diagnostics structs.
- `src/journal/aggregation.rs`: main fill-to-trade aggregation loop.
- `src/journal_update.rs`: handles loading results, cache save, warnings, notes, filters, and refresh.
- `src/journal_views/`: renders header, summary, filters, status, trade cards, and note editor.

## Regression coverage

Journal tests include static same-millisecond fill fixtures derived from a real data-quality report. They do not store the wallet address and do not call the API.

The covered cases include:

- Many same-millisecond open-long fills that must chain from zero to the final position.
- Same-millisecond close-long fills that must remain part of the long trade, not create a false short.
- A large close bucket with more than thirty fills sharing the same millisecond.

These tests protect the main user-visible failure mode: inaccurate trade cards caused by ordering same-millisecond fills by `tid` instead of by position continuity.
