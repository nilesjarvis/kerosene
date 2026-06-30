# Order Lifecycle Safety Progress

## Completed

- Redacted `PlaceOrderRequest` debug output so order price, size, and CLOID do not leak through debug formatting.
- Reclassified mixed exchange responses with both order effects and errors as ambiguous, and forced account refresh for those responses.
- Added durable pending status state for uncertain cancel and move operations:
  - Cancel status checks keep exact account, oid, and symbol context after the visual pending indicator is cleared.
  - Move status checks are armed only after ambiguous or transport-unknown modify results.
  - Account changes and disconnects are blocked while these status checks are pending.
  - Complete account refresh clears pending cancel/move status checks for that account.

## Validation

- `cargo fmt -- --check` passed.
- `git diff --check` passed for scoped order-lifecycle files.
- Focused Rust test execution is blocked in this environment because `alsa-sys` cannot find the system `alsa.pc` package through `pkg-config`.

## Remaining Findings

- Filled placement responses without an oid are still treated as non-ambiguous.
- Advanced Chase history can hide overfills when fill metrics are unavailable.
- TWAP account-change blocking could explicitly include reconciliation deadlines for defense in depth.
