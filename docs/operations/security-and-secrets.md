# Security And Secrets

Kerosene is trading software. It handles agent private keys, API keys, wallet
addresses, signatures, and account data. Secret material must stay out of logs,
plain config snapshots, screenshots, docs examples, and commits.

## Secret Types

Secret-bearing values include:

- Hyperliquid agent private keys
- Hydromancer API key
- HyperDash API key
- X OAuth access token, Client ID, and refresh token
- Telegram fast-mode login code/password/API hash while in memory
- Telegram API hash embedded at build time through
  `KEROSENE_TELEGRAM_API_HASH`
- encrypted secret password/confirmation inputs
- any future API token or signing key

Wallet addresses are not private keys, but they can identify a user. Avoid
printing real wallet addresses in tests/docs unless explicitly anonymized.

## Runtime Secret Handling

`app_state.rs` defines:

```rust
pub(crate) type SensitiveString = Zeroizing<String>;
```

Secret buffers and payloads use `Zeroizing<String>` so memory is cleared on
drop where practical.

Saved-account deletion moves the removed profile into a narrow rollback owner
instead of cloning profile credentials. A failed durable config save moves that
same owner back into account state; after a successful save, its agent and
legacy per-profile integration keys are scrubbed before keychain cleanup, which
receives only the profile secret ID. Encrypted deletion staging continues to
use zeroizing payload and plaintext buffers.

Wallet-address rebinding likewise moves the active profile key and key-input
buffer into one rollback owner. Credential persistence receives one short-lived
account snapshot after the active key has been removed. Failure restores the
original allocations; success scrubs the rollback keys and sensitive identity
copies immediately.

Explicit agent-key saving builds one caller-owned persisted-profile snapshot
with the draft key substituted directly, while the previous committed key
remains the signing authority. Failed storage drops that caller-staged copy;
successful storage moves the exact staged key buffer into committed profile
state rather than cloning it again. Backend-required payload and serialization
buffers remain short-lived zeroizing scopes inside the synchronous call.

Account switching performs same-profile, pending-request, Chase, and uncertain-
TWAP checks before capturing target credentials. A successful saved-profile
switch clones the canonical key once into a narrow target and moves that exact
allocation into the key-input owner. Rejected and ghost switches create no
target-key copy; ghost cleanup still scrubs any stray canonical key.

Keyed add-account submission keeps the window draft as failure authority and
places its one caller-owned key copy only in the credential-storage snapshot.
The provisional canonical profile contains metadata but no signing key, so an
immediate encrypted-config save can persist the profile atomically without
making it trading-capable before credential storage accepts it. Failure drops
the staged copy and preserves the exact draft allocation; success moves the
staged profile into canonical state. A first saved account also moves the
verified, normalized draft allocation into the key-input owner, while ordinary
switch-on-add continues through the post-gate account-switch capture.

Deferred legacy keychain loading constructs a lookup shell from the profile
secret ID and any existing per-profile Hydromancer fallback; it does not clone
the account's name, address, or canonical agent-key owner. A loaded agent-key
allocation moves into the canonical profile and is copied once for the active
key input. A newly migrated, normalized Hydromancer allocation moves into
global runtime state and is copied once for its input. Conflict, trimming,
persistence, and legacy-cleanup behavior remain owned by the existing migration
path.

Startup hydration treats an active profile's partial-bundle legacy fallback as
one agent/Hydromancer transaction. Its keychain lookup shell retains the exact
profile ID but omits unrelated metadata and canonical secret owners. When the
bundle has no global Hydromancer key, every unambiguous loaded value that
profile-wide cleanup may delete is placed in the attempted bundle first. Until
that write succeeds, the loaded buffers remain the exact plaintext config
fallbacks. When the bundle already has a global Hydromancer key, it retains its
established authority; choosing a different conflict policy requires an
explicit migration-behavior decision.

Switching from OS-keychain storage to encrypted config builds the candidate
payload directly from borrowed persisted-profile references, without a second
full profile snapshot. Legacy readers receive the exact profile ID plus
non-secret field-presence guards, not canonical names, wallet addresses, or
credential contents. Newly loaded agent and integration buffers move into the
candidate payload when no normalization is required; a required normalized
integration buffer becomes the owner instead. Keychain-cleanup and clear-config
tasks receive only identity shells. Existing bundle precedence, field-read
decisions, conflict handling, cleanup scope, and persistence ordering remain
unchanged.

Secret-bearing state includes:

- `wallet_key_input`
- `hydromancer_api_key`
- `hydromancer_key_input`
- encrypted secret password/confirmation buffers
- X OAuth token/client/refresh input and runtime state
- profile secret payloads

Do not clone secrets unnecessarily. When a task must own a key, keep the
ownership scope narrow.

## Storage Modes

Credential storage supports:

- OS keychain
- encrypted config

OS keychain mode stores profile/global secrets outside plaintext config.

Encrypted config mode stores an encrypted blob in `KeroseneConfig` using:

- Argon2id key derivation
- XChaCha20Poly1305 encryption
- random salt and nonce
- schema/version/cipher metadata

Encrypted mode requires unlock before secrets are available for use or update.

## Config Snapshot Rules

