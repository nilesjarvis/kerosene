# TWAP Orders

TWAP orders are Kerosene's client-side execution helper for splitting one target order into smaller, scheduled IOC slices. They are designed for traders who want to reduce timing impact, keep execution inside an explicit price range, and monitor long-running advanced orders without keeping a chart or order-book widget focused on the same symbol.

> **Risk notice:** TWAP orders are automation running from your local Kerosene instance. They are not a native Hyperliquid order type. If Kerosene is closed, no server-side TWAP continues running. If Kerosene is connected but market data, rate limits, or exchange status become uncertain, Kerosene pauses, retries, and reconciles before sending more slices. Completed/stopped/error history is persisted, but live TWAP orders are not resumed after restart.

## When to use a TWAP order

Use a TWAP order when you want to:

- Split a larger buy or sell into smaller child orders.
- Execute over a fixed time window instead of all at once.
- Keep each child order inside a hard min/max price range.
- Randomize slice sizes so child orders are less uniform.
- Let the order continue in the background while switching charts or widgets.
- Review the filled slices, skipped slices, and event log after completion.

Do not treat a TWAP as guaranteed execution. A slice may fill, partially fill, or skip when there is not enough resting liquidity inside the configured range.

## User-facing behavior

### Starting a TWAP

From the order-entry pane:

1. Connect a wallet and enter a Hyperliquid agent key.
2. Select the symbol you want to trade.
3. Enter the total target size in coin or USD terms.
4. Set the duration and slice count.
5. Set min and max price bounds, or let Kerosene derive defaults from the current mid and market-slippage setting.
6. Choose whether to randomize slice sizes.
7. Choose reduce-only if applicable for a perpetual order.
8. Click `TWAP BUY <symbol>` or `TWAP SELL <symbol>`.

Kerosene captures the active account, agent key, symbol metadata, size, price range, reduce-only flag, schedule, and randomization setting. After the TWAP starts, it no longer depends on the active chart or order-book widget staying on that symbol.

### What you will see

Active TWAP orders appear in the Advanced Orders pane. Each row shows:

- Side: `BUY` or `SELL`.
- Symbol.
- Filled size and target size.
- Sent slices and total configured slices.
- Price range.
- Status: `Waiting`, `Running`, `Paused`, `Stopping`, `Done`, `Partial`, `Stopped`, or `Error`.
- A spinning gear while the order is active.
- `Info` for the detail window.
- `Stop` when the TWAP is still active.

The details window shows summary metrics, pause/retry state, child slices, cloids/order ids, event logs, and operating notes.

### Historical records

Completed, partially completed, stopped, and errored TWAP orders are copied into Advanced Orders history. History is saved in the normal Kerosene config and survives app restart.

The persisted history includes:

- Side, symbol, account, status, and final summary.
- Target, filled, remaining, average fill, and price range.
- Duration metadata and completion timestamp.
- Child slice records with planned size, limit price, fill size, average fill price, order id, status, and exchange summary.
- Event logs such as start, placement, fill, skip, stop, completion, and error messages.

Live TWAP state is intentionally not persisted. If Kerosene is closed mid-TWAP, the historical list may contain the latest archived terminal records, but Kerosene will not restart unfinished TWAPs on boot.

## Execution model

### Marketable IOC limit slices

Each TWAP child is submitted as an IOC limit order, not a resting GTC order.

Kerosene first reads the latest TWAP-specific order book subscription, then walks the relevant side of book:

- Buy slices use asks.
- Sell slices use bids.
- The selected child limit price is the worst visible level needed to fill the planned slice size.
- If the visible book cannot fill the full planned size inside min/max bounds, the slice is skipped.

This gives each slice market-order-like immediacy while still enforcing the user's price range. It also keeps child orders from intentionally resting after the slice attempt. If the exchange unexpectedly returns a resting child order id, Kerosene immediately attempts to cancel it.

### Directional price safety

Hyperliquid prices must be rounded for wire format. TWAP uses directional IOC rounding to avoid accidentally making a marketable child non-marketable:

- For buys, Kerosene avoids rounding below the selected ask.
- For sells, Kerosene avoids rounding above the selected bid.
- If the rounded/selected IOC price cannot remain inside the configured price range, the slice is skipped.

This reduces the chance of an avoidable no-fill caused by client-side rounding.

### No-match IOC handling

The exchange can return:

`Order could not immediately match against any resting orders`

This happens when the local book showed matchable liquidity, but that liquidity disappeared before the IOC reached the exchange. Kerosene treats this as an ordinary no-fill slice event and continues the TWAP schedule. It is not treated as a terminal error.

The TWAP book snapshot must be fresh. Current TWAP slices reject book data older than `2s` to reduce stale-book races. Stale market data pauses the TWAP instead of consuming a slice; execution resumes when fresh book data arrives or the deadline ends the TWAP.

### Sizing and minimum notional

Kerosene validates TWAP sizing before start and again before each slice:

- Total size must be positive and finite.
- USD-denominated TWAPs require a fresh finite positive mid price to convert USD to base size.
- The configured slice count must be valid.
- The schedule interval must be at least `5s` between planned slices.
- Each planned child must satisfy Hyperliquid's minimum order notional after asset precision quantization.
- Child size is floored to the asset's `sz_decimals` precision and never rounded up.

This prevents low-precision assets from passing validation with a theoretical child size that would later round down below the exchange minimum.

### Randomized slices

When randomization is enabled, Kerosene varies each planned slice size around the remaining average size. The implementation keeps the total bounded by the original target and carries skipped/unfilled size into later slices.

Randomization is local and deterministic per TWAP id/start seed; it is meant to avoid perfectly uniform child sizing, not to hide execution intent.

### Scheduling and throttling

TWAP scheduling is driven by a periodic app tick while at least one non-terminal TWAP is active and not waiting on an in-flight exchange operation.

Current schedule limits:

- Maximum active advanced orders: `8`.
- Maximum TWAP slices per order: `100`.
- Minimum TWAP duration: `60s`.
- Maximum TWAP duration: `24h`.
- Minimum interval between planned TWAP slices: `5s`.
- Maximum aggregate TWAP slice rate: `1 slice/sec`.
- Minimum global interval between advanced-order exchange requests: `250ms`.

The aggregate slice-rate cap prevents multiple active TWAPs from scheduling more slices than the local executor can process cleanly. When several TWAPs are due, Kerosene services the earliest due slice first.

### Pauses, retries, and reconciliation

Each child order receives a Hyperliquid client order id (`cloid`). Kerosene uses this cloid to check order status after ambiguous placement results and to cancel an unexpected resting child when no exchange order id is available.

Retryable failures such as rate limits pause the TWAP with bounded exponential backoff. Terminal failures such as invalid signatures, insufficient margin, reduce-only rejection, invalid tick/notional, or disabled assets stop the TWAP in `Error`.

Ambiguous placement or transport results pause the TWAP and query order status by cloid before another slice can be sent. This avoids accidental double execution.

## Technical model

### State

Core TWAP state lives in `src/twap_state.rs`.

Important types:

- `TwapOrderForm`: editable UI inputs for duration, slices, min/max, and randomization.
- `TwapOrder`: live client-side order state.
- `TwapStatus`: `Running`, `WaitingForMarket`, `Stopping`, `Stopped`, `Completed`, `CompletedPartial`, or `Error`.
- `TwapChildOrder`: one attempted child slice.
- `TwapEvent`: chronological event log for the details page and persisted history.
- `TwapPendingOp`: in-flight place or unexpected-resting cancel operation.
- `TwapBookSnapshot`: latest book and local freshness timestamp.

`TradingTerminal` owns active TWAP orders in `twap_orders`, tracks `selected_twap_id`, and stores persisted terminal history in `advanced_order_history`.

### Start flow

The start flow is implemented in `src/order_execution/twap.rs`.

