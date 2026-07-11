# Account, Wallet, And Portfolio

The account system connects a Hyperliquid wallet address and agent key to live
account state. It merges REST snapshots, websocket user-data updates, all-mids,
wallet tracking, portfolio history, income analytics, and user-facing account
views.

## Component Map

| Component | Key files | Responsibility |
| --- | --- | --- |
| Account API/model | `src/account.rs`, `src/account/` | Account data fetches, types, HIP-3 normalization, merge logic, spot data, wallet fetchers. |
| Account runtime | `src/account_state.rs`, `src/account_update/` | Active profile, connect/disconnect, account refresh, user stream application, profile picker. |
| Account views | `src/account_views/` | Summary bar, positions, open orders, balances, history, account picker, income. |
| Wallet tracker | `src/wallet_state/`, `src/wallet_update/`, `src/wallet_views/` | Watch-only tracked wallets, address book, detail windows, snapshot refreshes. |
| Wallet clusters | `src/wallet_cluster_state.rs`, `src/wallet_cluster_update.rs`, `src/wallet_cluster_views.rs` | Saved groups of trading profiles, aggregate positions, and split order submission. |
| Portfolio | `src/portfolio_state/`, `src/portfolio_update.rs` | Portfolio history, PnL charts, income state and refreshes. |
| Analytics and metrics | `src/account_analytics/`, `src/account_metrics.rs`, `src/pnl_card/` | Portfolio/income HTTP fetches, position metrics, exportable PnL cards. |
| User streams | `src/ws/user_streams/`, `src/subscription_state/user_data.rs` | Mids, fills, orders, positions, balances, and account updates. |

## Account Profiles

Saved account profiles contain user-facing metadata and secret references:

- label
- wallet address
- secret ID
- active profile selection
- ghost account markers for in-memory watch-only profiles

Agent private keys are not serialized into plaintext profile config. They are
stored in OS keychain or encrypted config through the secret-storage layer.

The account picker can:

- select saved accounts
- rename profiles
- add accounts
- add ghost wallets
- forget ghost accounts
- delete saved accounts
- save credentials for the active profile

## Connect Flow

```text
ConnectWallet
  -> validate/normalize wallet address and key state
  -> fetch account data with selected read provider
  -> fetch portfolio history
  -> bootstrap all-mids
  -> load journal account/cache
  -> subscribe user data stream
  -> AccountDataLoaded / portfolio messages / mids messages
```

Account data fetches use `fetch_account_data_scoped_with_provider`, which can
choose Hyperliquid or Hydromancer-backed reads depending on provider settings
and available keys.

## Account Data Model

The account model includes:

- clearinghouse state
- positions
- open orders
- fills and trade history
- spot balances
- margin/equity/account value fields
- per-dex and HIP-3 normalization
- data freshness metadata
- fetch scope/completeness indicators

Key modules:

- `account/types/`
- `account/data/bootstrap/`
- `account/data/merge.rs`
- `account/data/fees.rs`
- `account/spot.rs`
- `account/wallets/`

REST snapshots are merged with websocket events and optimistic local updates
where safe. The model tracks freshness so high-risk actions can reject stale
data.

Spot balances have independent completeness, fetch time, and revision state;
position freshness is not used as a proxy. Account bootstrap fetches spot and
perpetual clearinghouse state independently so a failed perpetual read does not
discard a valid spot snapshot. Percentage orders require a complete, fresh
spot snapshot and use the selected pair's verified base/quote token identities.

## User Data Stream

`subscription_state/user_data.rs` creates `WsUserDataStreamParams` for:

- connected account private data
- all-mids across visible dexes
- wallet detail windows that need independent watch-only updates
- selected wallet cluster member addresses, without duplicate all-mids streams

`account_update/stream.rs` applies:

- open order updates
- fills
- positions
- balances
- all-mids
- repair/refresh triggers when websocket state is lagging or incomplete
- Chase/TWAP reconciliation signals
- chart overlay synchronization

Websocket updates should not blindly override newer local verification state.
Tests cover stale websocket behavior for advanced order reconciliation.

A targeted `spotState` frame replaces balances, marks them fresh, and advances
the spot-balance revision. A spot fill can arrive before that frame, so any live
spot fill first marks balances incomplete. Signed spot dispatches do the same;
percentage sizing remains blocked until the balance lane or a full refresh
reconciles the resulting totals and holds.

## Positions

Positions are shown in `account_views/positions/`.

Features include:

- sort by configured column/direction
- hide/unhide positions
- show hidden positions toggle
- close-position controls
- NUKE routing eligibility
- summary rows and account-value calculations
- per-position PnL and funding metrics
- PnL card export entry point

Hidden positions are scoped by account and persisted. Hidden/muted exposure is
a trading risk boundary and must be considered by close/NUKE/order automation.

## Open Orders

Open orders are rendered by `account_views/orders/`. Rows can include:

- confirmed open orders from account data
- locally pending placement indicators
- cancel actions
- reduce-only metadata
- chart overlay synchronization

Order cancellation routes through signed order execution. Views should emit
messages, not call signing functions.

## Balances And History

Account views cover:

- spot balances
- trade history
- funding history
- portfolio tab content
- income view

Spot and outcome balances can feed order-entry helpers such as outcome sell
prefill.

## Wallet Tracker

