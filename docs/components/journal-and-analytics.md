# Journal And Analytics

The journal reconstructs user-level trade cards from raw Hyperliquid fills. It
shares account identity, fill cache, read-data provider, chart snapshot, and
portfolio analytics concepts with the broader account system.

For user-facing details and aggregation edge cases, see
[Trading Journal](../journal.md). This document focuses on implementation
boundaries.

## Component Map

| Component | Key files | Responsibility |
| --- | --- | --- |
| Journal state/model | `src/journal/state.rs`, `src/journal.rs` | Account-scoped journal state, filters, sorting, notes, status. |
| Fill fetch/cache | `src/journal/cache.rs`, `src/api/user_fills.rs`, `src/journal_update.rs` | Paged fill fetches, per-wallet cache, merge/dedup, cache writes. |
| Aggregation | `src/journal/aggregation/` | Fill identity, ordering, perpetual trade reconstruction, spot/outcome grouping. |
| Current positions | `src/journal/current_positions.rs` | Fallback partial trades from current account snapshot. |
| Snapshots | `src/journal/snapshot.rs`, `src/journal_views/trade_card/snapshot.rs` | Per-trade candle snapshots, markers, excursion metrics, embedded chart view. |
| Views | `src/journal_views/` | Journal window/pane, header, controls, summary, trade cards, notes editor. |
| Analytics | `src/account_analytics/`, `src/portfolio_state/`, `src/pnl_card/` | Portfolio/income snapshots and exportable visual summaries. |

## Account Scope

Journal data is scoped to the active account key. That avoids mixing notes,
fills, and cache state between saved accounts or ghost wallets.

Persisted data includes:

- journal entries and notes
- journal entries by account
- active account scope metadata

Runtime data includes:

- in-flight fill request
- loading/error status
- expanded trade cards
- chart snapshot cache
- current visible filters/sort where not persisted

## Fill Loading

Journal fills are fetched with Hyperliquid `userFillsByTime` through
`api::fetch_user_fills`.

The loading path:

```text
journal opens or refresh requested
  -> load per-wallet cache if available
  -> request user fills by time
  -> merge page into loaded fills
  -> deduplicate boundary fills
  -> continue paging if API cap reached
  -> aggregate trades
  -> save cache
```

The first request's end time acts as a sync watermark. Pagination advances from
the newest loaded fill timestamp, and local deduplication handles inclusive page
boundaries so same-millisecond fills are not skipped.

## Cache

`journal/cache.rs` stores per-wallet cache files:

```text
~/.config/kerosene/journal_cache_<address>.json
```

Platform-specific config directories are used on macOS and Windows. Cache
writes are atomic, and Unix permissions are restricted where supported.

Cache files contain market/account history and should not include private keys
or API keys.

## Fill Normalization

Aggregation starts by normalizing fill identity and order:

- deterministic chronological sorting
- composite identity deduplication
- same-millisecond position-chain repair

The composite identity includes time, trade/order IDs, hash, coin, side, price,
and size. This avoids dropping legitimate fills that share only one identifier.

Same-millisecond groups can arrive with transaction IDs that do not reflect
execution order. The journal chains fills by `startPosition` and signed size
when possible, falling back to deterministic sorting if the chain is ambiguous.

## Perpetual Trade Aggregation

Perpetual fills are walked from oldest to newest. The aggregator tracks one
current trade per coin and infers trade boundaries from position transitions:

- zero to non-zero opens a trade
- increasing exposure keeps the trade open
- reducing exposure keeps the trade open
- returning to zero closes the trade
- crossing through zero closes one trade and opens a flipped trade
- settlement fills add fee/PnL without changing size

Tracked values include:

- start and end time
- direction
- max absolute position
- volume
- fees
- realized PnL
- fill count
- average entry price
- whether basis is complete

If loaded history begins after the real open, the trade can be marked partial.

## Spot And Outcome Aggregation

Spot and outcome assets do not behave like perpetual margin positions, so the
journal groups them by order ID instead of open/close lifecycle.

Spot/outcome cards are marked as filled executions and excluded from closed
perpetual win-rate calculations.

## Current Position Reconciliation

When account data reports an open perpetual position that cannot be
reconstructed from loaded fills, `journal/current_positions.rs` can add a
partial open trade. These cards are useful for current exposure but clearly
communicate incomplete opening history.

## Notes

Trade notes are user-authored and persisted by account. Note changes should
update journal state and call config persistence, but they should not mutate
fill cache files.

## Chart Snapshots

Journal chart snapshots request candles around a trade and render a compact
trade-specific chart. Snapshot state includes:

- candle request context
- markers for trade fills
- excursion metrics
- cached snapshot results
- expanded snapshot trade IDs

When the read-data provider changes, snapshot cache is cleared so later
snapshots use the selected provider.

## Summary And Analytics Views

`journal_views/summary/` computes and renders:

- realized PnL summaries
- win-rate metrics
- fee totals
- top assets
- account value or PnL chart series

Portfolio/income analytics are adjacent but separate:

- `account_analytics/portfolio/`
- `account_analytics/income/`
- `portfolio_state/charts/`
- `pnl_card/`

These analytics should not be used as the authoritative source for trading
validation.

## Tests To Check

Use focused tests in:

- `src/journal/tests/**`
- `src/journal/aggregation/**/tests`
- `src/journal/current_positions/tests.rs`
- `src/journal/cache/tests.rs`
- `src/journal_views/summary/chart/tests.rs`
- `src/journal_views/trade_card/**/tests`
- `src/api/user_fills/**/tests`
- `src/account_analytics/**/tests`
- `src/portfolio_state/**/tests`
- `src/pnl_card/tests/**`

For bug fixes in aggregation, add a regression test with the smallest fill set
that reproduces the issue.
