# Security And Secrets

Kerosene is trading software. It handles agent private keys, API keys, wallet
addresses, signatures, and account data. Secret material must stay out of logs,
plain config snapshots, screenshots, docs examples, and commits.

## Secret Types

Secret-bearing values include:

- Hyperliquid agent private keys
- Hydromancer API key
- Complete Hyperliquid proxy URLs, including optional username/password
- HyperDash API key
- OpenRouter API key
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
pub(crate) struct SensitiveString(Zeroizing<String>);
```

`SensitiveString` wraps a zeroizing buffer and redacts its `Debug` output.
Secret buffers and payloads use `Zeroizing<String>` so memory is cleared on
drop where practical.

Secret-bearing state includes:

- `wallet_key_input`
- `hydromancer_api_key`
- `hydromancer_key_input`
- encrypted secret password/confirmation buffers
- X OAuth token/client/refresh input and runtime state
- profile secret payloads

Do not clone secrets unnecessarily. When a task must own a key, keep the
ownership scope narrow.

The Kerosene Assistant passes owned OpenRouter and HyperDash keys to the Pi child
process through its environment. Keys must never appear in process arguments, the
assistant snapshot, RPC payloads, transcripts, or debug output. Pi runs with an
isolated config directory and no session persistence. Successful key rotation
invalidates both starting and connected runtimes so a child cannot retain the
previous key for the rest of the session. This transport does not isolate keys
from other processes running as the same OS user; on Linux, those processes may
be able to read the child's environment through `/proc`.

Assistant snapshots contain sensitive trading history even though credentials
and wallet addresses are omitted. Before starting any assistant tasks, startup
removes `snapshot.json` and staged `snapshot-<generation>-<request>.json` files
from `kerosene-agent-<pid>` temporary directories whose process has exited. It
also clears residue with the new process's reused PID. Live or inaccessible PIDs
are preserved. Unix cleanup checks directory ownership and rejects shared
writable directories; symlink directories and Windows reparse points are skipped.
Deletion is nonrecursive and limited to recognized snapshot files. Extension
files, Pi configuration, and unrecognized content remain untouched.

Cleanup is best effort: residue can remain between a crash and the next launch,
when permissions prevent deletion, or while a PID has been reused by another
live process. Unlinking is not secure erasure and cannot remove backups or
filesystem copies. Owner-only file permissions are not a boundary against
other processes running as the same user.

Kerosene itself persists bounded Assistant chats in the owner-only
`assistant_sessions.json` side-file. Chat content is sensitive account context,
not a secret credential: persistence types and save-result messages must redact
content from `Debug`, the side-file must never contain the OpenRouter key or raw
tool payloads, and Clear All Config must remove it.

Assistant P&L card files are untrusted, potentially identifying input. Decode
them with file, dimension, and allocation limits; normalize them in memory;
redact their paths from `Debug`; and never persist their bytes or preview
handles. Images are sent only to the selected vision-capable OpenRouter model.
The specialized matching tool may reveal bounded public wallet candidates only
when the current turn carries an explicit attachment authorization; it must not
turn those candidates into claims about a person's identity or ownership.

## Storage Modes

Credential storage supports:

- OS keychain
- encrypted config

OS keychain mode stores profile/global secrets outside plaintext config.

On Linux, Kerosene uses the D-Bus Secret Service backend. It does not provide
Kerosene with a per-application credential ACL: other applications in the same
login session may retrieve secrets from an unlocked collection. The
[Secret Service specification](https://specifications.freedesktop.org/secret-service/latest-single/)
does not mandate access control, and the
[GNOME Keyring security FAQ](https://wiki.gnome.org/Projects/GnomeKeyring/SecurityFAQ)
explicitly excludes protection against malicious applications reading an
unlocked keyring. Desktop sandboxing and service-specific policy may further
restrict access; do not assume that the "OS Keychain" label alone provides it.

Choose encrypted config when credentials should require a separate Kerosene
password before loading. This adds at-rest protection while locked; after
unlocking, credentials must still enter application memory and it does not
protect against a compromised user session.

Normal credential saves are scoped read-modify-write operations: the existing
bundle must be read successfully, and only the profile or integration named by
the user action is changed. A Hydromancer, HyperDash, X, OpenRouter, or
account-key save must never rebuild the whole bundle from runtime fields or
interpret an unrelated empty field as a clear. Full-bundle replacement is
reserved for explicit storage-mode migration and startup migration after all
source credentials have been read. If the current bundle cannot be read, the
update is rejected without writing.

Windows Credential Manager limits the size of one generic credential record.
Kerosene therefore stores the serialized payload as bounded, generation-based
chunks and commits a small manifest last. Loading validates the manifest and
reassembles every chunk before parsing. Existing single-record `secrets_v1`
credentials remain readable and are removed after a successful sharded write.
Never log chunk contents, names derived from user data, or the reassembled
payload.

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
- `openrouter_api_key`

Unknown credential fields are ignored when reading config and decrypted secret
payloads and omitted from subsequent serialization. This lets older bundles
load after an integration is removed while preserving supported credentials.

Saved account profiles persist secret IDs and wallet metadata, not raw agent
keys. Secret payloads map secret IDs to agent keys and global integration
tokens inside the selected secret storage backend.

Subaccount credentials bind the profile secret ID, effective child address,
and parent address. Missing, mismatched, or malformed parent metadata cannot
load a key saved for a subaccount into a main-account profile. Recovery keeps
the parent binding; legacy unbound credentials are not migrated into child
profiles. `CapturedAgentKey` owns the zeroizing key and redacted exchange target
together so asynchronous operations cannot lose their subaccount identity.

Hyperliquid API wallets are approved by the parent and may sign for its
subaccounts. A separate API wallet per process/session avoids shared nonce
collisions; Kerosene allocates unique nonces across its local signing requests.
See [Hyperliquid API wallets](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/nonces-and-api-wallets).

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

## API Key Boundaries

Hydromancer, HyperDash, and X keys are only needed in:

- request tasks
- subscription setup
- secret persistence
- settings input/update flows

Saving or replacing keys should update secret storage and clear stale
connection/cache state when required. Hydromancer key rotation should evict old
websocket managers so old-key tasks stop.

Hydromancer WebSocket authentication currently uses a `token` query parameter
over `wss://`. In-process URLs and errors are redacted, but TLS-terminating
services can see the handshake URL and may log it. Do not replace this with
header authentication without confirming provider support.

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

Opt-in desktop notifications send trading details to the operating system's
notification service, including user-chosen wallet labels, symbols, sides,
prices, and interest totals. Notification history or lock-screen previews may
retain or display this context. Disable desktop notifications when this is not
appropriate; in-app toasts remain available.

## Filesystem Safety

Config paths use platform config directories. Imported asset file names are
validated before being referenced. Journal caches and Telegram session files
should use restrictive permissions where supported. Config snapshots and
Telegram session files use owner-only modes on Unix and protected owner-only
ACLs on Windows; permission hardening happens before config bytes are written.

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

## Proxy Credentials

`api::proxy::ProxyUrl` wraps a zeroizing URL with redacted Debug output. Settings
use `SecretInput` and a masked input; list labels omit usernames and passwords.
Only the keychain/encrypted `SecretPayload` serializes URLs. Proxy transport
errors and failure response bodies are replaced with fixed errors or HTTP status
codes to prevent credential reflection. HTTPS verification remains enabled and
proxy-client redirects are disabled. See [Hyperliquid Proxies](hyperliquid-proxies.md).