The wallet tracker is watch-only. It tracks addresses without agent keys and
shows snapshots across positions, spot balances, and open order counts.

Key modules:

- `wallet_state/model.rs`
- `wallet_state/tracker/`
- `wallet_state/address_book/`
- `wallet_update/tracker/`
- `wallet_views/tracker/`

Wallet tracker features:

- add/remove tracked addresses
- labels and address book
- import/export wallet labels
- periodic refresh
- detail windows per wallet
- open-order counts
- HIP-3 and spot fallback handling

Portfolio-margin headline equity and available balance are spot-state values,
not the values reported by an individual perpetual clearinghouse. Tracker
refreshes therefore inspect spot state even when the perp response is positive,
price non-stable balances only from exact validated spot marks, and include
negative balances. Missing spot state or marks produce a redacted valuation
warning and retain a usable perp snapshot rather than silently presenting a
partial spot total as authoritative.

Tracked wallets are persisted, but any private trading keys are not part of
wallet tracker state.

## Wallet Details

Wallet detail windows use `window::Id` and are rendered outside the main pane
grid. They can subscribe to user-data streams for their own address and show:

- summary
- warnings
- positions
- orders
- spot balances
- labels

The detail window should not mutate the connected trading account unless a
message explicitly targets account profile state.

For portfolio-margin wallets, detail-window equity is recomputed from spot
balances and token-0 maintenance availability. If any material held balance
cannot be priced, the headline value is unavailable instead of showing a
plausible but incomplete number; row-level values and PnL likewise preserve an
explicit unavailable state.

## Wallet Clusters

Wallet clusters are persisted groups of saved account profiles. They let a user
open a dedicated window, add saved trading profiles to a cluster, assign
relative order weights, view aggregate loaded positions, and submit one-shot
orders or reduce-only close actions across the selected members.

Cluster membership references account profile secret IDs. The cluster config
does not store private agent keys; placement captures each member's committed
profile key into a zeroizing task at submission time. Ghost/watch-only accounts
are not eligible for cluster signing.

Key behavior:

- `wallet_cluster_state.rs` owns runtime cluster form state, member snapshots,
  aggregate position summaries, and recent execution legs.
- `wallet_cluster_update.rs` handles create/select/member edits, snapshot
  refresh, websocket updates, order splitting, result classification, and
  orderStatus checks for ambiguous legs.
- `wallet_cluster_views.rs` renders the auxiliary window opened from the add
  widget menu.
- Cluster member streams are generated in
  `subscription_state/user_data.rs` for the selected cluster only.

Cluster close actions require fresh member snapshots and route through the
shared order preparation boundary with `OrderSurface::ClusterClose`.

## Portfolio And Income

Portfolio state lives in `portfolio_state/` and is updated by
`portfolio_update.rs`. It supports:

- portfolio history
- PnL charts
- income snapshots
- portfolio and income panes
- periodic analytics refresh

`account_analytics/` owns HTTP parsing for portfolio history and income data.
The state is read-only analytics; trading actions should not depend on it for
order-critical validation.

## PnL Cards

`pnl_card/` creates exportable PnL card windows/images for a position or
summary target.

Features include:

- display mode
- percent mode
- optional price privacy
- optional position size display
- copy/save image
- contrast and text rendering helpers

PnL cards can include financial values, so privacy toggles and output handling
should be treated carefully.

## Freshness And Refresh

Account data carries freshness information. Close-position, NUKE, and some
automation paths reject stale snapshots and request refresh rather than trading
against outdated positions.

Spot-balance freshness is evaluated separately from positions and open orders.
Unrelated account revisions do not invalidate a spot percentage selection, but
a spot dispatch, fill, balance replacement, account switch, or provider/key
generation change does. This prevents a pre-trade balance snapshot from sizing
another action while exchange holds are still converging.

Refresh state includes:

- `account_loading`
- `account_refresh_followup_pending`
- `account_reconciliation_required`
- `account_error`
- `account_refresh_backoff_until_ms`

If a refresh is requested while another is in flight, the follow-up flag keeps
the second refresh from being dropped.

Connected-account refresh contexts also capture the account-data revision at
dispatch. If a user-data websocket frame advances that revision before the
REST result arrives, the result is not installed or used to settle order
uncertainty; the newer merged websocket state remains visible and one
post-frame refresh is started. This prevents a pre-event snapshot from erasing
an order/fill/balance/position delta or proving an operation absent. During
initial loading, when no base snapshot exists to merge into, an account frame
queues the same post-frame refresh instead of starting a competing request.

## Tests To Check

Use focused tests in these areas:

- `src/account/types/data/tests/**`
- `src/account/data/bootstrap/tests.rs`
- `src/account/data/merge/tests.rs`
- `src/account/data/fees/tests.rs`
- `src/account_update/stream/tests/**`
- `src/account_state/switching/tests.rs`
- `src/account_views/positions/**/tests`
- `src/account_views/summary/**/tests`
- `src/wallet_state/**/tests`
- `src/wallet_update/**/tests`
- `src/wallet_views/**/tests`
- `src/account_analytics/**/tests`
- `src/portfolio_state/**/tests`
- `src/pnl_card/tests/**`

For account-data changes, include tests for stale data, merge behavior,
spot/HIP-3 handling, and websocket repair where relevant.
