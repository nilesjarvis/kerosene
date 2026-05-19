# Chase Orders

Chase orders are Kerosene's client-side execution helper for working a limit order at the current best bid or ask. They are designed for traders who want to stay passive, keep priority near the top of book, and avoid manually dragging or replacing an order every time the book moves.

> **Risk notice:** Chase orders are automation running from your local Kerosene instance. They are not a native Hyperliquid order type. If Kerosene is closed, disconnected, rate-limited, or unable to sign/cancel/modify, the last resting exchange order may remain open. Always monitor open orders and use small sizes until you understand the behavior.

## When to use a Chase order

Use a Chase order when you want to:

- Place a limit order at the best bid for a buy or best ask for a sell.
- Reprice only when the market moves toward your fill.
- Keep one active resting order instead of manually canceling and replacing it.
- Track partial fills and continue chasing only the remaining size.
- Stop the automation without losing visibility into the underlying exchange order.

Do not treat a Chase order like a market order. It may fill quickly, partially fill, or sit unfilled. It intentionally uses limit orders and will not intentionally cross the spread just to complete execution.

## User-facing behavior

### Starting a Chase order

From the order-entry pane:

1. Connect a wallet and enter a Hyperliquid agent key.
2. Select the symbol you want to trade.
3. Enter a positive order size.
4. Choose reduce-only if applicable for a perpetual order.
5. Click `CHASE BUY <symbol>` or `CHASE SELL <symbol>`.

Kerosene fetches the current order book, chooses the best bid for a buy or the best ask for a sell, then submits a limit order at that price.

You can also adopt an existing resting order into the Chase lifecycle from order actions. In that case Kerosene tracks the existing order id, side, size, price, and reduce-only metadata, then begins repricing from that current order.

### What you will see

Active Chase orders appear in the Advanced Orders pane. Each row shows:

- Side: `BUY` or `SELL`.
- Symbol.
- Remaining size and current chase price.
- Current order id when available.
- Reprice count.
- `RO` when the order is reduce-only.
- Status: `Starting`, `Placing`, `Resting`, `Queued`, `Modifying`, `Checking`, `Canceling`, or `Stopping`.

The order-entry pane also shows the currently selected Chase order and exposes a `Stop Chase` button. The Advanced Orders pane exposes per-order `Stop` controls and `Stop All` when any Chase order is active.

### Repricing behavior

Kerosene maintains a single resting exchange limit order per Chase order.

- A Chase buy starts at the best bid and only reprices upward when the best bid moves above the current order price.
- A Chase sell starts at the best ask and only reprices downward when the best ask moves below the current order price.
- If the market moves away from the order, Kerosene leaves the existing order in place instead of moving it farther from the fill.
- Reprices are throttled to avoid excessive exchange requests.
- If a reprice is needed while throttled or while another exchange request is in flight, Kerosene stores the latest observed best price and handles it later.

This means the Chase order is aggressive in the direction of getting filled, but conservative about moving away from the current working price.

### Partial fills and completion

Kerosene watches user open-order and fill updates.

- If the exchange reports a smaller remaining size for the chased order, Kerosene updates the Chase order's remaining size.
- If the order disappears from open orders, Kerosene requests an account refresh before deciding it is done. This avoids stopping a Chase order just because a stale WebSocket snapshot temporarily omitted the order.
- If the refreshed account data confirms the order is no longer open, Kerosene ends the Chase order and shows a fill summary when matching fill data is available.
- If the place or modify response reports the order fully filled immediately, Kerosene removes the Chase order from the active list.

### Stopping a Chase order

Stopping a Chase order stops the automation and attempts to cancel the current resting exchange order.

Expected stop behavior:

- If no exchange request is in flight, Kerosene sends a cancel request for the current order id.
- If a place, modify, or cancel request is already in flight, Kerosene waits for that result before taking the next safe action.
- If a stop is requested while an initial placement later comes back as resting, Kerosene immediately sends a cancel for that newly placed order.
- After cancel success, Kerosene removes the Chase order and refreshes account data.

If cancel status cannot be confirmed, Kerosene retries status checks/cancel handling up to the configured retry limit, then stops tracking and warns you to check open orders manually.

## Limits and safety rails

Current built-in limits:

- Maximum active advanced orders: `8` total across Chase and TWAP.
- Minimum per-order reprice interval: `1s`.
- Minimum global interval between Chase exchange requests: `250ms`.
- Rate-limit cooldown after a detected exchange rate-limit response: `5s`.
- Maximum reprice count: `1,000`.
- Maximum Chase lifecycle duration: `15 minutes`.
- Maximum allowed drift from initial Chase price: `5%`.
- Maximum cancel confirmation retries: `5`.

Kerosene also stops or refuses a Chase order when:

- The wallet is not connected or the agent key is missing.
- The selected ticker is muted in risk settings.
- The symbol cannot be resolved.
- The order size or price is invalid.
- Outcome markets are selected; Chase trading is disabled for those markets.
- A perpetual resting order lacks reliable reduce-only metadata.
- The connected account changes after the Chase starts.
- The exchange reports a terminal place/modify/cancel condition that requires reconciliation.

## Technical model

### Lifecycle overview

```
                         CHASE ORDER LIFECYCLE
                         =====================

ORDER ENTRY
+--------------------------------------------------------------------+
| User enters size and clicks CHASE BUY / CHASE SELL                  |
+-------------------------------+------------------------------------+
                                |
                                v
START VALIDATION
+--------------------------------------------------------------------+
| Required before local state is created:                             |
| - wallet connected + agent key captured                             |
| - no pending order action                                           |
| - < 8 active advanced orders total (Chase + TWAP)                   |
| - valid size, symbol, precision, and USD reference price if needed  |
| - symbol is not hidden by risk settings                             |
| - selected market supports Chase trading; outcome markets do not    |
|                                                                    |
| Any failure -> reject with user-visible status/toast; no Chase.      |
+-------------------------------+------------------------------------+
                                |
                                v
CREATE LOCAL ChaseOrder
+--------------------------------------------------------------------+
| Stored immediately after validation:                                |
| - id, coin, side, target_size, remaining_size, account, agent key   |
| - current_oid = None                                                |
| - current_price = 0, current_price_wire = "", initial_price = 0     |
| - pending_op = None, pending_best_price = None                      |
| - pending_order_action = ChaseBuy / ChaseSell                       |
+-------------------------------+------------------------------------+
                                |
                                v
FETCH INITIAL BOOK
+--------------------------------------------------------------------+
| Fetch symbol-aware order book -> choose best bid for buy,           |
| best ask for sell.                                                  |
|                                                                    |
| Fetch error or no usable best price -> remove Chase and stop.        |
+-------------------------------+------------------------------------+
                                |
                                v
PLACE INITIAL LIMIT ORDER
+--------------------------------------------------------------------+
| Before sending:                                                     |
| - verify account still matches captured account                     |
| - round best price for Hyperliquid wire format                      |
| - quantize residual size                                           |
| - set current_price/current_price_wire and initial_price            |
| - set pending_op = Place                                            |
|                                                                    |
| If global request gate is busy: queue pending_best_price and wait    |
| for the Chase reprice tick to call place_at_best later.              |
+-------------------------------+------------------------------------+
                                |
                                v
place_order(limit)
                                |
              +-----------------+------------------+
              |                 |                  |
              v                 v                  v
        error/unknown       full fill        resting response
              |                 |                  |
              |                 |                  v
              |                 |        +-------------------------+
              |                 |        | record oid              |
              |                 |        | current_oid = oid       |
              |                 |        | pending_op = None       |
              |                 |        | oid_confirmed = false   |
              |                 |        +-----------+-------------+
              |                 |                    |
              v                 v                    v
        archive/refresh    archive fill       ACTIVE, UNCONFIRMED
                                               until open-orders
                                               snapshot confirms oid


ACTIVE RECONCILIATION
+--------------------------------------------------------------------+
| Account open-order snapshots and refreshes update the active Chase: |
| - matching oid confirms the order and syncs remaining size/price    |
| - matching fills update filled_size                                 |
| - target fully filled -> archive with fill summary                  |
| - size too large after fills -> queue a size-correction modify      |
| - confirmed oid missing from WS -> request REST account refresh     |
+-------------------------------+------------------------------------+
                                |
                                v
BOOK-DRIVEN REPRICE CANDIDATE
+--------------------------------------------------------------------+
| WebSocket book update -> best bid/ask                               |
|                                                                    |
| Ignore/no-op when:                                                  |
| - wrong coin, hidden symbol, stop requested, pending op in flight   |
| - no current oid, same rounded wire price, or price moves away      |
| - current oid has not been confirmed yet                            |
|                                                                    |
| Queue pending_best_price when throttled or waiting for confirmation. |
+-------------------------------+------------------------------------+
                                |
                                v
REPRICE GUARDS PASS
+--------------------------------------------------------------------+
| Check lifecycle limits: valid price, max duration, max reprices,    |
| and max drift from initial price.                                   |
|                                                                    |
| If a limit is reached -> stop Chase and cancel current oid if any.  |
+-------------------------------+------------------------------------+
                                |
                                v
VERIFY BEFORE MODIFYING
+--------------------------------------------------------------------+
| Kerosene does not immediately modify on the book tick. It first:    |
| - stores pending_best_price                                         |
| - marks missing_open_order_refresh_requested                        |
| - refreshes account data to verify fills and open orders            |
+-------------------------------+------------------------------------+
                                |
                                v
ACCOUNT REFRESH RESULT
+--------------------------------------------------------------------+
| Open order still exists:                                            |
|   if pending price or size correction remains -> modify same oid     |
|   otherwise continue resting                                        |
|                                                                    |
| Open order missing and pending price exists:                        |
|   assume old oid is gone after complete refresh, clear current_oid,  |
|   place residual size at pending best price                         |
|                                                                    |
| Open order missing and no pending price:                            |
|   archive as filled/ended using fill history when available          |
|                                                                    |
| Incomplete open-orders/fills refresh:                               |
|   pause instead of modifying or placing replacement                  |
+-------------------------------+------------------------------------+
                                |
                                v
MODIFY CURRENT ORDER
+--------------------------------------------------------------------+
| Send modify_order(current_oid, pending_best_price or current_price, |
| residual_size).                                                     |
|                                                                    |
| While sending:                                                      |
| - pending_op = Modify { oid }                                       |
| - last_reprice_at = now                                             |
| - reprice_count++                                                   |
+-------------------------------+------------------------------------+
                                |
                                v
MODIFY RESULT
+--------------------------------------------------------------------+
| full fill:                                                          |
|   record fill, archive Chase, refresh account                       |
|                                                                    |
| success/resting:                                                    |
|   update expected price, clear pending op, set oid_confirmed=false, |
|   request account refresh before chasing again                      |
|                                                                    |
| retryable/rate-limit error:                                         |
|   clear pending op, keep pending target queued, apply cooldown       |
|                                                                    |
| terminal-looking or unknown result:                                 |
|   check status with account refresh before deciding                  |
|                                                                    |
| non-retryable modify error:                                         |
|   stop Chase and cancel current oid if possible                     |
+--------------------------------------------------------------------+


STOP FLOW (user stop, Stop All, wallet/account change, or limits)
+--------------------------------------------------------------------+
| Set stop_requested = true and remember stop_reason.                 |
+-------------------------------+------------------------------------+
                                |
        +-----------------------+------------------------+
        |                       |                        |
        v                       v                        v
 pending Place            pending Modify           pending Cancel
        |                       |                        |
        v                       v                        v
 wait for result       wait for modify result      wait for cancel result
        |                       |                        |
        |                       v                        |
        |              if still has current_oid,          |
        |              send cancel_order                  |
        |                                                |
        v                                                v
 if late resting oid appears, cancel it          cancel result handling

No pending op:
  - current_oid exists -> pending_op = Cancel { oid }, send cancel_order
  - no current_oid     -> archive/clear immediately

Cancel result handling:
  - success -> archive/remove Chase and refresh account
  - filled/canceled/not found/unknown -> check order status via refresh
  - other cancel error -> clear pending op, warn, count retry
  - after 5 cancel failures/unknowns -> archive with "check open orders"


LIMITS AND GATES
+----------------------------+---------------------------------------+
| Max active advanced orders | 8 total across Chase + TWAP           |
| Per-order reprice throttle | 1s/order                              |
| Global exchange interval   | 250ms                                 |
| Rate-limit cooldown        | 5s                                    |
| Max reprices               | 1,000                                 |
| Max duration               | 15 min                                |
| Max price drift            | 5% from initial Chase price           |
| Max cancel retries         | 5                                     |
+----------------------------+---------------------------------------+
```

