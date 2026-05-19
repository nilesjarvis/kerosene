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

- Maximum active Chase orders: `8`.
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

 ┌─────────────────────────────────────────────────────────────────────┐
 │                        ORDER ENTRY PANE                             │
 │  User: enters size → clicks CHASE BUY / CHASE SELL                  │
 └────────────────────────────────┬────────────────────────────────────┘
                                  │
                                  ▼
 ┌─────────────────────────────────────────────────────────────────────┐
 │                     START VALIDATION                                 │
 │  ┌─────────────────────────┐  ┌────────────────────────────┐        │
 │  │  Check prerequisites:   │  │  • Wallet connected        │        │
 │  │  • Agent key captured   │  │  • Valid size + symbol     │        │
 │  │  • < 8 active chases    │  │  • Not risk-muted          │        │
 │  │  • No pending ops       │  │  • Not outcome market      │        │
 │  └───────────┬─────────────┘  └────────────────────────────┘        │
 │              │ FAIL → reject with toast                              │
 └──────────────┼──────────────────────────────────────────────────────┘
                │ PASS
                ▼
 ┌────────────────────────────────┐
 │  CREATE ChaseOrder struct      │
 │  • id, coin, price, size       │
 │  • current_oid = None         │
 │  • pending_op = Place         │
 └────────────┬───────────────────┘
              │
              ▼
 ┌────────────────────────────────┐
 │  FETCH ORDER BOOK (best bid/   │
 │  ask) → round price for wire   │
 │  format                        │
 └────────────┬───────────────────┘
              │
              ▼
         ╔══════════════╗
         ║ STATUS:      ║
         ║ STARTING →   ║
         ║  PLACING     ║
         ╚══════╤═══════╝
                │
                ▼
 ┌────────────────────────────────────────┐
 │  SUBMIT place_order() via Hyperliquid  │
 │  REST API                              │
 └────────────┬───────────────────────────┘
              │
       ┌────────┴────────┐
       │                 │
       ▼                 ▼
  FULL FILL         PARTIAL / RESTING
       │                 │
       │                 ▼
       │     ┌────────────────────────────┐
       │     │  Store current_oid         │
       │     │  CHASE ORDER IS NOW ACTIVE │
       │     │                            │
       │     │  ╔══════════════╗          │
       │     │  ║ STATUS:      ║          │
       │     │  ║ RESTING      ║          │
       │     │  ╚══════════════╝          │
       │     └────────┬───────────────────┘
       │              │
       ▼              ▼
       │     ┌─────────────────────────────────────────┐
       │     │  ENTER BOOK-DRIVEN REPRICE LOOP         │
       │     │                                        │
       │     │  WebSocket → ChaseBookUpdate            │
       │     │       │                                 │
       │     │       ▼                                 │
       │     │  ┌──────────────────────────┐           │
       │     │  │ Reprice conditions met?  │           │
       │     │  │                          │           │
       │     │  │ BUY: best bid > price    │           │
       │     │  │ SELL: best ask < price   │           │
       │     │  │ + throttle OK            │           │
       │     │  │ + no pending op          │           │
       │     │  │ + lifecycle limits OK    │           │
       │     │  └────────┬─────────────────┘           │
       │     │           │ NO                          │
       │     │    ┌──────┴──────┐                      │
       │     │    │ Stay        │◄───┘                 │
       │     │    │ RESTING/QUEUED │                    │
       │     │    └──────────────┘                     │
       │     │           │ YES                         │
       │     │           ▼                             │
       │     │  ╔═════════════════╗                    │
       │     │  ║ STATUS:         ║                    │
       │     │  ║ MODIFYING       ║                    │
       │     │  ╚════════════════╝                    │
       │     │           │                             │
       │     │           ▼                             │
       │     │  modify_order(oid, new_price)           │
       │     │  reprice_count++                        │
       │     │           │                             │
       │     │           ▼                             │
       │     │  ┌──────────────────┐                   │
       │     │  │ ModifyResult:    │                   │
       │     │  └────────┬─────────┘                   │
       │     │           │                             │
       │     │     ┌─────┼────────┬──────┬───┐        │
       │     │     │ OK  │ Retry │Rate- │FULL│        │
       │     │     │     │       │limit │FILL│        │
       │     │     │     │       │(5s)  │   │        │
       │     │     │     │       │     │   │        │
       │     │  ┌──┴─┐   │     ┌─┴─┐  │   │        │
       │     │  │◄───┤   │     │   │  ▼   ▼        │
       │     │  └────┘   │     │   │ END  END      │
       │     │           │     │   │ + log+ hist    │
       │     │           ▼     │   │ fill &         │
       │     │  ╔═════════╗    │   │ move to        │
       │     │  ║ STATUS: ║    │   │ advanced       │
       │     │  ║CHECKING ║    │   │ history        │
       │     │  ╚════════╝     │                │
       │     │        │        │                │
       │     │        ▼        │                │
       │     │  ┌──────┐       │                │
       │     │  │ REST  │       │                │
       │     │  └───┬───┘       │                │
       │     │      │           │                │
       │     │      ▼           │                │
       │     │  Open order disappears from WS?    │
       │     │      │                      │     │
       │     │      ▼                      │     │
       │     │  ╔════════════╗             │     │
       │     │  ║ STATUS:    ║             │     │
       │     │  ║ CHECKING   ║             │     │
       │     │  ╚════════════╝             │     │
       │     │        │                    │     │
       │     │   REST account refresh       │     │
       │     │        │                    │     │
       │     │     confirmed gone?          │     │
       │     │  ┌──────┴───────┐            │     │
       │     │  │ YES  │  NO   │            │     │
       │     │       │        │             │     │
       │     │       ▼        ▼             │     │
       │     │    END  back to               │     │
       │     │  + log  RESTING               │     │
       │     │  fill  (was stale)            │     │
       │     │  summary                      │     │
       │     └───────────────────────────────┘     │
       │                                           │
       │     MOVE to advanced_order_history        │
       │
       ▼
  ┌──────────────────┐
  │   END + LOG      │
  │   FILL SUMMARY   │
  └──────────────────┘


 ═══════════════════════════════════════════════════════════════════
                          STOP FLOW (any time)
 ═══════════════════════════════════════════════════════════════════

 ┌─────────────────────────────────────────────────────────┐
 │  User clicks STOP / STOP ALL                              │
 │  OR lifecycle limit reached (15min, 1000 reprices,       │
 │  5% drift, wallet disconnect, account change)             │
 └──────┬─────────────────────────────────────────────────┘
        │
        ▼
 ╔═════════════════════╗
 ║ STATUS: STOPPING    ║
 ╚═══════╤════════════╝
         │
         ▼
 ┌──────────────────────────────┐
 │  stop_requested = true       │
 │                              │
 │  Operation in flight?        │
 │  ┌───────┬───────┬──────────┐│
 │  │ Place │ Modify│   None   ││
 │  └───┬───┴───┬───┘         ││
 │      │       │             ││
 │      │       │      ┌───┐  ││
 │      ▼       ▼      ▼   ▼  ││
 │    Wait for  │  cancel_order││
 │    op result │  (oid)      ││
 │    then ────┘      │       ││
 │    cancel           ▼       ││
 │                 ┌───┴──┐    ││
 │                 │ OK   │    ││
 │                 └──┬───┘    ││
 │                    │        ││
 │                    ▼        ││
 │              Cancel failed? ││
 │           ┌────────┬───────┘││
 │           │ YES    │  NO    ││
 │           └──┬─────┴──┬────┘││
 │              │        │     ││
 │              ▼        ▼     ││
 │           Retry      END   ││
 │           (<5x, warn) +    ││
 │           move to hist     ││
 │                    │       ││
 │                    ▼       ││
 └───────── MOVE to history ──┘


 ════════════════════════════════════════════════════════════
                   DURATION LIMITS (auto-stop)
 ════════════════════════════════════════════════════════════

 ┌───────────────────────┬──────────┐
 │ Max active chases     │ 8        │
 │ Reprice throttle      │ 1s/order │
 │ Global interval       │ 250ms    │
 │ Rate-limit cooldown   │ 5s       │
 │ Max reprices          │ 1,000    │
 │ Max duration          │ 15 min   │
 │ Max price drift       │ 5%       │
 │ Max cancel retries    │ 5        │
 └───────────────────────┴──────────┘
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

1. Validate prerequisites: active account, captured agent key, no incompatible pending order action, valid quantity, known symbol, supported market, risk settings, and active Chase count.
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

When checks pass, Kerosene sends `modify_order` for the same order id and increments the reprice count. If checks are temporarily blocked by throttling, it stores `pending_best_price`; `handle_chase_reprice_tick` later drains that queued price when allowed.

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
