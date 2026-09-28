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
| Wallet clusters | `src/wallet_cluster_state.rs`, `src/wallet_cluster_update.rs`, `src/wallet_cluster_update/`, `src/wallet_cluster_views.rs`, `src/wallet_cluster_views/` | Saved groups of trading profiles, aggregate positions, and split order submission. |
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

The Add Account window holds address and key drafts separately from the active
profile. `AddAccountAddressChanged` and `AddAccountKeyChanged` only update that
draft. `AddAccountSubmit` validates it and persists the new profile's credentials
before optionally switching accounts through the normal trading-state guards.
Cancelling drops the draft without changing saved or active credentials.

### Existing Subaccounts

The Add Account window discovers existing Hyperliquid subaccounts through the
public `subAccounts` info request, using the configured Hyperliquid read proxies
when enabled. Enter the parent address, choose **Discover
Subaccounts**, select the child, and supply an agent key approved by the parent
if trading is needed. Each child is saved as its own account profile. Creation
and transfers are outside this workflow.

`wallet_address` remains the effective account address used for positions,
balances, fills, WebSocket subscriptions, portfolio history, and reconciliation.
The optional `master_address` records the parent for a subaccount; legacy and
ordinary account profiles default to `None`. Changing a saved subaccount's
address requires adding or selecting a different profile, so its parent/key
binding cannot be reused accidentally.

`account_state/subaccounts.rs` validates discovery results and redacts message
payloads. The messages `AddAccountDiscoverSubaccounts`,
`AddAccountSubaccountsLoaded`, and `AddAccountTargetSelected` route through the
account update module. Results carry window ID, request generation, and parent
address; stale responses are discarded after edits or window replacement.
Invalidating a selected child requires an explicit new selection before save.

Trading uses a parent-approved API/agent wallet, with the selected child's
address signed into `vaultAddress`. Parent and child addresses are also bound
to the stored credential, including backup/config-loss recovery. HyperDash and
Hydromancer keys remain global data-provider credentials.

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

`account/types/data/completeness.rs` records unique section/message pairs and
builds the overall warning summary from borrowed messages in order of first
occurrence, deduplicating identical text across sections. Section warnings retain
their own messages and fallback text. Marking positions incomplete clears
actionability; marking them degraded preserves the existing actionability flag.

Fetch-scope constructors borrow input, trim whitespace, and fold ASCII case when
building owned DEX keys. Selecting a blank DEX falls back to the default
all-markets scope. Reading a scope borrows DEX names in their stored order and
uses the caller's fallback only for an empty all-markets list.

Hydromancer request orchestration lives in `account/data/bootstrap/hydromancer.rs`.
Its `portfolio.rs` submodule owns single/batch response parsing, the redacted
portfolio model, scoped conversion, native/DEX merging, and batch-size policy.
Batch requests use that shared scope limit and serialize borrowed address slices
while the public fetch task retains the owned list for result assembly.
Joined portfolio and order helpers borrow addresses, scopes, and API keys from
the enclosing task; the task retains its `Zeroizing` key through the requests.
DEX names become owned strings when stored in returned account or wallet data.
Owned response parsers move JSON fields, tuple payloads, and address strings into
their outputs. Primary snake-case fields take precedence over camel-case aliases
whenever present, including null or malformed values. Tests beside the model
cover those distinctions, validation order, metadata selection, and redaction.
Portfolio getters and native bootstrap/wallet conversion deserialize retained
JSON by reference into owned models. Getters preserve independent results on
repeated calls; bootstrap errors can still preview the original redacted JSON.
Wallet snapshot conversion moves parsed positions into aggregation while keeping
equity and withdrawable parsing independent of position-schema errors.
Hydromancer wallet-detail conversion also consumes temporary DEX states and order
vectors, preserving the independently returned native clearinghouse snapshot.

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

Account projection in `account_positions.rs` borrows native positions from the
connected account snapshot and eagerly appends owned outcome and spot rows, in
that order. Table filtering, section lists, summary accumulation, chart overlays,
tab counts, and PnL-card metrics consume these rows without copying native wire
fields. Rendered widgets, action messages, and export snapshots own their outputs.

Spot-pair selection collects borrowed USD-quoted candidates once and stably sorts
them by asset index. It prefers a fill-reconciled pair, then the most recent fill
(the last candidate wins a timestamp tie), then the first live mark, then the
first candidate. Only an owned synthesized position copies the selected key.

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
- spot token and USDC transfers
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

