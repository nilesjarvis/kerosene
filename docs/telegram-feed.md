# Telegram Feed

Telegram Feed is a pane widget for monitoring channel posts inside Kerosene.
Public mode fetches Telegram's web preview pages at `https://t.me/s/<channel>`
without login credentials. Optional fast mode uses an authenticated MTProto
session for live updates and selected private channels.

## User-facing behavior

Open Telegram Feed from the add-widget menu or Alfred. First-run onboarding
offers public mode or Telegram sign-in. The default public channel is
`@marketfeed` unless the user has persisted a different channel list.

The controls provide:

- Channel input for adding a public channel username.
- `Add` action, also triggered by submitting the input.
- Alert toggle for new-message notifications.
- Manual refresh button.
- Channel chips with channel avatars or initials and a remove action.
- A private-channel picker when signed in to fast mode.
- An outcome-market toggle for ticker impact chips.

Each post shows:

- Channel avatar, title, and username.
- A live age label, such as `12.345 s ago`.
- For newly seen live posts, optional arrival latency in the form
  `seen +250 ms`, computed as local fetch completion time minus the Telegram
  send timestamp.
- Message text, with unsupported emoji removed before rendering.
- Clickable ticker impact chips when the text mentions Hyperliquid symbols.
- A link-copy action for the Telegram post URL.

Ticker impact chips are parsed from the loaded Hyperliquid symbol universe,
excluding spot markets.
Kerosene anchors the reference price to a mid recorded at or before the post's
publication time. If no sample exists, the current live mid is used only for
posts at most 90 seconds old; otherwise the reference remains unavailable.
Once captured, the reference survives later edits and mention refreshes. The
chip shows the percentage move from that reference to the latest live mid.
Clicking a chip selects that symbol and opens the primary chart when present.

News keywords can also map to related markets. Mentions of `oil`, `Iran`, or
`Hormuz` display `xyz:BRENTOIL` and `xyz:WTIOIL` when those markets are present
in the loaded symbol universe.

New messages are highlighted with a background color that cools down over
`TELEGRAM_NEW_MESSAGE_COOLDOWN_MS`, currently 120 seconds. Initial backfill is
quiet and does not fire alerts.

## Channel rules

The public-channel input accepts usernames and public links, including:

- `marketfeed`
- `@marketfeed`
- `https://t.me/marketfeed`
- `https://t.me/s/marketfeed`

Private invite links and internal Telegram paths are rejected. Usernames must:

- Start with an ASCII letter.
- Be 5 to 32 characters long.
- Contain only ASCII letters, numbers, and `_`.

The public channel list is normalized to lowercase and deduplicated. Private
channels are added separately from the signed-in account's channel scan. The
scan offers private channels already accessible to that account;
pasting an invite link does not join a channel.

## Loading and refresh flow

When the pane opens, Kerosene fetches the latest public posts for each configured
channel. Each channel request:

1. Normalizes the channel username.
2. Requests `https://t.me/s/<channel>`.
3. Parses the channel profile metadata.
4. Parses the latest public post blocks.
5. Keeps the newest `TELEGRAM_FEED_FETCH_LIMIT` posts per request, currently 10.
6. Stores fetch timing on each post.

The pane keeps up to `TELEGRAM_FEED_RENDER_LIMIT` posts in memory, currently
100, sorted newest first.

Manual refresh uses visible loading state, so the refresh button can show a
spinner and channel chips can show active loading color. Timer refreshes use a
background loading state so existing rows are not torn down or visually
repopulated every polling interval.

On successful refresh, existing posts are matched by `(channel, message_id)` and
updated in place for editable fields such as text, timestamp, and URL. Existing
fetch timing is preserved so the displayed arrival latency does not drift on
later refreshes. Only previously unseen message ids are inserted as new posts and
eligible for alerting.

## Polling and latency

Public mode uses polling. Fast mode receives MTProto updates and uses public
polling as a fallback for public channels.