1. Validate maximum active advanced orders.
2. Require connected wallet and captured agent key.
3. Reject muted tickers and unsupported outcome markets.
4. Resolve symbol metadata: asset id, precision, spot/perp type, display symbol.
5. Parse duration, slice count, quantity, and price range.
6. Validate schedule density against active TWAPs.
7. Convert USD size using a fresh mid price when needed.
8. Validate target size and quantized child notional.
9. Create a `TwapOrder` with `WaitingForMarket` status.
10. Start symbol-specific book subscriptions through the app subscription layer.

### Book update and slice flow

`handle_twap_book_update` stores the latest book snapshot for the TWAP id and symbol. `handle_twap_tick` selects one due TWAP and calls `execute_due_twap_slice`.

Slice execution:

1. Expire the TWAP if the deadline has passed.
2. Verify the connected account still matches the account captured at start.
3. Require a fresh book snapshot.
4. Respect the global advanced-order exchange throttle.
5. Compute and quantize the next planned slice size.
6. Walk the book inside min/max bounds.
7. Directionally round/select a marketable IOC limit price.
8. Re-check minimum child notional.
9. Mark a pending place operation.
10. Submit `place_order(..., OrderKind::LimitIoc, reduce_only)`.

### Result handling

`handle_twap_slice_result` handles the exchange response:

- Exchange transport failure: mark child `Unknown`, pause the TWAP, query order status by cloid, and force account reconciliation.
- Explicit IOC no-match: mark child `No fill`, log a non-error event, and continue later.
- Exchange error: classify as retryable, terminal, or ordinary rejected/no-fill before deciding whether to retry, stop, or consume the slice.
- Filled response with parseable size: mark child filled, update `filled_size` and `remaining_size`.
- Filled response without parseable size: mark status unknown, stop the TWAP in `Error`, and force reconciliation.
- Unexpected resting order id or cloid: mark child `Resting`, send cancel, retry uncertain cancels up to a bounded limit, and reconcile afterward.
- No fill/no oid: mark child `No fill` and continue the schedule.

Successful non-terminal slices do not trigger a full account refresh every time. Account refresh is reserved for terminal completion or ambiguous/unknown outcomes, reducing API pressure.

### Account reconciliation

TWAP reconciliation runs from account refreshes and user fill updates. Kerosene matches account fills against child order ids and updates filled size, average price, child status, and terminal status when possible.

Important behavior:

- Unknown child status can be repaired by later account fills.
- If an unknown child proves partially filled, the TWAP becomes `CompletedPartial`.
- If account fills prove the target is complete, the TWAP becomes `Completed`.
- Ambiguous exchange outcomes pause future slices until reconciliation, avoiding accidental over-execution.

### Stopping

Stopping a TWAP prevents future slices.

- If no exchange request is in flight, Kerosene marks the TWAP stopped immediately.
- If a child place/cancel is in flight, Kerosene marks `Stopping` and waits for the in-flight result.
- If an unexpected child order rested, Kerosene attempts to cancel it using the captured agent key.
- Disconnecting or switching wallets stops active TWAPs tied to the old account.

Stopping a TWAP does not cancel already-filled slices. It only prevents future slices and handles any in-flight child order safely.

## Persistence model

Live TWAP orders are process-local. They contain `Instant`, captured key material, live pending operations, and websocket-derived state, so Kerosene does not serialize or resume them.

Terminal TWAP snapshots are copied into `AdvancedOrderHistoryEntry` in `src/advanced_order_history.rs`. The config snapshot persists `advanced_order_history`, and boot loads/prunes the saved records back into memory.

History records are bounded to avoid unbounded config growth:

- Maximum saved advanced order history entries: `100`.
- Maximum saved event logs per entry: `200`.
- Maximum saved child records per entry: `200`.

## Operational notes

- Keep Kerosene open and connected while a TWAP is running.
- Use ranges that reflect the worst price you are actually willing to accept.
- Thin books, HIP-3 assets, and tight ranges will naturally produce more skipped/no-fill slices.
- A no-fill slice is not necessarily a problem; it often means the book moved or there was insufficient range-bounded liquidity.
- If a TWAP enters `Error` because status is unknown, review the details window and account fills before manually restarting.
- The history page is an audit aid, not a guarantee that all exchange-side activity was captured after app shutdown.
