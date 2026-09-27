# Native desktop audit harness

This harness drives the actual iced application in a dedicated Xvfb display,
using XTEST keyboard and pointer events. It measures update, view-construction
and subscription-construction time, records the existing redacted network
telemetry, samples process CPU/RSS/descriptors, and captures screenshots.

It builds a **copy of the current working tree**, including uncommitted Rust
changes. It does not modify application source or replace the normal binary.
The generated binary requires `--test`; saved accounts, keychain entries,
configuration and disk caches are not loaded. No signing credentials are supplied.

See [the measured audit](../../docs/end-to-end-performance-audit.md) for findings,
evidence, acceptance criteria, and the remaining whole-application coverage plan.

## Requirements

Linux, Rust/Cargo, Xvfb, xwininfo, libX11, libXtst, and Python with Pillow,
psutil and websockets. No desktop session or window manager is required.
The tested machine used Python 3.14, Vulkan and an NVIDIA RTX 4090. Other renderers
need their own baselines; the fixed 1600×1000 scripts are not cross-platform tests.

## Build and run

From the repository root:

```sh
python3 tests/e2e/prepare.py

# Once: nine public, unauthenticated metadata reads, spaced two seconds apart.
# Reuse this directory on subsequent runs. No candle-history fan-out is captured.
python3 tests/e2e/capture_public.py target/e2e/public-fixtures

# Fully local, repeatable 2-minute desktop/stream scenario; includes assertions.
python3 tests/e2e/suite.py target/e2e/public-fixtures target/e2e/baseline
```

Output paths must be new. The source manifest records the Git HEAD, per-file
SHA-256 values and generated message count. An existing ordinary release build
shares dependency artifacts, but the harness binary is `kerosene-e2e`.

For the isolated receiver-retention experiment described in the report:

```sh
python3 tests/e2e/prepare.py --no-idle-receiver
python3 tests/e2e/suite.py target/e2e/public-fixtures target/e2e/comparison --no-idle-receiver
```

That switch changes **only the staged Hyperliquid manager** from retaining an
unread receiver to retaining a sender and creating receivers via `subscribe()`.
It is an experiment, not a production fix or a fully validated manager refactor.
The source remains under `target/e2e/source-no-idle-receiver` for review.

## Exploratory desktop and faults

In separate terminals:

```sh
python3 tests/e2e/server.py target/e2e/public-fixtures target/e2e/local-server
python3 tests/e2e/desktop.py target/e2e/session --fixture target/e2e/local-server/session.json --seconds 900
```

Send actions to the running desktop:

```sh
python3 tests/e2e/send.py target/e2e/session '[{"action":"click","x":800,"y":693},{"action":"pause","seconds":1},{"action":"capture","label":"terminal"}]'
python3 tests/e2e/send.py target/e2e/session '{"action":"fault","settings":{"http_status":429,"fault_types":["candleSnapshot"]}}'
python3 tests/e2e/send.py target/e2e/session '{"action":"fault","settings":{}}'
python3 tests/e2e/send.py target/e2e/session '{"action":"quit"}'
python3 tests/e2e/analyze.py target/e2e/session
```

Supported actions: `click`, `move`, `down`, `up`, `scroll` (buttons 4/5), `key`
(X key names), `type`, `focus`, `resize`, `capture`, `pause` (at most 5 seconds),
`phase`, `fault`, and `quit`. `focus`/`resize` accept an exact window title or XID.
Use `capture` to get both a screenshot and the current window tree. Input coordinates
are absolute within the isolated display. Add a pause after transitions; dispatching
an input is not proof that the intended control handled it.

Fixture controls:

| Setting | Effect |
| --- | --- |
| `tick_ms: 50` | Synthetic market snapshots at 20 Hz per subscribed topic. |
| `delay_ms: 2000` | Delay HTTP responses by two seconds. |
| `http_status: 429`, `fault_types: ["candleSnapshot"]` | Fail candle reads with `Retry-After: 60`. |
| `malformed: true`, `fault_types: ["candleSnapshot"]` | Return HTTP 200 with an invalid candle response shape. |
| `disconnect: N` | Changing this generation closes active WebSockets; reconnect is accepted. Keep the generation unchanged when changing unrelated options. |
| `silent: true` | Stop snapshot pushes, while the connection still answers protocol pings. |

An empty control object restores normal responses. Changing `disconnect` back to
zero also causes a disconnect. The suite records fault commands alongside inputs.
Exploratory controls written directly to the server file are not automatically
recorded in the desktop action log.

The optional `--live` desktop mode skips endpoint redirection. The historical
audit predates the shared request budget and demand-driven outcome-volume fixes;
its startup burst and 429 counts are not a current baseline. Use fixtures for
repeatable tests. This is not a live load-test runner.

## What the artifacts establish

- `telemetry.jsonl`: enum variant names and timing histograms, state **counts**,
  allowlisted network metadata, totals, and dropped-entry counts. No message
  payloads, URLs, headers, credentials, account addresses or error strings are exported.
- `process.jsonl`: wall-clock samples, phase, CPU (100% = one core), RSS, threads,
  and file descriptors. CPU means are weighted by actual sample intervals.
- `actions.jsonl`, PNGs and window trees: real input dispatch and visible results.
- Local server `requests.jsonl`: public operation/symbol/interval, response code,
  timing and concurrent handler count; no account or request-body logging.
- `summary.json`: aggregated metrics. Logarithmic timing percentiles are **upper
  bounds**, not exact quantiles. HTTP concurrency stops at headers, not body completion.
- `checks.json`: automated proof of onboarding/search event handling, market
  delivery, loaded history, telemetry continuity, surviving process and no account.

The default suite intentionally measures a fixed memory/stream scenario; it is
not a whole-application acceptance suite. `--scenario FILE` accepts a different
JSON action list; its checks still require the basic events and a `finish` capture.

## Measurement boundaries

The HTTP observer records completion at response headers; semantic parsing,
body completion and provider weights need additional instrumentation. Rust
`view` timing measures widget construction, not layout, draw, GPU execution,
presentation, or input-to-pixel latency. No FPS claim can be made from these
numbers. Screenshot capture and instrumentation add overhead.

The local server replays captured metadata and generates candle/book/tick data.
HIP-3 mids/contexts are synthetic. It is not an exchange execution emulator.
Unknown HTTP operations fail with 501; `/exchange` is rejected. Replay redirects
observed Rust HTTP calls and the Hyperliquid WebSocket manager only. It does not
emulate paid-provider sockets, Telegram MTProto, assistant subprocesses, or OS
keychain/persistence. No credentials should be entered into this test desktop.

Xvfb/process cleanup occurs when the driver exits normally. After manually
interrupting a run, check its recorded PID and display for surviving children.
The fixture server must also be stopped when an exploratory session finishes.
