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
| `assets/agent/kerosene.ts` | Embedded Pi extension and the single `kerosene_data` tool. |

The assistant opens from the Widgets menu or the OpenRouter section of
Settings > Integrations. Closing the window terminates Pi, clears the transcript,
and deletes the sensitive snapshot. Sessions use Pi's `--no-session` mode and
are never written to Pi's session store.

## Runtime Contract

Kerosene starts Pi with:

- RPC mode over stdin/stdout JSONL
- the `openrouter` provider and configured model
- session persistence disabled
- project resources unapproved
- a strict `kerosene_data` tool allowlist
- an isolated `PI_CODING_AGENT_DIR`
- version checks and telemetry disabled

The OpenRouter key is passed only in the child environment as
`OPENROUTER_API_KEY`; it is never placed in arguments, snapshot content, RPC
messages, or debug output. Changing the OpenRouter key or model terminates the
active assistant session so a child cannot continue with stale credentials.

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

Each prompt first writes a fresh `schema_version: 1` JSON snapshot to a
per-process temporary directory. On Unix, the directory and file use owner-only
permissions. The extension reads that file only when the model calls
`kerosene_data`.

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

The snapshot explicitly omits:

- API and private keys
- wallet addresses
- order IDs
- transaction hashes
- wallet-level HyperDash identities

Errors are represented as booleans instead of raw upstream messages because
those messages can contain sensitive request context.

## Current MVP Limits

- Pi must be installed or bundled separately.
- Chat history is ephemeral and is cleared when the window closes.
- Assistant output is rendered as wrapped plain text; rich Markdown/code-block
  rendering is a follow-up.
- No shell, filesystem, order, signing, or mutation tools are exposed. Pi can
  still write code or calculations in its response, but executable analysis
  needs a separately sandboxed tool before it is safe to enable.
- Snapshot refresh happens at prompt boundaries, not continuously during a
  single agent turn.

## Follow-up Scope

The next production increment should pin and package Pi binaries for macOS,
Windows, and Linux; add a sandboxed analysis runtime; add richer Markdown/code
rendering; cover packaging smoke tests; and optionally introduce consented,
versioned data plugins beyond the MVP sections.