Plain config snapshots intentionally write empty secret fields:

- `agent_key`
- `hydromancer_api_key`
- `hyperdash_api_key`
- `x_access_token`
- `x_oauth_client_id`
- `x_refresh_token`
- `schwab_client_id` / `schwab_client_secret` / `schwab_access_token` /
  `schwab_refresh_token`
- `openrouter_api_key`

Saved account profiles persist secret IDs and wallet metadata, not raw agent
keys. Secret payloads map secret IDs to agent keys and global integration
tokens inside the selected secret storage backend.

## Ghost Wallets

Ghost wallets are in-memory only. They should not cause agent keys or ghost
secret state to be persisted. If a ghost account is active, journal/account
snapshot logic should avoid writing ghost-only secret-linked data where
appropriate.

## Signing Boundary

`src/signing/` is the only implementation boundary for signed Hyperliquid
exchange actions.

Rules:

- Do not implement ad hoc signing in feature modules.
- Do not log signing payloads, signatures, nonces with key context, or raw
  exchange requests if they could expose sensitive material.
- Order execution modules should pass keys into signing tasks through
  zeroizing-owned values.
- Tests should use known dummy keys or fixtures, not real keys.

Parsed exchange responses retain exact nested status values for lifecycle
classification, but every response-model `Debug` layer exposes only allowlisted
response-type metadata, status counts, and explicit redaction markers. A
type-only response summary emits the recognized protocol label or a
value-neutral marker; ordinary protocol copy remains unchanged.

The shared order-result classifier likewise retains its exact normalized status
for existing UI, cancellation checks, and reconciliation decisions. Formatting
an `ExecutionOutcome` exposes only its outcome kind and control flags; the
status is represented by a redaction marker so OIDs, fill size, fill price, and
sanitized-but-still-sensitive external copy do not become diagnostic output.
The stored status and every downstream consumer remain unchanged.

TWAP planning and activity messages also remain exact for the order-status UI,
live activity log, and terminal advanced-order history. Their independently
formattable runtime layers do not expose that text: `TwapPlannedSliceSkip` and
`TwapEvent` diagnostics retain kind, timing where applicable, and error state,
but represent the message with a redaction marker. This does not change event
storage, display, history serialization, scheduling, or retry behavior.

Leverage updates retain their exact input, symbol, account-correlation, asset,
margin-mode, and leverage values for validation, stale-result checks, signing,
and user feedback. Those values must not become a diagnostic payload: the Elm
input message uses `RedactedOrderInput`, the submission and pending-result
contexts implement value-redacting `Debug`, and the serialized leverage action
redacts its asset and leverage when formatted. This changes only diagnostic
representations; the signed wire fields and normal leverage behavior remain
unchanged.

## API Key Boundaries

Hydromancer, HyperDash, and X keys are only needed in:

- request tasks
- subscription setup
- secret persistence
- settings input/update flows

Saving or replacing keys should update secret storage and clear stale
connection/cache state when required. Hydromancer key rotation should evict old
websocket managers so old-key tasks stop.

## Release-Time Embedded Credentials

Kerosene can be built with optional Telegram fast-mode defaults through
`KEROSENE_TELEGRAM_API_ID` and `KEROSENE_TELEGRAM_API_HASH`. The API hash is
compiled into the binary when set. Public release builds should leave these
variables unset unless the bundled Telegram application credentials are
explicitly approved as public, non-user-specific, and rotation-safe. Without
bundled values, users can enter their own Telegram developer API ID and hash
when enabling fast mode.

## UI And Output Safety

Do not display secrets in:

- settings status messages
- toasts
- logs
- screenshots
- PnL card images
- chart screenshots
- test snapshots
- docs examples

When showing credential status, say where credentials are stored or what failed
without echoing the value.

## Filesystem Safety

Config paths use platform config directories. Imported asset file names are
validated before being referenced. Journal caches and Telegram session files
should use restrictive permissions where supported.

Do not accept arbitrary stored paths for future secret or asset features without
normalization and tests.

## Trading Risk Boundaries

Security also includes preventing unintended trades:

- close-position and NUKE require fresh account data
- hidden/muted positions should not be silently routed
- move-order replacement must not switch account/key after canceling the
  original order
- Chase/TWAP must respect account/key availability and market-type checks
- ambiguous order results require verification or refresh

Do not weaken these checks for UI convenience.

## Logging And Debugging

Safe to log:

- high-level status
- anonymized request IDs
- non-secret error strings
- counts and durations
- synthetic test addresses/keys

Do not log:

- private keys
- API keys
- bearer tokens
- encrypted secret passwords
- Telegram login codes/passwords
- real account dumps
- signed payloads or signatures from real accounts

When in doubt, redact.

## Tests To Check

Use focused tests in:

- `src/config/secrets/**/tests.rs`
- `src/secret_storage/**` tests where present
- `src/config/tests/**` for credentials omission
- `src/signing/tests/**`
- `src/order_execution/**/tests` for key/account safety
- `src/order_update/**/tests` for result verification
- `src/pnl_card/tests/privacy.rs`
- `src/journal/cache/tests.rs` for cache file behavior

For any storage or signing change, inspect generated config output and ensure
secret fields remain empty or encrypted.
