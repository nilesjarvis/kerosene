# Kerosene Assistant And Pi

The Kerosene Assistant is a native iced chat window backed by the open-source
[Pi agent harness](https://github.com/earendil-works/pi) in RPC mode and either
the user's configured OpenRouter account or an auto-detected local llama.cpp
server. The Assistant is analysis-first: it can inspect a sanitized Kerosene
snapshot, reason about it, and produce explanations or analysis code. Its only
application mutations are explicitly allowlisted, reversible actions that
enable or disable supported visual indicators and create or remove persisted
drawing items on already-open candlestick charts. It cannot create charts,
change their market or timeframe, edit an existing drawing in place, expose
trading controls, place or cancel orders, sign, or invoke arbitrary application
messages.

## Component Map

| File | Responsibility |
| --- | --- |
| `src/agent_pnl_card.rs` | Bounded image selection, validation, normalization, preview, and redacted transport types. |
| `src/agent_state.rs` | Window, transcript, runtime status, answer/reasoning streaming, tool cards, and redacted prompt wrapper. |
| `src/agent_update.rs` | Window lifecycle, prompt submission, snapshot/runtime orchestration, and stale-generation guards. |
| `src/agent_views.rs` | Native chat window, composer, status, usage, empty state, and tool activity UI. |
| `src/agent_snapshot.rs` | Versioned, bounded, sanitized read-only export of Kerosene state. |
| `src/agent_workspace.rs` | Strict host-action contract, active-turn authorization, all-or-nothing validation, idempotent chart mutations, and acknowledgements. |
| `src/agent_runtime.rs` | Pi subprocess discovery, isolated environment, JSONL RPC transport, correlated extension UI responses, and event parsing. |
| `src/llama_cpp.rs` | Loopback-only llama.cpp process/endpoint discovery, capability verification, and isolated Pi provider configuration. |
| `src/chart_indicator.rs` | Shared typed registry for chart UI indicators and Assistant-visible indicator capabilities. |
| `assets/agent/kerosene.ts` | Embedded Pi extension, typed snapshot/data tools, bounded indicator and drawing actions, deterministic calculations, and fixed-provider data adapters. |

The assistant opens from the Widgets menu or the OpenRouter section of
Settings > Integrations. Opening it starts a bounded local llama.cpp detection
task. Closing the window terminates Pi, discards unsent image
attachments, and deletes the sensitive snapshot. Kerosene persists its bounded
chat sessions separately, while Pi uses `--no-session` and never writes them to
its own session store.

The provider/model name in the Assistant prompt bar opens the provider selector.
OpenRouter retains its searchable live catalog, limited to text-output models
that advertise tool calling. A compatible detected llama.cpp server appears as
a separate local provider with endpoint, model, context, tool, and vision
capabilities. Provider choice is persisted; changing provider or model restarts
Pi before the next turn.

Local discovery checks `KEROSENE_LLAMA_CPP_URL`, running `llama-server` process
arguments, and the conventional ports 8080 and 8081. Only plain-HTTP loopback
URLs are accepted. Kerosene verifies llama.cpp's `/props` response and
`/v1/models` catalog without sending a generation request. A local server is
selectable only when its chat template advertises tool calling, because the
Assistant depends on the `kerosene_*` tools. Non-default process ports are
supported.

## P&L Card Investigation

The empty state and composer expose a P&L card action. Users can choose a PNG,
JPEG, or WebP file, or drag one anywhere over the Assistant window on platforms
where iced supports file-drop events. Kerosene decodes the image with strict
file, dimension, and allocation limits, resizes it to at most 2000×2000, and
normalizes it to an in-memory PNG. The preview and image bytes are transient and
are never written to `assistant_sessions.json`.

Attaching a card requires the selected provider to advertise image input as
well as tool calling. Pi receives the normalized image through its
RPC `prompt.images` field. The model first extracts only visible trade fields;
when a perp symbol and a position-specific number are available, it can call the
attachment-gated `kerosene_pnl_card_match` tool.

That tool performs a bounded HyperDash current-position search, scores candidates
with explicit per-field tolerances, and validates up to ten leading candidates
against Hyperliquid `clearinghouseState`. It returns at most five public wallet
addresses with score evidence, coverage, timestamps, and validation state. An
address is always presented as a public position candidate, never as proof of a
person's identity or wallet ownership. Closed or old cards may have no match
because the provider path covers current open positions.

Assistant text uses an adaptive native reveal queue on top of Pi's real text
deltas. Short backlogs resolve word by word with a fading leading edge and an
inline activity cursor; larger backlogs reveal in wider batches so presentation
does not fall materially behind the model. A settled response flushes the final
backlog before the turn becomes ready. Tool boundaries and abort/error paths
also flush pending text so a delta can never be rendered into the wrong message.

Reasoning-capable models run with Pi's medium thinking level. Kerosene consumes
Pi's real `thinking_start`, `thinking_delta`, and `thinking_end` RPC events; it
does not derive or fabricate reasoning from the visible answer. Each reasoning
block appears as an expanded `Thought for …` disclosure with a compact sparkle
header, elapsed time, muted text, and a vertical trace rail. The user can
collapse or reopen it. Reasoning event payloads use redacted `Debug` output,
are bounded in memory, and remain transient: they are not copied, replayed into
future model context, or written to `assistant_sessions.json`.

Tool calls use the same trace language. The first call in a user turn renders a
single expanded `Running … tools` disclosure that absorbs later calls from the
same turn and settles to `Ran … tools`. Its compact rows show a plain action
verb, bounded request detail, and only exceptional state (`Running` or
`Failed`) beside a vertical rail. The disclosure can be collapsed at any time;
individual tool cards and the duplicate post-response data-call drawer are not
rendered.

Completed responses expose Copy and Regenerate actions and up to two
model-authored follow-up questions. The system prompt requires an exact hidden
`KEROSENE_FOLLOW_UPS_V1` JSON metadata block at the end of every response. The
questions must be specific to the user's request and the answer's concrete
findings or uncertainties; generic category-based fallbacks are not generated.
Kerosene bounds and validates the questions, removes the metadata block before
rendering, copying, replaying, or persisting the answer, and fills the composer
for review instead of sending a selected follow-up immediately. These
presentation controls, follow-ups, and tool traces are transient; persisted
session content remains limited to user and assistant messages.

Opening the Assistant while an account is connected also makes it an active
journal-data consumer. Kerosene immediately hydrates any local journal cache and
starts the normal incremental journal sync even when the Trading Journal window
is closed. A prompt snapshot can therefore use cached trades while a refresh is
in progress, with partial coverage reported explicitly.

## Runtime Contract

Kerosene starts Pi with:

- RPC mode over stdin/stdout JSONL
- either the `openrouter` provider and configured model or an isolated
  `llamacpp` provider and the verified local model
- session persistence disabled
- medium reasoning enabled for models that advertise reasoning support (Pi
  clamps non-reasoning models to off)
- an isolated, empty Pi configuration directory and temporary project workspace
- a strict allowlist containing only documented `kerosene_*` tools; two tools
  can request bounded visual-indicator or drawing host actions
- an isolated `PI_CODING_AGENT_DIR`
- version checks and telemetry disabled

For llama.cpp, Kerosene writes a zero-cost custom provider to the isolated Pi
`models.json`, points it at the verified loopback `/v1` URL, and does not put the
OpenRouter key in the child environment. The file is not written to the user's
normal Pi configuration. Model prompts therefore stay on the user's machine;
the allowlisted Kerosene tools may still call their documented market-data
sources.

The OpenRouter key is passed only for OpenRouter sessions in the child environment as
`OPENROUTER_API_KEY`; it is never placed in arguments, snapshot content, RPC
messages, or debug output. If configured, the HyperDash key is passed as
`KEROSENE_AGENT_HYPERDASH_API_KEY` and can only be consumed by the embedded
extension's fixed aggregate-positioning and attachment-gated P&L matching
queries. Pi has no shell, file-read, or
generic network tool with which to inspect either environment value. Runtime
errors additionally redact both keys. Changing an integration key or model
terminates or invalidates the relevant active session/request generation.

The enabled tool allowlist is:

- `kerosene_data`
- `kerosene_set_chart_indicators`
- `kerosene_manage_chart_drawings`
- `kerosene_market_data`
- `kerosene_activity`
- `kerosene_journal`
- `kerosene_calculate`
- `kerosene_risk`
- `kerosene_positioning`
- `kerosene_pnl_card_match`
- `kerosene_ohlcv`
- `kerosene_sessions`

Built-in Pi tools such as `bash`, `read`, `write`, and `edit` are not enabled.

Pi executable discovery checks, in order:

1. `KEROSENE_PI_BINARY`
2. the private Pi runtime bundled in Kerosene's Linux, macOS, Windows, or
   AppImage resource directory
3. a legacy `pi`/`pi.exe` binary next to the application or in common packaged
   resource directories
4. `pi`/`pi.exe` on `PATH`

Release packaging reads the pinned version from `packaging/pi/version.txt`,
downloads the matching official standalone archive, and verifies its digest
against `packaging/pi/SHA256SUMS`. The minimal runtime bundle includes the
executable plus the package metadata and built-in themes required for RPC
startup. Packaging fails unless the bundle reports the pinned version and
passes an offline `get_state` RPC smoke test with Kerosene's embedded extension.
The upstream Pi license ships beside the runtime.

For development, install Pi with:

```text
npm install -g --ignore-scripts @earendil-works/pi-coding-agent
```

The npm distribution requires Node.js 22.19 or newer. This is a development
fallback only. Packaged Kerosene releases use the standalone Pi runtime and do
not require a user-installed Node.js runtime or a shell-visible `pi` command.

## Snapshot Contract

Each prompt first writes a fresh `schema_version: 5` JSON snapshot to a
per-process temporary directory. On Unix, the directory and file use owner-only
permissions. The snapshot contains public sections and a private sanitized
`_tool_data` backing index. `kerosene_data`, including its `all` mode, never
returns `_tool_data`; typed tools use it to answer targeted queries without
sending the entire market or activity history to the model.

Sections are:

- `overview`
- `workspace`
- `account`
- `portfolio`
- `markets`
- `journal`
- `positioning`
- `sessions`
- `all`

The workspace section contains a bounded list of open candlestick charts, their
stable chart IDs, selected/surface state, symbol, timeframe, current
Assistant-visible indicator states, selected drawing ID, and bounded persisted
drawing records. Its catalogs advertise exact indicator and drawing type IDs,
dependencies, supported styles, and the Unix-millisecond/positive-price anchor
contract. Per-chart and global drawing coverage make truncation explicit. The
Assistant must read this section immediately before a chart action; the Rust
host revalidates chart and drawing IDs, catalog membership, dependencies,
coordinates, active tool call, and non-aborted turn before mutating anything.
Free-form drawing labels are length-bounded and credential-redacted before they
enter the snapshot.

The account section includes margin summary, positions, spot balances, open
orders, recent fills, recent funding, and completeness metadata. Portfolio data
is summarized by bucket. The public journal section reports availability, sync
state, trade/reflection counts, and coverage; individual journal trades remain
in the private sanitized tool index. Positioning exposes aggregate totals and aggregate
change statistics, never wallet-level identities. Lists are bounded to prevent
uncontrolled context growth.

Every major section now reports provenance and/or coverage. List coverage keeps
`returned_count`, `total_count`, `truncated`, `endpoint_fetch_complete`, and
`complete_for_current_state` distinct. Market selection prioritizes the active
symbol, favourites, and account-relevant symbols before applying the 250-row
public cap. The private market index retains every sanitized Kerosene mid and
its canonical/display metadata for targeted lookup. Raw `@N` and `#N` values
are documented as exchange identifiers, not privacy redaction.

Snapshot generation time and source observation time are separate. The root
`generated_at_ms` records serialization time; section provenance keeps
`observed_at_ms`/the compatibility alias `as_of_ms`, computed age, and an
explicit freshness state. Missing observation time stays null instead of being
replaced with snapshot generation time.

The snapshot explicitly omits:

- API and private keys
- wallet addresses
- order IDs
- transaction hashes
- internal journal trade and legacy-note IDs
- wallet-level HyperDash identities

The snapshot itself always keeps those omissions. For a turn with an explicit
P&L image attachment, a private request flag authorizes only
`kerosene_pnl_card_match` to return a bounded set of public candidate addresses;
the addresses are fetched on demand and are not added to the snapshot.

Errors are represented as booleans instead of raw upstream messages because
those messages can contain sensitive request context.

## Typed Tool Contract

- `kerosene_data` reads one public snapshot section. `all` is reserved for a
  genuine cross-component summary.
- `kerosene_set_chart_indicators` idempotently sets explicit enabled states on
  one or more already-open candlestick charts. The extension first validates
  requested chart and indicator IDs against the current workspace snapshot,
  then uses a reserved correlated Pi extension-UI request to ask the Rust host
  to apply the complete batch. The host preflights the entire request before
  mutation, excludes presentation labels and Quick Trade controls, rejects
  missing dependencies, applies the batch once, schedules config persistence,
  and returns per-chart `changed` or `already_set` outcomes. It cannot create a
  chart, change symbols/timeframes, place an order, or dispatch a generic
  Kerosene message.
- `kerosene_manage_chart_drawings` atomically creates or removes up to 64
  persisted annotations across already-open candlestick charts. It supports
  horizontal levels, vertical lines, trend lines, rays, extended lines,
  rectangles/zones, price/time measurements, Fibonacci retracements, and
  Fibonacci extensions. Adds use exact Unix-millisecond/positive-price anchors
  and an allowlisted style vocabulary; an exact geometry/style retry returns
  `already_present`. Removes require a current drawing ID and reject locked
  annotations. The host applies no part of a batch if any operation fails,
  mirrors the result into the canvas, and schedules normal config persistence.
  It does not switch toolbar modes or edit an existing drawing's geometry or
  style in place.
- `kerosene_market_data` resolves up to 20 raw/canonical/display symbols against
  the complete private market index.
- `kerosene_activity` filters or deterministically aggregates sanitized fills
  and funding with bounded pagination. Order IDs and transaction hashes remain
  omitted. Malformed financial rows are excluded with validation counts rather
  than silently converted to zero.
- `kerosene_journal` reads the active account's reconstructed journal trades,
  including fee-adjusted realized PnL, entry/volume efficiency metrics,
  basis-quality flags, bounded credential-redacted reflections, and tags. It
  can list, summarize, or rank best/worst trades with symbol, side, market,
  annotation, and time filters. Best/worst defaults to closed, basis-complete
  trades ranked by `gross realized PnL - journal fees`; responses expose sync
  and truncation coverage.
- `kerosene_calculate` performs allowlisted exposure, liquidation-buffer,
  stress, fill, funding, reconciliation, and bounded OHLCV-statistics
  calculations. Candle statistics include deterministic return, dispersion,
  realized-volatility, drawdown, ATR, and moving-average formulas. It is not an
  arbitrary code executor.
- `kerosene_risk` returns clearinghouse, spot, portfolio, and income scopes
  separately, along with deterministic ratios and explicit interpretation.
- `kerosene_positioning` uses fixed HyperDash GraphQL operations and returns
  aggregate long/short and aggregate change statistics only. Individual wallet
  identities are not returned to the model.
- `kerosene_pnl_card_match` is disabled unless the current snapshot records an
  explicit P&L image attachment. It uses fixed HyperDash and Hyperliquid
  operations, bounded candidate/validation counts, deterministic scoring, and
  returns public position candidates with uncertainty and coverage rather than
  personal attribution.
- `kerosene_ohlcv` uses a fixed Hyperliquid endpoint, allowlisted intervals,
  validated Kerosene symbols, a 90-day maximum request window, and a 500-row
  output cap.
- `kerosene_sessions` computes bounded weekday and DST-aware Kerosene market
  session summaries from fixed Hyperliquid daily/30-minute candle requests,
  independent of whether a Session Data pane is open.

### Drawing Intent And Coordinate Resolution

The first prompt presents drawings as direct workspace operations, not as
toolbar automation. The model should translate common requests as follows:

| User intent | Required behavior |
| --- | --- |
| “Draw support at 100,000 on this chart” | Read `workspace`, target the selected chart, and add a `horizontal_level` at the exact supplied price. |
| “Mark the current price” | Resolve the chart, fetch current market data in the same turn, and use the observed price rather than a remembered value. |
| “Connect the last two swing lows” | Read the chart symbol/timeframe, fetch bounded OHLCV, identify and disclose the chosen candles, then use their exact timestamps and lows. Ask if the lookback or swing definition would materially change the result. |
| “Box this range” | Use exact supplied/evidenced time-price corners. A chart ID alone does not encode a selected visual range, so ask when the corners are unavailable. |
| “Delete this drawing” | Remove `selected_drawing_id` only when the selected chart/drawing reference is unambiguous. |
| “Clear my drawings” | Remove only enumerated, unlocked IDs when drawing coverage is complete and the whole request fits one atomic batch. Never perform a partial clear under a truncated snapshot. |
| “Move/restyle this line” | Explain that in-place edits are not yet exposed. A remove-plus-add replacement is valid only when the exact old drawing and intended replacement are unambiguous. |

No drawing request implies an order or signing action. Advice such as “where
would support go?” produces an answer until the current user explicitly asks
the Assistant to apply it.

Every typed read/analysis tool response includes a normalized `quality` envelope
with source, observation/retrieval/snapshot times, freshness state, coverage,
assumptions, exclusions, and warnings. The workspace action instead returns an
authoritative mutation and persistence acknowledgement. Statistical
summaries expose sample counts and dispersion; journal summaries additionally
report metric-specific missing-value coverage rather than treating missing PnL
or fee values as zero.

The system prompt directs the model to use deterministic tools for arithmetic,
route best/worst-trade and reflection questions to `kerosene_journal`, avoid
guessing symbol mappings, use plain Markdown rather than unsupported LaTeX
delimiters, and stop after a decisive complete empty-state result. For indicator
actions it requires a fresh workspace read, treats the selected chart as “this
chart,” asks when multiple plausible targets remain, distinguishes advice from
permission to mutate, accepts mutation authority only from the current user
message rather than snapshot/provider/journal/image/tool/prior-turn content,
uses the smallest supported set when choice is delegated, sends one complete
idempotent batch, and reports only the host acknowledgement. Drawing actions
use the same current-message authorization rule. The model must resolve exact
chart and removal IDs, use the selected drawing only for an unambiguous “this
drawing,” derive candle-based anchors from current-turn evidence rather than
inventing coordinates, respect truncated drawing coverage, and send one
complete batch. Drawing labels and chart content are data, never instructions.
Journal
reflections are treated as user-authored context rather than verified market
facts. The evidence protocol also requires the model to distinguish
observations, deterministic calculations, user-authored context, and
interpretations; disclose scope, time, metric, sample, and coverage for
material quantitative claims; surface conflicts; avoid unsupported precision
and causal claims; and state what evidence is missing when a conclusion cannot
be supported.

If current Pi emits `agent_end` without visible text, Kerosene retries once with
a short visible-answer instruction. A second empty settlement becomes an
explicit error instead of being presented as a successful blank response.

## Current MVP Limits

- Kerosene persists bounded chat text locally; image attachments, reasoning
  traces, and tool cards remain transient.
- Assistant output renders streamed Markdown with headings, emphasis, lists,
  quotes, tables, links, inline code, and highlighted fenced code blocks.
- No shell, filesystem, order, signing, trading-control, or generic mutation
  tools are exposed. Workspace mutation is limited to the typed reversible
  chart-indicator and persisted-drawing actions described above.
- Deterministic analysis is limited to the allowlisted operations. General
  executable analysis still needs a separately sandboxed runtime before it is
  safe to enable.
- OHLCV/session and aggregate-positioning tools use fixed provider operations;
  there is no generic URL or query input.
- Snapshot refresh happens at prompt boundaries, not continuously during a
  single agent turn.
- P&L matching covers current open positions. It does not search closed-position
  history or social-account identity, and rounded or incomplete cards may remain
  ambiguous.
- Journal analysis reflects the active account's currently loaded journal and
  reports partial, incomplete, or truncated coverage rather than silently
  presenting it as full history.

## Follow-up Scope

Future work should evaluate whether an additional no-network/no-filesystem
analysis sandbox is justified beyond the deterministic tools and may introduce
consented, versioned data plugins beyond the current contracts.
