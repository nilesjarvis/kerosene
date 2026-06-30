# Security And Secrets

Kerosene is trading software. It handles agent private keys, API keys, wallet
addresses, signatures, and account data. Secret material must stay out of logs,
plain config snapshots, screenshots, docs examples, and commits.

## Secret Types

Secret-bearing values include:

- Hyperliquid agent private keys
- Hydromancer API key
- HyperDash API key
- X OAuth access token
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

Secret-bearing state includes:

- `wallet_key_input`
- `hydromancer_api_key`
- `hydromancer_key_input`
- encrypted secret password/confirmation buffers
- X OAuth token input/runtime state
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
