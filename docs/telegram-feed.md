# Telegram Feed

Telegram Feed is a pane widget that shows recent posts from public Telegram
channels. It is designed for market-news monitoring inside Kerosene without
requiring Telegram login credentials, API keys, or private-channel access.

The implementation fetches Telegram's public web preview pages at
`https://t.me/s/<channel>`. It does not use MTProto, a bot token, a user
session, or push delivery.

## User-facing behavior

Open Telegram Feed from the add-widget menu or Alfred. The pane starts with the
default `@marketfeed` channel unless the user has persisted a different channel
list.

The controls provide:

- Channel input for adding a public channel username.
- `Add` action, also triggered by submitting the input.
- Alert toggle for new-message notifications.
- Manual refresh button.
- Channel chips with channel avatars or initials and a remove action.

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
When a post is first seen, Kerosene stores the current live mid as that ticker's
reference price. The chip then shows the live percentage move from that
reference to the latest live mid. Clicking a chip selects that symbol and opens
the primary chart when a chart pane is present.

News keywords can also map to related markets. Mentions of `oil`, `Iran`, or
`Hormuz` display `xyz:BRENTOIL` and `xyz:WTIOIL` when those markets are present
in the loaded symbol universe.

New messages are highlighted with a background color that cools down over
`TELEGRAM_NEW_MESSAGE_COOLDOWN_MS`, currently 120 seconds. Initial backfill is
quiet and does not fire alerts.

## Channel rules

Only public username channels are supported. Accepted inputs include:

- `marketfeed`
- `@marketfeed`
- `https://t.me/marketfeed`
- `https://t.me/s/marketfeed`

Private invite links and internal Telegram paths are rejected. Usernames must:

- Start with an ASCII letter.
- Be 5 to 32 characters long.
- Contain only ASCII letters, numbers, and `_`.

The channel list is normalized to lowercase and deduplicated.

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

Telegram Feed uses polling. It is not real-time push.

The background poll interval is `TELEGRAM_FEED_REFRESH_INTERVAL_SECS`, currently
15 seconds, and only runs while the Telegram Feed pane is open and no Telegram
feed refresh is already in flight.

Expected delivery latency is:

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
HTML polling path as a fallback. Users can toggle fast mode in the feed widget.

Fast mode requires a Telegram session. If the app is built with
`KEROSENE_TELEGRAM_API_ID` and `KEROSENE_TELEGRAM_API_HASH`, users only need to
enter their phone number and Telegram login code. Otherwise, the widget also
accepts a user-provided Telegram developer API ID and hash. The API hash is not
persisted in `config.json`.

Release builders should treat `KEROSENE_TELEGRAM_API_HASH` as embedded binary
credential material. Do not set it for public distributable builds unless the
bundled Telegram application credentials are explicitly intended to be public,
non-user-specific, and rotation-safe. When omitted, users can still provide
their own Telegram developer API ID and hash at login time.

The MTProto session is stored separately in the Kerosene config directory as
`telegram_fast.session` and is permission-tightened on Unix-like platforms.
Signing out from the widget clears that session file family.

Fast updates are additive: new MTProto posts go through the same `(channel,
message_id)` merge and dedupe path as public-page refreshes. Public polling
continues to run so existing no-login behavior remains available.

Public refresh, private-channel discovery, fast-auth, avatar, and media tasks
each have an exact runtime result owner. Their nonzero request allocators wrap
while skipping live owners, survive an in-process config clear, and settle once;
the fast-stream generation is advanced across that reset. Stale, replayed, or
pre-clear results are rejected before their result wrappers are opened. Public
and fast pages must also carry the same channel on the outer result, profile,
and every post before they can enter the shared merge path.

Fast cursor generations use the same wrapping invalidation rule. Removing a
channel or clearing config therefore prevents a late backfill/media task from
reinstalling an invalidated cursor, even at integer wrap.

Telegram only pushes channel updates to the signed-in account for channels it
receives updates for. For channels outside the account's update stream, the
public HTML polling path remains the fallback source.

## Persistence

The persisted configuration stores:

- `telegram_feed_channels`
- `telegram_feed_notifications_enabled`
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
metadata until the parser is updated. This is the main tradeoff of avoiding a
Telegram-authenticated MTProto client.

## Security and privacy

Public mode does not collect Telegram credentials and only fetches public
`t.me/s` pages for normalized usernames. It rejects private invite links and
private identifiers entered as public channels. Optional fast mode is the
separate authenticated integration described above: short-lived login inputs
remain runtime-only, while the MTProto session uses its dedicated restricted
session file family.

## Code map

Core implementation:

- `src/telegram_feed.rs`: state model, channel normalization, HTML parsing,
  HTTP fetches, timing labels, avatar validation, and parser tests.
- `src/telegram_fast_feed.rs`: optional MTProto auth, session handling,
  startup backfill, and live update streaming.
- `src/feed_update/telegram.rs`: update routing for refreshes, channel edits,
  fast-mode auth events, post merging, notifications, avatar request state, and
  update tests.
- `src/feed_views/telegram.rs`: pane controls, channel chips, post cards,
  avatar rendering, heat styling, and responsive layout.

Application wiring:

- `src/message.rs`: Telegram Feed messages.
- `src/feed_update.rs`: feed update dispatch.
- `src/feed_views.rs`: feed view dispatch.
- `src/subscription_state/timers/app.rs`: background polling timer.
- `src/config/schema.rs` and config persistence modules: persisted channels and
  notification toggle.
- `src/pane_state.rs`, `src/pane_update.rs`, `src/main_view/panes.rs`, and
  layout conversion modules: pane creation and layout persistence.
- `src/alfred_state/catalog/widgets.rs`: Alfred widget entry.
