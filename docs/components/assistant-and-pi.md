# Kerosene Assistant And Pi

The Kerosene Assistant is a native iced chat window backed by the open-source
[Pi agent harness](https://github.com/earendil-works/pi) in RPC mode and the
user's configured OpenRouter account. The
MVP is intentionally read-only: the assistant can inspect a sanitized Kerosene
snapshot, reason about it, and produce explanations or analysis code, but it
cannot place orders or mutate application state.

## Component Map

| File | Responsibility |
| --- | --- |
| `src/agent_state.rs` | Window, transcript, runtime status, streaming, tool cards, and redacted prompt wrapper. |
| `src/agent_update.rs` | Window lifecycle, prompt submission, snapshot/runtime orchestration, and stale-generation guards. |
| `src/agent_views.rs` | Native chat window, composer, status, usage, empty state, and tool activity UI. |
| `src/agent_snapshot.rs` | Versioned, bounded, sanitized read-only export of Kerosene state. |
| `src/agent_runtime.rs` | Pi subprocess discovery, isolated environment, JSONL RPC transport, and event parsing. |
| `assets/agent/kerosene.ts` | Embedded Pi extension, typed read-only tools, deterministic calculations, and fixed-provider data adapters. |

The assistant opens from the Widgets menu or the OpenRouter section of
Settings > Integrations. Closing the window terminates Pi, clears the transcript,
and deletes the sensitive snapshot. Sessions use Pi's `--no-session` mode and
are never written to Pi's session store.

## Runtime Contract

Kerosene starts Pi with:

- RPC mode over stdin/stdout JSONL
- the `openrouter` provider and configured model
- session persistence disabled
- an isolated, empty Pi configuration directory and temporary project workspace
- a strict allowlist containing only `kerosene_*` read-only tools
- an isolated `PI_CODING_AGENT_DIR`
- version checks and telemetry disabled

The OpenRouter key is passed only in the child environment as
`OPENROUTER_API_KEY`; it is never placed in arguments, snapshot content, RPC
messages, or debug output. If configured, the HyperDash key is passed as
`KEROSENE_AGENT_HYPERDASH_API_KEY` and can only be consumed by the embedded
extension's fixed aggregate-positioning queries. Pi has no shell, file-read, or
generic network tool with which to inspect either environment value. Runtime
errors additionally redact both keys. Changing an integration key or model
terminates or invalidates the relevant active session/request generation.

The enabled tool allowlist is:

- `kerosene_data`
- `kerosene_market_data`
- `kerosene_activity`
- `kerosene_calculate`
- `kerosene_risk`
- `kerosene_positioning`
- `kerosene_ohlcv`
- `kerosene_sessions`

Built-in Pi tools such as `bash`, `read`, `write`, and `edit` are not enabled.

Pi executable discovery checks, in order:

1. `KEROSENE_PI_BINARY`
2. a `pi`/`pi.exe` binary next to the application or in common packaged
   resource directories
3. `pi`/`pi.exe` on `PATH`

For development, install Pi with:

```text
npm install -g @earendil-works/pi-coding-agent
```

The npm distribution requires Node.js 22.19 or newer. The standalone Pi binary
does not require a user-installed Node.js runtime.

Packaging a pinned Pi standalone binary beside Kerosene is the intended release
path; it avoids requiring Node.js on user machines.

## Snapshot Contract

Each prompt first writes a fresh `schema_version: 2` JSON snapshot to a
per-process temporary directory. On Unix, the directory and file use owner-only
permissions. The snapshot contains public sections and a private sanitized
`_tool_data` backing index. `kerosene_data`, including its `all` mode, never
returns `_tool_data`; typed tools use it to answer targeted queries without
sending the entire market or activity history to the model.

Sections are:

- `overview`
- `account`
- `portfolio`
- `markets`
- `positioning`
- `sessions`
- `all`

The account section includes margin summary, positions, spot balances, open
orders, recent fills, recent funding, and completeness metadata. Portfolio data
is summarized by bucket. Positioning exposes aggregate totals and aggregate
change statistics, never wallet-level identities. Lists are bounded to prevent
uncontrolled context growth.

Every major section now reports provenance and/or coverage. List coverage keeps
`returned_count`, `total_count`, `truncated`, `endpoint_fetch_complete`, and
`complete_for_current_state` distinct. Market selection prioritizes the active
symbol, favourites, and account-relevant symbols before applying the 250-row
public cap. The private market index retains every sanitized Kerosene mid and
its canonical/display metadata for targeted lookup. Raw `@N` and `#N` values
are documented as exchange identifiers, not privacy redaction.

The snapshot explicitly omits:

- API and private keys
- wallet addresses
- order IDs
- transaction hashes
- wallet-level HyperDash identities

Errors are represented as booleans instead of raw upstream messages because
those messages can contain sensitive request context.

## Typed Tool Contract

- `kerosene_data` reads one public snapshot section. `all` is reserved for a
  genuine cross-component summary.
- `kerosene_market_data` resolves up to 20 raw/canonical/display symbols against
  the complete private market index.
- `kerosene_activity` filters or deterministically aggregates sanitized fills
  and funding with bounded pagination. Order IDs and transaction hashes remain
  omitted.
- `kerosene_calculate` performs allowlisted exposure, liquidation-buffer,
  stress, fill, funding, and reconciliation calculations. It is not an
  arbitrary code executor.
- `kerosene_risk` returns clearinghouse, spot, portfolio, and income scopes
  separately, along with deterministic ratios and explicit interpretation.
- `kerosene_positioning` uses fixed HyperDash GraphQL operations and returns
  aggregate long/short and aggregate change statistics only. Individual wallet
  identities are not returned to the model.
- `kerosene_ohlcv` uses a fixed Hyperliquid endpoint, allowlisted intervals,
  validated Kerosene symbols, a 90-day maximum request window, and a 500-row
  output cap.
- `kerosene_sessions` computes bounded weekday and DST-aware Kerosene market
  session summaries from fixed Hyperliquid daily/30-minute candle requests,
  independent of whether a Session Data pane is open.

The system prompt directs the model to use deterministic tools for arithmetic,
avoid guessing symbol mappings, use plain Markdown rather than unsupported
LaTeX delimiters, and stop after a decisive complete empty-state result.

If current Pi emits `agent_end` without visible text, Kerosene retries once with
a short visible-answer instruction. A second empty settlement becomes an
explicit error instead of being presented as a successful blank response.

## Current MVP Limits

- Pi must be installed or bundled separately.
- Chat history is ephemeral and is cleared when the window closes.
- Assistant output renders streamed Markdown with headings, emphasis, lists,
  quotes, tables, links, inline code, and highlighted fenced code blocks.
- No shell, filesystem, order, signing, or mutation tools are exposed.
- Deterministic analysis is limited to the allowlisted operations. General
  executable analysis still needs a separately sandboxed runtime before it is
  safe to enable.
- OHLCV/session and aggregate-positioning tools use fixed provider operations;
  there is no generic URL or query input.
- Snapshot refresh happens at prompt boundaries, not continuously during a
  single agent turn.

## Follow-up Scope

The next production increment should pin and package Pi binaries for macOS,
Windows, and Linux; cover packaging smoke tests; evaluate whether an additional
no-network/no-filesystem analysis sandbox is justified beyond the deterministic
tools; and optionally introduce consented, versioned data plugins beyond the
current contracts.