### State

The core state lives in `ChaseOrder` (`src/signing/model.rs`). It stores:

- `id`: local Chase id.
- `coin`, `asset`, `sz_decimals`, `is_spot`: symbol and exchange metadata.
- `account_address`: connected wallet when the Chase was started or adopted.
- `agent_key`: captured agent key used for lifecycle requests. Debug output redacts it.
- `is_buy`, `reduce_only`, `remaining_size`: execution settings.
- `current_oid`: exchange order id of the current resting order.
- `current_price` and `current_price_wire`: rounded price Kerosene expects the exchange to have.
- `initial_price`, `started_at`, `reprice_count`: lifecycle limit tracking.
- `pending_op`: in-flight exchange request (`Place`, `Modify`, or `Cancel`).
- `last_reprice_at` and `pending_best_price`: throttle and queue state.
- `stop_requested` and `stop_reason`: safe stop coordination.
- `oid_confirmed` and `missing_open_order_refresh_requested`: open-order reconciliation guards.

`TradingTerminal` owns the active map of Chase orders and exposes a selected Chase order for the order-entry UI.

### Start flow

The start flow is implemented in `src/order_execution/chase.rs` and `src/order_execution/chase/lifecycle.rs`.

1. Validate prerequisites: active account, captured agent key, no incompatible pending order action, valid quantity, known symbol, supported market, risk settings, and active advanced-order count.
2. Create a `ChaseOrder` with no `current_oid` and a `Place` not yet sent.
3. Fetch the current book using symbol-aware significant figures.
4. Choose the best bid for buy or best ask for sell.
5. Round the price for Hyperliquid wire format.
6. Submit a normal limit order through `place_order`.
7. Store the returned order id if resting, or end the Chase immediately if fully filled.

