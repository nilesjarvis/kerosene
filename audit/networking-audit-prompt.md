# Networking Stack Deep Audit — Long-Running Goal Prompt

Use this prompt to drive a deep, read-only audit of Kerosene's networking
stack. Treat this as a long-running goal. This is an AUDIT, not a fix pass:
do not modify any source file. The only files you may create or edit are
`audit/networking-audit-progress.md` and `audit/networking-audit-report.md`.

## Objective

Assess whether Kerosene's networking implementation is prudent for production
trading software. Identify concrete issues with evidence, severity, and
suggested fixes — but apply none of them. Focus on the big picture: the final
deliverable is an architecture-level prudence verdict backed by verified
findings, not a pile of nitpicks.

## Repository Context

Kerosene is a Rust 2024 desktop trading terminal for Hyperliquid built with
iced 0.14 (Elm-style state/message/update/view). This is trading software:
never print, snapshot, log, or commit private keys, API keys, bearer tokens,
or other secret material. Read `AGENTS.md` before starting. Prefer `rg` for
search. Ignore unrelated dirty files in the worktree.

You may run `cargo check` and focused `cargo test` invocations to verify
claims; these are the only writes outside `audit/` you may cause (build
artifacts in `target/`).

## Architecture Map (pre-gathered — verify, don't re-derive from scratch)

Three-layer stack, all sharing one global `reqwest::Client`
(`LazyLock`, `src/api.rs:39`; 15s request / 5s connect timeouts):

1. **REST** — `src/api.rs` + `src/api/` (candles, exchange_symbols,
   user_fills, order_book, order_status, calendar, sec, outcome_volume,
   hype_etfs, hype_unstaking_queue). Helpers in `src/account/http.rs`
   (`post_info_json_with_retries`: 3 attempts, aborts on 429) and
   `src/account_analytics/http.rs` (single attempt). All errors are
   `Result<T, String>`.
2. **WebSockets** — exchange WS multiplexer in `src/ws/manager.rs`
   (`ws_manager_task` ~line 102; commands via unbounded mpsc; routed frames
   via `broadcast::channel(10000)`; 16ms L2 coalescer in
   `ws/manager/coalescer.rs`; reconnect policy in `ws/manager/timing.rs`:
   1s base, 2x, 60s cap, no jitter; 45s stale detection; 30s ping).
   Separate Hydromancer WS stack in `src/ws/hydromancer/` (per-API-key
   manager registry with rotation/shutdown, own coalescer, 95s read timeout,
   2s reconnect base). Market/user subscription builders in
   `ws/market_streams*` and `ws/user_streams*`; telemetry counters in
   `ws/telemetry.rs`; assembly into iced subscriptions in
   `src/subscription_state.rs`.
3. **Order transport & signing** — `src/signing/` (~500 lines).
   `signing/client.rs`: atomic monotonic nonce (`allocate_exchange_nonce_from`,
   CAS on `AtomicU64`, ~line 25), msgpack action serialization, Keccak256 +
   EIP-712 + k256 ECDSA, POST to `https://api.hyperliquid.xyz/exchange`,
   30s action expiry. Response parsing in
   `signing/model/exchange_response.rs` (permissive custom deserializer).

Optional integrations: `src/hydromancer_api.rs` (bearer-auth REST, paginated
funding history), `src/hyperdash_api/` (GraphQL), `src/telegram_feed.rs` +
`src/telegram_fast_feed.rs` (HTML polling of t.me every 15s),
`src/x_feed.rs` + `src/x_feed_stream.rs` (X API v2 bearer + stream).

## Seeded Hypotheses (from a prior automated pass — VERIFY or REFUTE each)

Each of these must end the audit marked "confirmed", "refuted", or "partially
confirmed" with file:line evidence:

1. Reconnect backoff has no jitter (`ws/manager/timing.rs`) — thundering-herd
   risk on exchange-wide hiccups.
2. 429 rate-limit backoff exists only in `api/user_fills.rs`; all other REST
   endpoints fail immediately when throttled.
3. `api.rs` client builder falls back silently to `Client::new()` on builder
   failure, losing timeout/UA configuration.
4. Stringly-typed errors (`Result<T, String>`) throughout networking obscure
   transient vs. fatal failures, so callers can't make retry decisions.
