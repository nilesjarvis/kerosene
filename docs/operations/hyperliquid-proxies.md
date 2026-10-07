# Hyperliquid Proxies

Settings > Network provides an optional proxy pool for official Hyperliquid
REST reads. It is disabled by default. Add any number of URLs, then enable
**Distribute API reads across proxies**. Changes apply to new requests immediately;
requests already in flight finish with their original pool.

Supported URL forms:

- `http://proxy.example:8080`
- `https://proxy.example:8443`
- `socks5://proxy.example:1080`
- `socks5h://proxy.example:1080` (the proxy resolves the destination hostname)

URLs may include `username:password@` before the host. Percent-encode reserved
characters in credentials. HTTP and HTTPS default to ports 80 and 443; SOCKS5
and SOCKS5h default to 1080. IPv6 hosts use brackets. Paths other than `/`,
queries, fragments, unsupported schemes, and malformed ports are rejected.
Equivalent normalized URLs cannot be added twice. The list shows host/port and
whether authentication is configured, with both username and password hidden.
Remove a row to replace its URL; removing the last row disables the pool.

## Request Distribution

`api::proxy::HyperliquidRequestExt::send_info` is used by official info fetches:
chart candles/context, symbols, exchange stats, watchlists, books, fills, order
status reads, account snapshots, wallet tracking/details, portfolio/income, and
the API latency probe. Each send selects a healthy route with enough remaining
request-weight capacity and the fewest in-flight requests, rotating between
equally busy routes. Each proxy owns a reusable reqwest client, connection pool,
and rolling-minute budget: 900 weight total, at most 700 for background reads.
The remaining 200 weight is reserved for account-state and order-status reads.
The direct connection retains its own budget and cooldown, independently of
the configured proxies. Routes are charged only after selection; unavailable
routes, unreplayable requests, and admission timeouts do not spend capacity.

Concurrency remains process-wide: four background slots and two account-read
slots, shared with direct Hyperliquid reads. Waiting for direct or proxy budget
capacity releases the concurrency slot and retries selection when capacity can recover;
an exhausted route does not block another route that has capacity. Initial
admission waits at most 90 seconds, covering the 60-second budget window.
Each direct connection or proxy pool admits requests in FIFO order within each
priority so small polls cannot starve larger fill/history pages. The account
queue remains independent of background admission. Retries must fit within the
remaining request deadline and cannot start a fresh admission timeout.

Only POSTs to `https://api.hyperliquid.xyz/info` enter the pool. Signed exchange
actions, WebSocket streams, Hydromancer, HyperDash, Telegram, and other
services retain their existing connections. Selecting Hydromancer as the read
data provider does not send Hydromancer credentials through these proxies;
any remaining official Hyperliquid REST reads still use the pool.

Proxy connections, failed/incomplete response bodies, oversized responses, and
HTTP 5xx/407 failures cool down for 15 seconds. HTTP 429
uses `Retry-After` (seconds or HTTP date), bounded to 1 second–1 hour, with a
60-second default. A read can try at most two distinct routes within the
15-second request budget after initial admission (or a shorter explicit request
timeout). This includes response-body transfer and retry admission. Successful
responses are buffered up to 16 MiB before releasing the route and concurrency
slot, so a slow download remains in flight and a broken body can fail over.
Response status, headers, URL, and decoding behavior are preserved for callers.
Failed routes return to rotation after their
cooldown. Concurrent successful requests do not erase an existing cooldown.
There is no automatic direct fallback while the pool is enabled: if every
route is unavailable, the read returns an error and normal feature refreshes
can try again after recovery. Disabling the pool restores the original client,
including its existing system-proxy behavior.

HTTPS retains certificate verification and uses CONNECT for HTTP proxies.
Redirects are disabled on proxy clients. Proxy transport errors and non-success
response bodies are never included in user-visible errors, since a proxy can
echo authentication material.

Hyperliquid documents per-IP REST request weight limits and separate
address-based limits for trading actions. Additional proxies now provide
additional local request-weight capacity. Each configured URL is treated as an
independent route; the app does not discover or deduplicate their outbound IPs.
Use distinct outbound IPs to obtain additional exchange capacity. URLs sharing
an outbound IP also share the exchange's limit, and other users of that IP can
consume it. Proxy routing does not change account-based action limits.
See the [official rate-limit documentation](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/rate-limits-and-user-limits).

## Persistence And Lifecycle

`hyperliquid_proxies_enabled` is a backwards-compatible, default-false config
field. Full URLs are `ProxyUrl` values with redacted Debug output and zeroizing
storage. They live in `SecretPayload.global.hyperliquid_proxy_urls`, whose default
is an empty list for older credential bundles. Raw URLs are skipped during
config serialization/deserialization; runtime config snapshots carry no URLs.

When enabled, pool construction revalidates stored URLs because secret-payload
deserialization does not run the input parser. Applying invalid saved URLs keeps
enabled reads paused until the entries are corrected or the pool is disabled.

The existing OS keychain and encrypted-config flows save, load, migrate, and
clear the proxy list together with other credentials. Explicit list mutations
must successfully persist before runtime state changes. Saving another
credential preserves the list. The enable switch uses normal debounced config
persistence. Clear All Config resets both the list and the active pool.

When encrypted credentials are locked at startup, an enabled pool pauses
Hyperliquid REST reads until the saved proxies are unlocked or the pool is
disabled. Settings > Network offers an unlock control in that state.

The four proxy settings messages route to `update_settings` and its `proxy`
submodule: `HyperliquidProxyInputChanged(SecretInput)`, `AddHyperliquidProxy`,
`RemoveHyperliquidProxy`, and `SetHyperliquidProxiesEnabled`.

## Validation

Local mock-proxy tests cover request/authentication forwarding, HTTPS CONNECT,
route balancing beyond a single-IP budget, per-route account reserves, exhausted
route selection, unavailable-route accounting, bounded admission/retries,
cooldown/recovery, Retry-After parsing, full-body load tracking, truncated and
oversized body failover, cancellation, request timeout, response metadata,
redaction, and destination isolation. Settings
and persistence tests cover validation, deduplication, routing, legacy defaults,
secret-store failures, encrypted round trips, both storage migration directions,
scoped keychain updates, and runtime clearing. They make no live trades and do
not contact a public proxy or the Hyperliquid API.