The background poll interval is `TELEGRAM_FEED_REFRESH_INTERVAL_SECS`, currently
15 seconds while the Telegram Feed pane is open. The tick skips public fetching
while fast mode is connected and fresh, or a public refresh is already in
flight. A fast connection with no event for more than 90 seconds is marked
stale, restarted, and allowed to fall back to public fetching.

Expected public-mode delivery latency is:

```text
time until next poll + Telegram public page availability + HTTP request time
```

The best case is roughly the request duration after a post appears on the public
preview page. The common case is up to one poll interval plus request time. The
displayed `seen +latency` value measures the difference between Telegram's send
timestamp and the local fetch completion time for newly seen live posts.

Telegram's public HTML timestamps may only provide second-level precision, so
the send-time milliseconds can be `.000` even though Kerosene's local fetch
timestamp is millisecond precision.

## Optional fast mode

Telegram Feed also has an optional fast mode that signs in through Telegram's
MTProto user API and listens for Telegram updates while preserving the public
HTML polling path as a fallback. Users enter fast mode through the feed's
Connect flow and can return to public mode.

Fast mode requires a Telegram session. If the app is built with
`KEROSENE_TELEGRAM_API_ID` and `KEROSENE_TELEGRAM_API_HASH`, users only need to
enter their phone number and Telegram login code, plus a 2FA password if their
account requires one. Otherwise, the widget also accepts a user-provided
Telegram developer API ID and hash. The API hash is not persisted in
`config.json`.

Release builders should treat `KEROSENE_TELEGRAM_API_HASH` as embedded binary
credential material. Do not set it for public distributable builds unless the
bundled Telegram application credentials are explicitly intended to be public,
non-user-specific, and rotation-safe. When omitted, users can still provide
their own Telegram developer API ID and hash at login time.

The MTProto session is stored separately in the Kerosene config directory as
`telegram_fast.session` and is permission-tightened on Unix-like platforms.
Signing out from the widget clears that session file family.

Fast updates are additive: new MTProto posts go through the same `(channel,
message_id)` merge and dedupe path as public-page refreshes. The timer remains
active to detect stale fast connections; background public fetching resumes
when fast mode is disconnected or stale. Manual refresh still uses the public
path. Private channels require fast mode and have no public-page fallback.

Telegram only pushes channel updates to the signed-in account for channels it
receives updates for. For channels outside the account's update stream, the
public HTML path is available through manual refresh and the background
fallback conditions above.

## Persistence

The persisted configuration stores:

- `telegram_feed_channels`
- `telegram_feed_private_channels` (selected peer IDs and titles)
- `telegram_feed_notifications_enabled`
- `telegram_feed_include_outcome_markets`
- `telegram_feed_onboarding_dismissed`
- `telegram_feed_fast_mode_enabled`
- `telegram_feed_fast_api_id`

Runtime-only data is not persisted:

- Parsed posts.
- Channel profile metadata.
- Avatar image handles.
- In-flight avatar request ids.
- Loading state.
- Last refresh timing and errors.
- Telegram API hash and login form inputs.

Legacy configs without Telegram Feed fields default to `@marketfeed` with
notifications disabled.

## Notifications

Notifications are opt-in through the pane toggle. When enabled, Kerosene creates
toast notifications for new messages detected after the initial load.

To avoid a notification burst, only the first few new posts in a refresh produce
individual alert messages. Additional posts are summarized by count.

Initial load never alerts because those messages existed before the user started
the pane session.

## Avatars

Channel avatars are parsed from Telegram's public channel metadata. If a channel
has no usable avatar, the UI falls back to initials.

Avatar fetching is hardened separately from post fetching:

- Avatar responses are capped at `TELEGRAM_AVATAR_MAX_BODY_BYTES`, currently
  512 KiB.
- The response body must look like a supported raster image by file signature.
- Image handles are cached in runtime state so the view does not recreate image
  handles every render.
- Avatar results are accepted only when both the requested URL and request id
  still match the current channel profile.
- Failed avatar fetches use `TELEGRAM_AVATAR_RETRY_BACKOFF_MS`, currently five
  minutes, before another attempt.