Adopting an existing resting order skips the initial book fetch and place request. `handle_chase_resting_order` creates `ChaseOrder` state around the existing order id and marks it confirmed.

### Book-driven repricing

Order book updates call `handle_chase_book_update`, which extracts the relevant best price and passes it into `chase_reprice_to_best_price`.

Before sending a modify request, Kerosene checks:

- The Chase still belongs to the currently connected account.
- No stop or exchange operation is already pending.
- The current order has an id.
- The next price is finite, positive, rounded, and different from the current wire price.
- The price moves toward fill (`buy: next > current`, `sell: next < current`).
- Per-order and global throttles permit another exchange request.
- Lifecycle limits have not been reached.

When checks pass, Kerosene stores `pending_best_price` and refreshes account data before modifying. During reconciliation, it sends `modify_order` for the same order id only if the order is still open and the pending price or size correction remains.

If a complete refresh shows the old order is gone while a pending price exists, Kerosene clears `current_oid` and places the residual size at the pending best price. If checks are temporarily blocked by throttling or an unconfirmed oid, it stores `pending_best_price`; `handle_chase_reprice_tick` later drains that queued price when allowed. `reprice_count` increments when the modify request is actually started.

### Result handling and reconciliation

Exchange result handling is split under `src/order_update/chase/`:

- `result.rs`: placement results and stopped-while-placing cleanup.
- `modify.rs`: modify results, terminal modify errors, and rate-limit cooldowns.
- `cancel.rs`: cancel results, cancel retries, and uncertain cancel handling.
- `resting.rs`: adopting existing resting orders into the Chase lifecycle.

Account WebSocket and REST refresh reconciliation happens in `src/account_update/stream.rs`:

- Open-order snapshots update `remaining_size`, current price, and confirmation state for the chased order id.
- A missing confirmed open order triggers one account refresh before Kerosene concludes the order is gone.
- A complete account refresh with no matching open order ends the Chase order and uses fill history to show a fill summary when possible.

### Why the agent key is captured

The Chase lifecycle stores a zeroizing copy of the agent key at start/adoption. This prevents later edits in the UI key field, wallet switching, or profile changes from accidentally signing a live Chase lifecycle request with the wrong identity. Lifecycle methods also verify the connected account still matches `account_address` before placement or repricing; if it does not, Kerosene stops or removes the Chase order instead of continuing blindly.

## Operational notes

- A Chase order depends on Kerosene staying open and connected.
- Stopping Kerosene does not magically cancel any resting exchange order.
- The visible status is best-effort based on exchange responses, WebSocket updates, and account refreshes.
- If Kerosene warns that status is uncertain or cancel confirmation failed, check Hyperliquid open orders directly.
- Rate limits or network instability can delay repricing or stopping.
- Chase orders are useful for execution convenience, not a substitute for risk management.
