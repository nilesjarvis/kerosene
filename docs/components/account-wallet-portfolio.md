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
| Combined portfolio | `src/combined_portfolio.rs`, `src/combined_portfolio_update.rs`, `src/combined_portfolio_views.rs` | Watch-only multi-wallet portfolio history, aggregate PnL, and its standalone window. |
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
- deposits and withdrawals (native USDC bridge and Unit spot assets)
- portfolio tab content
- income view

Spot and outcome balances can feed order-entry helpers such as outcome sell
prefill.

### Deposits/Withdrawals

The Positions / History widget's **Deposits/Withdrawals** tab combines the
native Hyperliquid USDC bridge with Unit Protocol operations, including spot
assets deposited or withdrawn through TradeXYZ. TradeXYZ uses Unit for these
transfers; no TradeXYZ credentials or trading agent key are required.

- `account/transfers/` owns the public read clients, exact decimal formatting,
  and the common runtime transfer model.
- `account_state/transfers.rs` owns the account-scoped history, independent
  provider loading/error states, pagination, and expanded row.
- `account_update/transfers.rs` handles `RefreshTransferHistory`,
  `TransferHistoryLoaded`, `TransferHistoryPage`, and `ToggleTransferDetails`.
- `account_views/history_tables/transfers.rs` renders 50 rows per page, newest
  first, with expandable wallet, bridge, fee, confirmation, and transaction
  details. Full addresses and references can be copied through redacted messages.

Opening the tab fetches both sources. A saved selected tab also loads on wallet
connection, and `subscription_state/timers/analytics.rs` polls every 30 seconds
while the tab is selected in an open workspace. A failed source retains its
previous rows with an explicit warning; the other source remains usable.
Connect, disconnect, invalid-address reset, and account switching clear this
runtime data and advance a generation so late replies cannot restore it.
These reads always use their native APIs, independently of the selected market
data provider. Hyperliquid requests honor the existing REST proxy transport.

Only the `BottomTabConfig::DepositsWithdrawals` selection is persisted. Existing
tab names and the fallback for unknown names remain compatible. Transfer data
and external wallet addresses are neither persisted nor included in Debug logs.

#### API contracts and limitations

Verified against primary sources on 2026-09-07:

- [TradeXYZ deposits](https://docs.trade.xyz/getting-started/funding-your-wallet/deposits)
  and [spot](https://docs.trade.xyz/trading/spot) identify Unit as the underlying
  native-chain asset bridge.
- [Unit API](https://docs.hyperunit.xyz/developers/api) and
  [operations](https://docs.hyperunit.xyz/developers/api/operations):
  `GET https://api.hyperunit.xyz/operations/{hyperliquid_account_address}`
  returns all associated operations. No pagination parameter is documented.
  `sourceChain`/`destinationChain` determine direction relative to Hyperliquid;
  `sourceAddress` is the source wallet, while `protocolAddress` is the separate
  Unit bridge address. The API can omit the destination and operation ID for
  newly discovered deposits. A missing address is displayed as **Not provided**;
  the bridge address is never substituted for the sender.
- Unit's amounts and fee estimates use native asset base units. They are
  converted with decimal-string arithmetic, including fractional fee estimates.
  Native decimals come from the operation contract and the
  [official Unit frontend asset registry](https://app.hyperunit.xyz/)
  (`unit.nativeDecimals`, inspected in its published JavaScript bundle).
  They must not be replaced with HyperCore token `weiDecimals`: BTC uses 8
  native decimals versus UBTC's 10; ETH uses 18 versus UETH's 9. Known historical
  assets remain recognized. Future unknown assets remain visible with amounts
  explicitly labeled **base units** rather than guessed token quantities.
- Unit transaction references include suffixes: Bitcoin `txid:vout`, Ethereum
  `hash:trace-id`, Solana `signature:destination-address`, and Hyperliquid
  `sender:nonce`. These are preserved and labeled **references**, not blindly
  linked as transaction hashes. Unknown operation states remain visible.
- [Hyperliquid ledger history](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/perpetuals)
  uses `POST https://api.hyperliquid.xyz/info` with
  `{"type":"userNonFundingLedgerUpdates","user":"<account>","startTime":0,"endTime":<milliseconds>}`.
  Only `deposit` and `withdraw` deltas become native Arbitrum USDC bridge rows.
  Internal, vault, spot, and account-class transfers are excluded, avoiding
  double counting Unit's Hyperliquid-side spot transfers.
- Native history follows the documented inclusive timestamp pagination for
  **all** ledger categories, deduplicating the overlapping boundary. Each fetch
  is bounded to 100 pages / 60 seconds; partial reads carry a warning and a
  continuation cursor. Completed reads subsequently overlap the last minute.
  A full page that cannot advance its timestamp is explicitly incomplete.
- The [ledger wire schema](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions)
  does not provide the external Arbitrum sender/recipient or delivery
  transaction. Native rows therefore expose the ledger hash separately and
  leave the external wallet unavailable. A native withdrawal is **Debited**,
  not claimed to be finalized on Arbitrum; a deposit is **Credited**. Additional
  verified chain indexing would be needed to enrich that missing information.

Focused checks: `cargo test transfers`, the account message-route tests,
config pane serialization tests, layout conversion tests, and analytics timer
tests. Fixtures contain synthetic wallet/transaction values only.

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

Local tracked wallets are persisted, but private trading keys are not part of
wallet tracker state. The optional remote wallet database is a read-only,
runtime-only source: only its URL is persisted. `wallet_state/remote_database.rs`
and `wallet_state/remote_database/api.rs` own the mirror and paginated PocketBase
client; `wallet_update/remote_database.rs` applies complete snapshots, guards
request IDs across endpoint changes, and reconciles remote additions/deletions.
The local address book remains separate; display and tracked-trade subscription
helpers combine the two sources. Remote metadata cannot be edited in the tracker
and is excluded from saved config and label exports. The timer in
`subscription_state/timers/wallet.rs` polls every 30 seconds independently of
tracker visibility, with an immediate boot/save sync. Failures retain the last
snapshot only in memory. See the [README database contract](../../README.md#remote-wallet-label-database).

### Compact Wallet Tracker

The add-widget menu and Alfred offer a Compact Wallet Tracker pane for the main
workspace or a Canvas. It shares the existing tracked wallet list, labels, and
summary snapshots. Its table contains only wallet label, account value,
unrealized PnL, and position bias. Bias uses gross long/short notional: a side
needs at least two thirds of exposure to show Long or Short; mixed exposure
shows Mixed, zero exposure shows Flat, and missing or invalid exposure shows an
unavailable value. Monetary values follow the display denomination and abbreviate
large amounts to fit narrow panes. Stale or degraded snapshots have a warning
marker and tooltip.

Clicking a wallet replaces that pane's list with its perpetual positions,
including HIP-3 symbols. Rows show side, leverage, size, value, and unrealized
PnL; wider panes also show entry price. Back returns to the wallet list. Each
pane selects independently and never opens a detail window or changes the active
trading account. Wallet membership and labels are managed in the existing tracker.

`wallet_state/compact.rs`, `wallet_update/compact.rs`, and
`wallet_views/compact.rs` own the pane's transient selection, request handling,
and views. The existing five-second wallet timer stays active while a compact
pane is open, even with the tracker window closed. It schedules due summary
reads and refreshes selected positions at a one-minute cadence, with manual
refresh available. Failed detail refreshes retain the previous snapshot and
retry after a minute. Unique request IDs reject late replies after navigation,
pane reuse, or layout changes; provider/key invalidation clears cached details
and pending requests. Position rows respect hidden-symbol settings.

Layouts persist `CompactWalletTracker { id }` and its optional padding override.
There is no separate instance config or persisted wallet selection: restored
panes start on the shared wallet list. Imported duplicate IDs are repaired.

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

## Combined Portfolio

Combined Portfolio is an auxiliary watch-only window for viewing several
wallets as one portfolio. Its wallet membership, labels, open state, and window
geometry are persisted independently from Wallet Tracker. Opening or refreshing
the window requests the public portfolio-history endpoint for every member in
parallel.

The combined chart normalizes each wallet's selected cumulative PnL series to
its own period baseline, aligns the series on their union of timestamps, and
carries each wallet's latest sample forward. The headline PnL is the sum of the
individual period changes, while combined account value is the sum of the
latest available account-value samples. Failed wallets stay visible as stale
rows and do not hide successfully loaded results.

This feature never reads or stores agent keys and never changes the active
trading account. Per-wallet Details actions reuse the existing watch-only wallet
detail window.

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

The Income pane uses three local views so its small PaneGrid footprint remains
readable: Overview presents realized interest, account health, current carrying
values, and the 12-month projection; Tokens shows annualized per-token
contributions; Payments shows recent hourly interest. Refresh and alert controls
remain available from the pane title bar in every view.

## PnL Cards

`pnl_card/` creates exportable PnL card windows/images for a position or
summary target.

Features include:

- display mode
- percent mode
- optional price privacy
- optional position size display
- copy/save image
- theme-aware contrast and directional styling
- one aspect-locked canvas renderer shared by the preview and exported PNG, so
  layout and the selected monospace font stay identical

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