Only the tab selections (`BottomTabConfig::DepositsWithdrawals` and `Transfers`)
are persisted. Existing tab names and the fallback for unknown names remain
compatible. Transfer data and external wallet addresses are neither persisted
nor included in Debug logs.

### Transfers

The **Transfers** tab shows Hyperliquid spot token and USDC transfers, including
sent/received transfers, subaccount USDC transfers, and internal spot/perps USDC
movements. It uses the same account-scoped ledger reader, refresh controls, and
30-second polling as Deposits/Withdrawals. The tabs filter the shared history
before pagination and keep separate page and expanded-row state.

Rows show UTC time, direction, asset, exact decimal amount, sender/recipient
(or the internal account route), and status. Expanded rows expose the full
wallet addresses and ledger transaction with copy controls. Transfer fees use
the reported fee asset when present; older spot records do not infer that asset
from the transferred token. Unit errors are only shown in Deposits/Withdrawals.

The [Hyperliquid ledger schema](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions)
(checked 2026-09-17) defines `spotTransfer`, `internalTransfer`,
`subAccountTransfer`, and `accountClassTransfer`. Direction is relative to the
connected account, with case-insensitive address comparisons. Bridge records
remain in Deposits/Withdrawals; vault flows, rewards, and genesis allocations
are not token transfers. Unit's Hyperliquid-side spot movement may appear in
Transfers while its bridge operation appears in Deposits/Withdrawals.

### API contracts and limitations

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
  Internal, spot, and account-class transfers appear separately in Transfers;
  vault flows are excluded from both tabs.
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

Tracker config restoration normalizes and deduplicates addresses in first-seen
order, taking the current list before legacy wallet entries. Muted addresses use
the same normalization independently, including addresses absent from the tracked
list. Address-book label lists share one collection/sort/deduplication path;
combined local/remote labels are sorted once, then subscription selection filters
muted addresses. Color or tags alone do not create a label subscription.
Config Debug output uses the same count-only redaction as runtime wallet state.

Wallet label and display helpers normalize addresses once and share the lookup
for the first nonblank label, checking the remote book before the local book.
Invalid addresses retain their raw display text and never use a stored label.
Tracker rows borrow loaded row state, reuse remote status, and consume prepared
display strings for label controls and address text.

`wallet_state/tracker/selection.rs` gives queued requests FIFO precedence over
automatic refreshes. Automatic core selection ranks borrowed addresses by age
and copies only the selected batch; order selection copies only the chosen
address. Timestamp ties retain tracked-list order. Core and order loading/retry
checks remain independent, and automatic order refresh requires a core snapshot.
Refresh-all rebuilds the core queue directly from tracked addresses, copying only
eligible, nonduplicate entries and reusing queue capacity.
Tracker tests cover these policies and request-context setup before tasks run.
Adding a wallet and restoring a muted wallet share input clearing, deferred
persistence, and refresh dispatch. Restored rows retain their data and remote
label protection; new rows start from defaults. Subscription refresh conditions
remain specific to each path.

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

Summary and table preparation borrow stored position rows and append owned spot
rows through `wallet_position_details_with_spot`. Spot synthesis remains eager;
the table filters its row buffer in place and caches symbol sort keys, preserving
source order for equal symbols. Positions, orders, and spot balances share the
table container in `wallet_views/style.rs`. Error and warning text borrow their
window snapshot; outgoing messages continue to own their values.

`wallet_state/details.rs` shares window selection and snapshot timestamps across
position, order, balance, and fill stream events. Matching windows clear their
error and record a refresh even while awaiting an initial snapshot; stream
events leave REST loading/context state intact. Each loaded window retains an
independent snapshot. Event-specific hidden-symbol filters, DEX order replacement,
and fill deduplication remain separate. Lag recovery preserves pending requests
and starts refreshes only for idle matching windows. Cluster position freshness
uses its own trading-specific rules.

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
- `wallet_cluster_update.rs` routes messages and holds the feature's shared
  preparation types. Its child modules separate cluster/member editing
  (`management.rs`), snapshot refresh/results and websocket updates (`data.rs`),
  order/close planning and member eligibility (`orders.rs`), dispatch and result
  reconciliation (`execution.rs`), and position aggregation/close sizing
  (`positions.rs`). Focused tests live beside each module.