5. The 10k-frame broadcast buffer plus coalescer applies no backpressure;
   lagged subscribers silently drop frames until lag detection triggers a
   full refresh.
6. Order POST has no idempotency/ambiguous-failure story: a timeout after
   send leaves order state unknown — check what callers do (retry could
   double-place; CLOID usage may or may not mitigate).
7. Permissive exchange response deserialization could misclassify malformed
   or partial responses as success/failure.
8. Hydromancer API keys live as plain `String` in long-lived manager tasks
   (zeroized only on drop, if at all).
9. Hardcoded timeout/stale constants (45s, 95s, 30s ping) with no runtime
   override — assess whether that's actually a problem for a desktop app.
10. No TLS pinning on the exchange/signing path — assess severity honestly
    for a desktop app trusting the OS store.

## Subagent Fan-Out Plan

The `multi_agent` feature is available — use it. Spawn read-only subagents,
one per track, in parallel where possible. Require each to return findings as
file:line evidence + severity + suggested fix, and to explicitly verify any
seeded hypotheses that fall in its track. Tracks:

1. **Exchange WS lifecycle** — manager task, reconnect/stale/ping handling,
   subscription identity stability across reconnects, resubscribe
   correctness, coalescer correctness (can it deliver stale book state or
   drop the last frame?).
2. **Hydromancer WS stack** — per-key manager registry, key rotation
   shutdown ordering, task leakage, duplicated logic vs. the exchange WS
   stack (consolidation opportunity is a big-picture finding).
3. **REST resilience** — timeout/retry/429 consistency across every
   `src/api/` module and HTTP helper, error typing, the client builder
   fallback, pagination loops (termination, dedup).
4. **Order transport safety** — the full sign-and-post path: nonce
   correctness under concurrency, expiry, ambiguous-failure handling,
   response parsing strictness, what update-loop callers do with failures.
   This is the highest-stakes track; prioritize bugs that could place a
   wrong/duplicate order or misreport order state.
5. **Backpressure & resource bounds** — every channel (mpsc unbounded,
   broadcast 10k, iced stream channels), buffer growth, lag handling, feed
   body-size limits, what happens on slow UI consumers during volatility
   spikes (when message rates are highest and correctness matters most).
6. **Integrations & secrets in transit** — telegram/x/hydromancer/hyperdash
   auth handling, headers, any secret material reaching logs/telemetry/error
   strings/toasts, HTML/JSON parse robustness against hostile or changed
   upstream content.

After tracks complete, run a synthesis pass yourself (no subagent): dedupe,
re-rank, and answer the big-picture questions below.

## Big-Picture Questions the Final Report Must Answer

- Is the overall architecture (single shared HTTP client, dual WS stacks,
  Elm-style task/subscription integration) prudent for trading software?
  What is structurally good and worth preserving?
- Where is the stack fragile under the conditions that matter most: exchange
  outage, rate limiting, high-volatility message bursts, ambiguous order
  submission failures?
- Is the duplication between the exchange and Hydromancer WS stacks earning
  its keep, or should they share a core?
- What are the top 5 changes by risk-reduction-per-effort, in order?

## Severity Scale

- Critical: could place a wrong/duplicate order, misreport order state, leak
  secret material, or lose user data.
- High: likely production failure under realistic stress (exchange hiccup,
  throttling, volatility burst), stuck/leaked task, or broken reconnect.
- Medium: maintainability or robustness issue with clear future cost.
- Low: local cleanup or clarity.

Do not pad the report with Lows; mention them in one short list at the end.

## Deliverables

Maintain `audit/networking-audit-progress.md` as a living log (per finding:
title, severity, evidence with file:line, risk/failure mode, suggested fix,
hypothesis verdict where applicable; plus a session-end status block:
tracks completed, tracks remaining, next step).

Finish by writing `audit/networking-audit-report.md`:

1. Executive summary (one screen: verdict + top findings).
2. How networking is implemented (concise, layer by layer).
3. Prudence verdict with reasoning.
4. Ranked findings (Critical → Medium) with evidence.
5. Seeded-hypothesis verdict table.
6. Top-5 recommended remediations by risk-reduction-per-effort.
7. Short list of Lows.

The audit is complete only when both files exist, every seeded hypothesis has
a verdict, and all six tracks have reported.