## Text normalization

Telegram HTML is converted to plain text by:

1. Converting line breaks to newline characters.
2. Removing HTML tags.
3. Decoding common HTML entities.
4. Stripping emoji and emoji-joiner characters that the bundled fonts do not
   reliably render.
5. Normalizing whitespace per line.

Emoji-only text posts become empty after normalization and are skipped unless the
post has media fallback text such as `[photo]`.

## Error handling and limits

Post requests have a timeout of `TELEGRAM_FEED_REQUEST_TIMEOUT`, currently five
seconds. Public page responses are capped at `TELEGRAM_FEED_MAX_BODY_BYTES`,
currently 2 MiB.

Public channel lists are capped at `TELEGRAM_FEED_MAX_PUBLIC_CHANNELS`, currently
12 channels, to keep each refresh batch bounded. Existing configs with more
public channels still load, but only the first 12 normalized public channels are
used and the pane shows that extra saved channels were ignored.

Visible refresh errors are shown in the pane. Background refresh errors do not
replace a working feed with an error unless the feed has no posts yet. Removed
channels ignore late post and avatar responses.

If Telegram changes the public `t.me/s` HTML structure, parsing can fail or lose
metadata until the parser is updated. This limitation applies to the public
fetching path.

## Security and privacy

Public mode fetches public `t.me/s` pages without Telegram credentials. Fast
mode uses runtime login inputs and stores a local session separately from
`config.json`. Treat that session as credential material. API hashes, login
codes, and 2FA passwords use zeroizing buffers; debug output redacts sensitive
inputs and private feed content.

Private-channel selection persists peer IDs and titles in configuration. It
does not persist loaded posts or channel avatars. Sign-out attempts remote
revocation and removes the local session file family. If local removal fails,
the widget reports an error instead of marking the session signed out; if only
remote sign-out fails, it reports a warning after local cleanup succeeds.

## Code map

Core implementation:

- `src/telegram_feed.rs`: state model, redacted debug output, channel
  normalization, timing labels, and shared plain-text/image helpers. Public
  fetch functions are re-exported here for existing callers.
- `src/telegram_feed/client.rs`: public HTTP requests, bounded response reading,
  shared avatar/media fetching, and HTML parsing. Parser helpers borrow slices
  of the HTML before constructing owned model fields.
- `src/telegram_feed/tests.rs` and `src/telegram_feed/client/tests.rs`: model
  and parser tests. Client tests include local HTTP fixtures for image response
  validation, error precedence, and both size limits.
- `src/telegram_fast_feed.rs`: optional MTProto auth, session handling,
  startup backfill, and live update streaming.
- `src/feed_update/telegram.rs`: update routing, refreshes, channel edits, post
  merging, notifications, and avatar/media request state.
- `src/feed_update/telegram/fast.rs`: fast-mode auth requests/results,
  onboarding transitions, stream events, and sanitized status messages.
  Auth requests share generation, pending-state, and completion-message setup;
  each caller retains its validation and input cleanup order.
- `src/feed_update/telegram/tests.rs` and its child modules: update regressions,
  including auth request admission and ownership behavior. The fast module also
  has a local status-helper test.
- `src/feed_views/telegram.rs`: pane controls, channel chips, post cards,
  avatar rendering, heat styling, and responsive layout.

Application wiring:

- `src/message.rs`: Telegram Feed messages.
- `src/feed_update.rs`: feed update dispatch.
- `src/feed_views.rs`: feed view dispatch.
- `src/subscription_state/timers/app.rs`: background polling timer.
- `src/subscription_state/telegram.rs`: fast-feed subscription identity and
  admission.
- `src/config/schema.rs` and config persistence modules: persisted channels and
  feed preferences.
- `src/pane_state.rs`, `src/pane_update.rs`, `src/main_view/panes.rs`, and
  layout conversion modules: pane creation and layout persistence.
- `src/alfred_state/catalog/widgets.rs`: Alfred widget entry.