- `wallet_cluster_views.rs` renders the auxiliary window opened from the add
  widget menu. Its `members.rs`, `ticket.rs`, `positions.rs`, and `executions.rs`
  children own each section. The members section receives the already-selected
  cluster, display text borrows state where possible, and event messages retain
  owned IDs. Disabled close buttons do not copy symbols, and the ticket builds
  the price input only when visible.
- Cluster member streams are generated in
  `subscription_state/user_data.rs` for the selected cluster only.

Cluster close actions require fresh member snapshots and route through the
shared order preparation boundary with `OrderSurface::ClusterClose`.

Order and close plans share `PreparedClusterLeg::new` to bind the prepared
request and completion context to the captured member. Dispatch moves the owned
signing key and context into the task while execution history retains its own
client-order ID. Position aggregation borrows input symbol names and copies a
name only when creating a summary; calculation order and optional totals remain
unchanged.

Full-cluster and single-member refreshes share selection in `data.rs`. They copy
the selected cluster ID and matching profile IDs before updating state, preserving
member order and repeated entries without cloning names, weights, or input drafts.
Read refreshes include zero-weight members and preserve cached snapshots and
position timestamps while loading. Missing profiles remove cached rows; invalid
addresses replace their rows with the existing validation error.

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

Portfolio and income each own an independent `AnalyticsRefreshState` from
`portfolio_state/refresh.rs`. It holds the loading flag, saturating request
counter, and one queued follow-up flag. Completion checks the request ID before
clearing loading state; invalidation advances the counter and clears pending
refresh work. Account matching, income eligibility, result handling, and
follow-up dispatch stay in `portfolio_update.rs`.

Each completion handler applies a result only to the matching connected account,
then uses one follow-up path for both current and previous-account responses.
That path checks the current connection and, for income, its Portfolio Margin
eligibility. Income snapshot application and interest alerts are handled together
in `apply_income_snapshot` after the request and account checks pass.

Income snapshot assembly validates each token's carrying values and annualized
projection before adding it to the totals. Recent payments sort borrowed hourly
entries by descending time and build the first 12 valid rows, keeping input order
for ties. Invalid amounts and aggregate samples do not consume that limit.
Portfolio buckets retain history order and distinguish missing volume from an
invalid supplied value. Portfolio data selection is independent of theme
construction.

The Income pane uses three local views so its small PaneGrid footprint remains
readable: Overview presents realized interest, account health, current carrying
values, and the 12-month projection; Tokens shows annualized per-token
contributions; Payments shows recent hourly interest. Refresh and alert controls
remain available from the pane title bar in every view.

Portfolio daily rows borrow the selected bucket histories, and the performance
chip is prepared only in dollar mode. Income compact and wide tables share their
common cells; payment amounts remain in raw token units while position values
use the display denomination. Income chart layout borrows projection labels and
creates owned text only for visible axis labels and the hovered tooltip.

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

Preview and export use the same `pnl_card_render_text` transformation. Whole-price
and fractional-price privacy rules share one ASCII-digit masking helper, with
their visibility thresholds selected separately. Export requests own snapshots
of the card settings and metrics after checking the card's account binding.

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

## Position Execution Fees

The Positions tab includes a sortable **Spent Fees** column beside uPnL. It
shows net execution fees in USD for the current position's lifetime: opening
fills, increases and partial closes, less maker rebates. A reversal attributes
only the opening fraction of that fill's fee to the new position. Funding and
the existing Total PnL calculation remain separate. The column follows the
responsive table layout and the Hide PnL privacy toggle.

`account/position_fees.rs` walks the account's deduplicated recent fills backward
using their optional `startPosition` metadata, resolving execution order within
a timestamp through position continuity. It requires a chain back to flat or a
reversal that reconciles to the live size. Missing opening history, gaps,
unrecognized fee tokens, invalid values, stale account ownership, incomplete
snapshots or a temporary fill/position mismatch show `—` with a tooltip. A
transferred balance is not assigned fees from an unrelated position. Spot base
token fees are converted at each fill's execution price and deducted from the
inventory used for reconciliation.

The account feed retains up to 2,000 fills; websocket snapshots merge into that
history so a short snapshot cannot evict REST-loaded opening fills. No new
history fetch or persistent account field is introduced. The optional fill
metadata defaults to absent for older/provider payloads and remains redacted in
Debug output. Hyperliquid's reported `fee` already includes any builder fee;
see the [official fill response documentation](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint#retrieve-a-users-fills).
