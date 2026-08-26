# Assistant Sessions

The Kerosene Assistant supports multiple local chat sessions. The session
selector is the left column of the Assistant window; it lets the user create a
session and switch between saved sessions. The navigation can collapse to a
compact icon rail without changing or persisting session data. A session keeps
its draft, user and assistant messages, title, usage totals, resolved model,
and latest context measurement. In-progress turns cannot be switched so one Pi
runtime cannot write into another session.

## State And Runtime Boundaries

- `src/agent_state.rs` owns the active session plus the inactive session list,
  applies storage bounds, and builds the bounded transcript used to resume a
  saved conversation.
- `src/agent_update.rs` coordinates create/switch actions, stops the previous Pi
  process, requests Pi context metrics, and schedules session saves.
- `src/agent_views.rs` renders the collapsible session navigation, persistence
  status, prompt-bar model selector, and active session context footer.
- `src/agent_persistence.rs` loads and atomically saves the side-file.

Pi continues to run with `--no-session`. After an app restart, runtime exit, or
session switch, Kerosene gives the next Pi process a bounded copy of that
session's prior messages. The replay explicitly labels the content as chat
history and instructs Pi to use fresh Kerosene tools for current application
facts.

After Pi starts and after each settled turn, Kerosene requests Pi's RPC state
and session statistics. The footer shows the resolved model and context as
`used / available (percent)`. These are Pi's model context-window metadata and
its own context-token estimate, not cumulative API billing tokens. If Pi cannot
provide a trustworthy token estimate, including immediately after compaction,
the used value is shown as unknown rather than retaining a stale measurement.
Before the first runtime starts, the configured OpenRouter model or detected
local llama.cpp model is shown. A detected local model can provide its declared
context capacity immediately.

The provider/model label in the prompt bar is interactive. It allows switching
between OpenRouter and a compatible auto-detected local llama.cpp server. The
OpenRouter view remains searchable and shows current pricing and context. The
local view shows the verified loopback endpoint and advertised capabilities.
Changing provider or model shuts down the current Pi runtime and marks any saved
chat context for replay on the next turn. Changes are disabled while a turn is
in progress.

## Persistence Contract

Sessions are stored separately from `config.json` as
`assistant_sessions.json` in the platform config directory. The current wire
format is schema version 1. Saves use a temporary file and replacement, use
owner-only mode (`0600`) on Unix, and apply an owner-only ACL on Windows. The
store is limited to 32 MiB and 50 sessions; drafts, message counts, individual
messages, and replay context are also bounded.

Only user and assistant messages are durable. P&L card previews, image bytes,
and tool activity cards are transient UI state. The active
response's evidence drawer and follow-up suggestions are transient for the same
reason. Image-based turns require the card to be attached again before they can
be regenerated. Assistant Markdown is parsed again when a stored session is loaded; the
most recent restored assistant response still receives Copy and Regenerate
actions.

Chat content may include account information and trading intent. Persistence
types and completion messages therefore use redacted `Debug` implementations,
and Clear All Config removes both the session file and an interrupted-save temp
file.
