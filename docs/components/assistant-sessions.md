# Assistant Sessions

The Kerosene Assistant supports multiple local chat sessions. The session
selector is the left column of the Assistant window; it lets the user create a
session and switch between saved sessions. A session keeps its draft, user and
assistant messages, title, usage totals, resolved model, and latest context
measurement. In-progress turns cannot be switched so one Pi runtime cannot
write into another session.

## State And Runtime Boundaries

- `src/agent_state.rs` owns the active session plus the inactive session list,
  applies storage bounds, and builds the bounded transcript used to resume a
  saved conversation.
- `src/agent_update.rs` coordinates create/switch actions, stops the previous Pi
  process, requests Pi context metrics, and schedules session saves.
- `src/agent_views.rs` renders the session sidebar, persistence status, and the
  active session's model/context footer.
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
Before the first runtime starts, the configured OpenRouter model is shown and
context capacity remains unknown.

## Persistence Contract

Sessions are stored separately from `config.json` as
`assistant_sessions.json` in the platform config directory. The current wire
format is schema version 1. Saves use a temporary file and replacement, use
owner-only mode (`0600`) on Unix, and apply an owner-only ACL on Windows. The
store is limited to 32 MiB and 50 sessions; drafts, message counts, individual
messages, and replay context are also bounded.

Only user and assistant messages are durable. Tool activity cards are transient
UI state and are reconstructed only through a future tool call. Assistant
Markdown is parsed again when a stored session is loaded.

Chat content may include account information and trading intent. Persistence
types and completion messages therefore use redacted `Debug` implementations,
and Clear All Config removes both the session file and an interrupted-save temp
file.
