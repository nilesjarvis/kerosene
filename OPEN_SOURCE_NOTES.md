# Open-Source Preparation Notes

_Last updated: 2026-05-08 09:45 CEST_

## Result

Status: **open-source feasible, with minor repository-hygiene caveats before publishing.**

The workspace has been cleaned of obvious personal identity markers, legacy public branding, generated artifacts, and hardcoded secret-like values. The project now has public-facing docs, MIT licensing metadata, and a security policy.

## Removed before release

Generated/local-only artifacts removed from the working tree:

- `target/`
- `AUDIT.md`
- `accounts_history.txt`
- `leverage.txt`
- `margin.txt`
- `margin_main.txt`
- `slider.txt`
- `check.log`
- `meta.json`
- `spot.json`
- `plan_draft.md`
- `plan_gradient.md`
- root patch/fix scripts: `patch*.py`, `patch.py`, `fix_*.py`
- root scratch/check artifacts: `check_ctxs`, `check_custom_theme_base`, `check_mids`, `check_pane_bg`, `check_symbol`, `explore`, `sim_iced`, `plan_test`
- empty/unneeded `hyperliquid-python-sdk/`
- generated binary `test_display`; source `tests/manual/test_display.rs` was kept

## Kept intentionally

- `src/**`: application source and integrated tests.
- `assets/**` and `branding/**`: Kerosene-branded project assets.
- `scripts/package.sh`: public packaging helper.
- `Cargo.toml` and `Cargo.lock`: Rust package metadata and reproducible dependency lockfile.
- `.gitignore`: expanded to keep generated/local artifacts out of future public commits.
- `README.md`: rewritten for Kerosene public release.
- `LICENSE`: MIT license text.
- `SECURITY.md`: basic security policy and secret-handling guidance.
- `.hermes/plans/2026-05-08_091838-open-source-kerosene.md`: local implementation plan. It is ignored by `.gitignore` and should not be included in a public repository unless intentionally published.

## Tests preserved for later review

The user requested that tests not be removed. Root-level test experiments were moved into `tests/manual/`:

- `tests/manual/test_canvas_event.rs`
- `tests/manual/test_canvas.rs`
- `tests/manual/test_display.rs`
- `tests/manual/test_divider.py`
- `tests/manual/test_focus.rs`
- `tests/manual/test_hover.rs`
- `tests/manual/test_keys.rs`
- `tests/manual/test_mouse_area.rs`
- `tests/manual/test_regex.py`
- `tests/manual/test_scroll.rs`
- `tests/manual/test_shrink.rs`
- `tests/manual/test_ws.js`

Recommendation before a polished public release: either integrate useful Rust tests into standard Cargo integration tests or keep them documented as manual GUI/websocket harnesses. Delete only after explicit approval.

## Branding and identity cleanup

Completed:

- Removed public-facing legacy branding from `README.md` and `AGENTS.md`.
- Removed old app config migration code from `src/config/files.rs` and `src/config/files/paths.rs`.
- Removed the historical `AUDIT.md` file.
- Replaced a public-looking wallet-address fixture with a synthetic placeholder address (`0xeeee...eeee`).
- Verified `.git` and `.github` are absent.

## Secret/identity scan results

Commands used for final audit excluded generated output, Hermes local planning metadata, this notes file, and `Cargo.lock` false-positive checksums.

Identity scan result:

- No personal names, local paths, personal GitHub remotes, LAN IPs, or old public branding matches were found outside excluded local metadata.

Secret scan result:

- No obvious hardcoded API token, bearer token, or private key literal was found.
- Remaining matches are expected secret-handling code paths, documentation, or synthetic test fixtures.
- Placeholder/test wallet addresses remain in tests, mostly repeated synthetic forms like `0xaaaa...`, `0xbbbb...`, `0xcccc...`, `0xdddd...`, `0xeeee...`, `0xabc...`, `0xdef...`, and `0x000...`.
- `Authorization` and bearer-header matches are runtime HyperDash API client code where auth is constructed from runtime input, not a committed token.
- `api_key`, `secret`, and `password` matches appear in expected secret-handling code paths and docs.

## Validation

Passed:

```sh
cargo fmt -- --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Result:

- `cargo check`: passed
- `cargo test`: passed, `505 passed; 0 failed`
- `cargo clippy --all-targets --all-features -- -D warnings`: passed

Clippy initially found existing style warnings. These were fixed without changing behavior.

## Open-source feasibility assessment

Current status: **ready to initialize/publish as an open-source repository after final human review.**

Green points:

- Public branding is Kerosene.
- MIT license and security policy are present.
- README is public-facing and documents risk/secrets.
- Generated artifacts and obvious local scratch/history files have been removed.
- Personal identity markers were not found by the final identity scan.
- Secret fields are generally modeled with `Zeroizing`, skipped serialization where appropriate, keychain/encrypted storage, and runtime-only bearer auth.
- Full Rust validation passes.

Remaining caveats before a polished public launch:

1. `tests/manual/test_*` files need review/integration, but were intentionally preserved.
2. `.hermes/` is ignored but present locally; do not include it in the public repository.
3. Optional Hydromancer/HyperDash integrations should be checked for public API terms/rate limits before announcement.
4. No GitHub repository, Git history, remote, commit, or push has been created from this cleaned workspace.
