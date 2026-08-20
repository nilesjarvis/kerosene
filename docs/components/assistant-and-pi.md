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
| `src/agent_pnl_card.rs` | Bounded image selection, validation, normalization, preview, and redacted transport types. |
| `src/agent_state.rs` | Window, transcript, runtime status, streaming, tool cards, and redacted prompt wrapper. |
| `src/agent_update.rs` | Window lifecycle, prompt submission, snapshot/runtime orchestration, and stale-generation guards. |
| `src/agent_views.rs` | Native chat window, composer, status, usage, empty state, and tool activity UI. |
| `src/agent_snapshot.rs` | Versioned, bounded, sanitized read-only export of Kerosene state. |
| `src/agent_runtime.rs` | Pi subprocess discovery, isolated environment, JSONL RPC transport, and event parsing. |
| `assets/agent/kerosene.ts` | Embedded Pi extension, typed read-only tools, deterministic calculations, and fixed-provider data adapters. |

The assistant opens from the Widgets menu or the OpenRouter section of
Settings > Integrations. Closing the window terminates Pi, discards unsent image
attachments, and deletes the sensitive snapshot. Kerosene persists its bounded
chat sessions separately, while Pi uses `--no-session` and never writes them to
its own session store.

The model name in the Assistant footer opens a searchable picker backed by
OpenRouter's live model catalog. Only text-output models that advertise tool
calling are listed. Rows show OpenRouter's current input/output token prices,
context capacity, provider, image-input support, and whether conditional pricing applies. Selecting a model
uses the existing persisted OpenRouter default and restarts Pi before the next
turn.

## P&L Card Investigation

The empty state and composer expose a P&L card action. Users can choose a PNG,
JPEG, or WebP file, or drag one anywhere over the Assistant window on platforms
where iced supports file-drop events. Kerosene decodes the image with strict
file, dimension, and allocation limits, resizes it to at most 2000×2000, and
normalizes it to an in-memory PNG. The preview and image bytes are transient and
are never written to `assistant_sessions.json`.

Attaching a card filters the existing OpenRouter picker to models that advertise
both image input and tool calling. Pi receives the normalized image through its
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

Completed responses expose Copy and Regenerate actions, a collapsible summary
of the actual `kerosene_*` data calls made during that turn, and two deterministic
follow-up suggestions based on the tool categories used. Follow-up selection
fills the composer for review instead of sending immediately. These presentation
controls and tool summaries are transient; persisted session content remains
limited to user and assistant messages.

Opening the Assistant while an account is connected also makes it an active
journal-data consumer. Kerosene immediately hydrates any local journal cache and
starts the normal incremental journal sync even when the Trading Journal window
is closed. A prompt snapshot can therefore use cached trades while a refresh is
in progress, with partial coverage reported explicitly.

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
extension's fixed aggregate-positioning and attachment-gated P&L matching
queries. Pi has no shell, file-read, or
generic network tool with which to inspect either environment value. Runtime
errors additionally redact both keys. Changing an integration key or model
terminates or invalidates the relevant active session/request generation.

The enabled tool allowlist is:

- `kerosene_data`
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

Each prompt first writes a fresh `schema_version: 3` JSON snapshot to a
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
- `journal`
- `positioning`
- `sessions`
- `all`

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

Every typed tool response includes a normalized `quality` envelope with source,
observation/retrieval/snapshot times, freshness state, coverage, assumptions,
exclusions, and warnings. Statistical summaries expose sample counts and
dispersion; journal summaries additionally report metric-specific missing-value
coverage rather than treating missing PnL or fee values as zero.

The system prompt directs the model to use deterministic tools for arithmetic,
route best/worst-trade and reflection questions to `kerosene_journal`, avoid
guessing symbol mappings, use plain Markdown rather than unsupported LaTeX
delimiters, and stop after a decisive complete empty-state result. Journal
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

- Kerosene persists bounded chat text locally; image attachments and tool cards
  remain transient.
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
