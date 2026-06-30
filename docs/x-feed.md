# X Feed

X Feed is a pane widget for authenticated X timeline monitoring. It is designed
for BYOK: the user supplies an OAuth 2.0 user-context access token locally, and
Kerosene uses it to read the authenticated account's following timeline and X
Lists.

The first implementation uses REST polling because the X endpoints that cover
the requested sources are user-context timeline endpoints:

- following/home timeline: `GET /2/users/{id}/timelines/reverse_chronological`
- owned/followed List discovery:
  `GET /2/users/{id}/owned_lists` and `GET /2/users/{id}/followed_lists`
- List posts: `GET /2/lists/{id}/tweets`

## Widget Model

X Feed is multi-instance. Runtime panes use `PaneKind::XFeed(XFeedId)`, and
per-widget state is stored in `TradingTerminal.x_feed.instances`.

Each widget has its own selected source:

- `Following`
- `List { id, name, private }`

Multiple widgets can select the same source. Refreshes are deduped by source so
one REST response updates every matching widget.

## Authentication

The BYOK path accepts a user access token in the widget. The token is staged in
runtime state as `SensitiveString`, authenticated against X, then saved through
the selected credential store: OS keychain or encrypted config. It is not written
as plaintext to `config.json`. Persisted config stores widget IDs and selected
non-secret sources in `x_feeds`; private List selections fall back to
`Following` before persistence.

On startup, a saved token is restored from the secret payload and the app starts
an auth/list refresh for any open X Feed widgets. Clearing the token removes it
from the selected secret store before clearing runtime state.

Production OAuth should use X OAuth 2.0 Authorization Code Flow with PKCE and
scopes such as `tweet.read`, `users.read`, `list.read`, and `offline.access`
when refresh tokens are needed. Access tokens and refresh tokens must remain
local and redacted in debug output.

## Latency

Polling interval defaults to 10 seconds while any X Feed pane is open. Polls
request up to 10 posts to keep X API usage cost-sensitive. Requests use
`since_id` based on the newest post already seen for the selected source.

X Filtered Stream is not used for this initial widget because it is app-context,
public-content filtering and does not represent the authenticated user's private
following timeline or private Lists. It can be added later as an optional public
watch source for lower latency on public accounts/rules.

## Files

- `src/x_feed.rs`: state model, X REST client, response parsing, dedupe helpers.
- `src/feed_update/x.rs`: token handling, auth/list refreshes, feed polling.
- `src/feed_views/x.rs`: token controls, source picker, post cards.
- `src/message.rs`: X feed messages and redacted result wrappers.
- `src/config/panes/x_feed.rs`: persisted widget source config.
- `src/layout_persistence/x_feeds.rs`: saved layout restore.
