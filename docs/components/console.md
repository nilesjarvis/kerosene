# Console

Open **Widgets → Windows → Console**, or find **Console** in Alfred. It opens a
separate, read-only window; selecting it again focuses the existing window.

The newest activity appears first. Each row shows a UTC timestamp, provider,
HTTP send/response or WebSocket send/receive, and a known operation/channel.
HTTP request IDs match sends with responses, transport failures or cancellations.
Responses include HTTP status and elapsed time to response headers. Proxy
attempts are marked `[proxy]` and counted individually, including retries.
WebSocket rows include frame payload bytes, before stream fanout or coalescing.
Connection attempts, connections and disconnections are also recorded.

Provider and activity filters let users isolate Hyperliquid, Hydromancer or other
services, HTTP, WebSocket, and errors. The provider summary shows HTTP attempts
over the last 60 seconds, requests awaiting headers, average WebSocket receives
and payload KiB per second over the same window, errors and HTTP 429 responses.
These are application traffic counts, **not exchange rate-limit weights**. The
activity filter affects rows; summary counters follow the provider filter.

Pause freezes the displayed history and counters. Scrolling or selecting an
older page also pauses, so incoming events do not move the rows being read.
Resume returns to current activity. Clear hides the current history while
preserving counters. Capture continues while paused or closed. The window and
its filters are runtime-only and are not restored from configuration.

## Implementation

- `network_activity.rs` owns a process-wide, bounded ring of 2,000 metadata
  records plus 60 one-second counter buckets and lifetime counts per provider.
  Counter accuracy does not depend on event retention. Producers append short
  records synchronously; no per-event UI tasks or unbounded channels are used.
- `network_activity/http.rs` observes each reqwest execution. The Hyperliquid
  proxy pool calls it for each direct/proxy attempt; other Rust HTTP callers use
  `HttpRequestExt::send_observed`. Request and response content, routing and
  failure behavior are preserved. A dropped HTTP future records cancellation.
- `network_activity/websocket.rs` observes Hyperliquid and Hydromancer frames at
  their shared socket managers, so multiple consumers do not inflate counts.
- `console_state.rs`, `console_update.rs`, and `console_views.rs` own the window,
  snapshot, filters and pagination. `subscription_state.rs` refreshes the visible,
  live window every 250 ms through `Message::ConsoleTick` and the Console route.
  Rendering is capped at 150 rows per page; older retained rows remain available.
- `main_view/windows.rs`, `window_update.rs`, the Widgets Windows section and
  Alfred handle normal auxiliary-window routing and lifecycle. Console does not
  introduce a `PaneKind` or a persisted config schema.

## Privacy and measurement scope

Only allowlisted provider and operation names, timestamps, request IDs, status,
duration, byte counts and proxy-use flags enter the log. Unknown operations are
shown as `other`; unknown services as `Other API`. No URL, path, query parameter,
header, request/response payload, error string, account address, signature, API
key or proxy credential is retained. History is never written to disk.

HTTP completion measures response headers, not body transfer or application
parsing success. HTTP redirects or transport-internal retries are represented by
their enclosing reqwest execution. WebSocket sizes exclude protocol/TLS overhead.
The Console covers Rust HTTP calls and the two market-data WebSocket managers;
Telegram's native MTProto client, assistant subprocess networking and other
non-HTTP transports are outside this instrumentation.

Tests cover bounds, ordering, rate expiry, provider isolation, redaction,
request/response preservation, cancellation, filters, pause, window lifecycle
and message routing.
