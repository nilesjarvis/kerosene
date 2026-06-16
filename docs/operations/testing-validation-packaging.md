# Testing, Validation, And Packaging

Kerosene is a Rust desktop trading app. Validation should be scaled to the risk
of the change: focused tests for narrow logic, broader checks when routing,
persistence, signing, market data, or shared models are touched.

## Fast Feedback

Use these during development:

```sh
cargo check
cargo test test_name
cargo test pattern -- --exact
cargo test --package kerosene --bin kerosene module::name::tests
```

For docs-only changes, Rust validation is usually not necessary unless links or
examples changed in code comments/doc tests.

## Standard Validation

Before merging significant code changes:

```sh
cargo fmt -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Run `cargo fmt` after Rust code edits.

## GUI Smoke Test

Linux headless smoke test:

```sh
timeout 20s xvfb-run -a cargo run
```

A timeout after the window starts is acceptable. A panic is not.

Use this when changing startup, iced settings, main shell, window routing,
canvas rendering, or platform packaging behavior.

## Focused Test Areas

| Change area | Useful tests |
| --- | --- |
| Message routing | `src/app_update/routing/tests/**` |
| Config schema/persistence | `src/config/tests/**`, `src/config_persistence/save/tests.rs` |
| Layout/panes/windows | `src/layout_persistence/**/tests`, `src/pane_interaction_update/tests.rs`, min-size tests |
| Market data/symbols | `src/market_state/**/tests`, `src/market_update/**/tests`, `src/api/**/tests.rs` |
| Order books | `src/market_update/order_book/**/tests`, `src/market_views/order_book/**/tests` |
| Charts | `src/chart_state/**/tests`, `src/chart_update/**/tests`, `src/chart/**/tests` |
| Screenshots | `src/chart_screenshot/tests/**` |
| Spaghetti/spread | `src/spaghetti/**/tests`, `src/spread_chart/**/tests` |
| Orders/signing | `src/order_execution/**/tests`, `src/order_update/**/tests`, `src/signing/**/tests` |
| Chase/TWAP | `src/order_execution/chase/**/tests`, `src/order_execution/twap/**/tests`, `src/twap_state/tests/**` |
| Account/wallet | `src/account/**/tests`, `src/account_update/**/tests`, `src/wallet_*` tests |
| Journal | `src/journal/**/tests`, `src/journal_views/**/tests` |
| Integrations | `src/ws/**/tests`, `src/hydromancer_api/tests.rs`, `src/hyperdash_*` tests, feed tests |
| Preferences/settings | `src/config/hotkeys/tests/**`, `src/preferences_update/**/tests`, `src/sound/tests.rs` |
| Risk filters | `src/risk_state/**/tests` |

## Manual Harnesses

`tests/manual/` contains development references and ad hoc harnesses. They are
not part of the normal `cargo test` suite.

Use them only as supplemental tools. Core behavior should have focused Rust
tests near the module being changed.

## Packaging Assets

Assets live under `assets/`:

- app icons
- screenshots
- bundled fonts
- sounds
- desktop file
- SVG/social/ticker assets

When adding assets:

- keep them under `assets/`
- update embedding/loading code if needed
- update packaging templates/scripts if the asset must ship
- avoid committing generated secrets or real account screenshots

## Linux Packaging

`scripts/package.sh` builds Linux packages:

```sh
./scripts/package.sh all
./scripts/package.sh deb
./scripts/package.sh rpm
./scripts/package.sh appimage
```

Outputs go under `target/`:

- Debian package under `target/debian/`
- RPM package under `target/rpm/`
- AppImage at `target/Kerosene-<version>-<arch>.AppImage`

The script installs missing Rust packaging tools where possible and uses
`Cargo.toml` package metadata.

Before building public release artifacts, confirm that
`KEROSENE_TELEGRAM_API_HASH` is unset unless the bundled Telegram application
credentials are explicitly approved for public distribution. If it is set, the
hash is compiled into the binary. Leaving it unset preserves the user-supplied
Telegram fast-mode login path.

## macOS Packaging

macOS packaging must run on macOS:

```sh
./scripts/package-macos.sh
./scripts/package.sh macos
```

It builds release, assembles `Kerosene.app`, writes plist metadata, renders
icons, ad-hoc signs, and creates a compressed DMG.

No Developer ID signing or notarization is performed by the default path.

## Windows Packaging

Windows packaging uses PowerShell:

```powershell
pwsh ./scripts/package-windows.ps1
```

The workflow builds the MSVC release binary, creates a portable zip, can sign
artifacts, can build a WiX MSI, and emits SHA256 sums.

Release builds should be Authenticode-signed when distributed.

`build.rs` generates Windows resources, icon handling, version metadata, and a
DPI-aware manifest.

## Dependency Changes

Before adding a dependency:

- check whether standard library or existing crates already cover it
- verify Linux/macOS/Windows compatibility
- understand packaging impact
- let Cargo update `Cargo.lock`
- add tests around any new parsing/crypto/network behavior

Do not hand-edit lockfile entries.

## Recommended Validation By Risk

Docs only:

- no Rust checks required unless examples affect code

UI-only layout/view:

- `cargo check`
- focused tests for helper logic if changed

Config/schema/persistence:

- `cargo test config`
- focused config/layout tests
- inspect snapshot behavior if secrets are nearby

Trading/signing/order automation:

- focused order/signing/Chase/TWAP tests
- account stale-data tests if touched
- `cargo test` when feasible

Websocket/transport:

- focused `ws` and integration tests
- consider GUI smoke if startup/subscription assembly changed

Packaging:

- build on the target platform when possible
- inspect package output paths and included assets
