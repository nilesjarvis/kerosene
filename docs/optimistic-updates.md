  # Optimistic Account Updates After Order Placement

  ## Summary

  - Implement optimistic client-side account updates after a successful
    single-order placement response.
  - Target normal order form and quick-order submissions first.
  - Keep the existing immediate account refresh, so optimistic data only
    improves perceived responsiveness while websocket/account snapshots
    remain authoritative.
  - Expected responsiveness gain: after the exchange success response
    arrives, open-order/fill UI can update on the next app update frame
    instead of waiting for websocket or full account refresh. That should
    reduce post-success visible latency from hundreds of milliseconds to
    several seconds down to effectively immediate UI latency. It will not
    reduce the network latency before the success response.

  ## Key Changes

  - Add an internal optimistic order context captured at submission time:
      - symbol, side, size, price, order kind, reduce-only, timestamp, and
        submission source.
      - Attach this context to OrderResult and QuickOrderResult messages
        instead of relying on mutable pending state.
  - Add transient optimistic account effects to TradingTerminal:
      - optimistic resting orders keyed by oid.
      - optimistic fills keyed by oid.
      - no optimistic mutation of margin, available balance, account value,
        or positions.
  - On successful non-error exchange responses:
      - If response contains resting.oid, immediately show an optimistic
        open order.
      - If response contains filled, immediately show an optimistic fill/
        trade marker using response totalSz, avgPx, and oid.
      - Ignore ambiguous or error responses for optimism, but keep current
        refresh behavior.

  ## Implementation Points

  - Submission context:
      - Build context in order_execution/submit.rs and order_execution/
        quick_order/submit.rs.
      - Update result handling in order_update/results.rs and order_update/
        quick_order/form/result.rs.
  - Optimistic projection:
      - Add a small helper module under order/account update code to
        convert successful ExchangeResponse plus submission context into
        optimistic open orders/fills.
      - Use separate optimistic state rather than inserting directly into
        authoritative account_data.
  - UI/data merge:
      - Update chart order overlays and trade markers in chart_state/
        overlays.rs to include optimistic effects.
      - Update open-orders and trade-history views to include optimistic
        rows until authoritative data catches up.
      - Update order-book user-level display if it currently reads directly
        from account_data.open_orders.
  - Reconciliation:
      - Remove optimistic orders when websocket or account refresh contains
        the same oid.
      - Remove optimistic resting orders if authoritative fills contain the
        same oid.
      - Remove optimistic fills when authoritative fills contain the same
        oid.
      - Expire any unmatched optimistic effect after a short timeout, e.g.
        30 seconds, to avoid stale UI if reconciliation never arrives.

  ## Test Plan

  - Unit test successful resting order response creates an optimistic open
    order and chart overlay immediately.
  - Unit test successful filled response creates an optimistic fill/trade
    marker immediately.
  - Unit test exchange error and ambiguous response do not create
    optimistic account effects.
  - Unit test websocket/account refresh with matching oid removes
    optimistic effects.
  - Unit test stale optimistic effects expire without touching
    authoritative account_data.
  - Existing tests for result_requires_account_refresh should continue to
    pass, confirming the full refresh still happens.

  ## Assumptions

  - Scope is normal order form plus quick order; chase, TWAP, cancel, move,
    close-position, and nuke flows stay authoritative-only for the first
    pass.
  - Optimistic fills may have incomplete metadata such as fee, closed PnL,
    and exact direction, so they should be visually treated as pending/
    optimistic where shown.
  - Authoritative websocket/account refresh always wins over optimistic
    state.

