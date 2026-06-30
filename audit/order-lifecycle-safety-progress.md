# Order Lifecycle Safety Progress

## Completed

- Redacted `PlaceOrderRequest` debug output so order price, size, and CLOID do not leak through debug formatting.
- Reclassified mixed exchange responses with both order effects and errors as ambiguous, and forced account refresh for those responses.
- Added durable pending status state for uncertain cancel and move operations:
  - Cancel status checks keep exact account, oid, and symbol context after the visual pending indicator is cleared.
  - Move status checks are armed only after ambiguous or transport-unknown modify results.
  - Account changes and disconnects are blocked while these status checks are pending.
  - Complete account refresh clears pending cancel/move status checks for that account.
- Treated filled order responses without an oid as ambiguous instead of fully settled, and displayed malformed status oids as `?` instead of `0`.
- Preserved Chase history overfills when authoritative fill metrics are unavailable, so archived filled size can exceed target size instead of being capped.
- Blocked account changes while a non-terminal TWAP has a reconciliation deadline, even if no status-check CLOID or child status marker is present.

## Validation

- `cargo fmt -- --check` passed.
- `git diff --check` passed for scoped order-lifecycle files.
- Focused Rust test execution is blocked in this environment because `alsa-sys` cannot find the system `alsa.pc` package through `pkg-config`.

## Remaining Findings

- No remaining findings from the completed order-lifecycle audit passes.
