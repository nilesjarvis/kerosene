# X Feed

X Feed is a pane widget for authenticated X timeline monitoring. It is designed
for BYOK: the user supplies OAuth 2.0 user-context credentials locally, and
Kerosene uses them to read the authenticated account's following timeline and X
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

The BYOK path accepts either a user access token or a Client ID plus refresh
token in the widget. Pasted values are staged in runtime state as
`SensitiveString`. Refresh credentials are exchanged through
`POST /2/oauth2/token` before the app authenticates against X; if X rotates the
refresh token, the rotated value replaces the stored one. Credentials are saved
through the selected credential store: OS keychain or encrypted config. They are
not written as plaintext to `config.json`. Persisted config stores widget IDs
and selected non-secret sources in `x_feeds`; private List selections fall back
to `Following` before persistence.

On startup, saved X credentials are restored from the secret payload. If a
refresh token is available, the first X request refreshes the access token before
auth/list/feed calls. Clearing X credentials removes the access token, Client
ID, and refresh token from the selected secret store before clearing runtime
state.

Direct-token authentication and refresh-token exchange share one runtime-only
credential-operation owner, and only the exact current owner may recover a
result. A newly started token refresh supersedes an older read-only auth check;
an already-dispatched token refresh remains authoritative because its response
may rotate the refresh token, so later Connect attempts stay deduped until it
settles. The refresh retains its dispatch-time Client ID and fallback refresh
token in a redacted, zeroizing request context, so repeated input cannot retarget
the active response. Credential clear drops the owner and all loading flags;
in-process config clear preserves only the terminal request allocator so an old
task cannot alias the first post-clear request. Restart begins a fresh allocator
because no task survives process exit. Accepted errors pass a final sensitive-
text redaction boundary without changing ordinary error text.

Production OAuth should use X OAuth 2.0 Authorization Code Flow with PKCE and
scopes such as `tweet.read`, `users.read`, `list.read`, and `offline.access`
when refresh tokens are needed. Access tokens, Client IDs, and refresh tokens
must remain local and redacted in debug output.

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
- `src/feed_update/x.rs`: credential ownership, token handling, auth/list
  refreshes, feed polling.
- `src/feed_views/x.rs`: token controls, source picker, post cards.
- `src/message.rs`: X feed messages and redacted result wrappers.
- `src/config/panes/x_feed.rs`: persisted widget source config.
- `src/layout_persistence/x_feeds.rs`: saved layout restore.
