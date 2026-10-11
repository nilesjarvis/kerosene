# Custom Builder Codes

Settings > Integrations > Builder Code configures one optional builder for new
orders placed through Kerosene. It is disabled by default. Save the builder's
wallet address and an explicit fee between 0.1 and 10 basis points, then enable
it. The common rate fits Hyperliquid's perpetual-market maximum; separate spot
rates and zero-fee attribution are not supported.

Approval is per trading account. **Check approval** reads Hyperliquid's
`maxBuilderFee`; **Approve in wallet** opens an expiring local browser page that
asks the user's main wallet to approve exactly the saved fee. The browser needs
an injected Ethereum wallet extension. An API-agent key cannot approve builder
fees. Saving or enabling settings does not sign an approval. A larger existing
approval never increases the configured charge.

Changes affect new orders and newly started automation. Existing Chase/TWAPs keep
the routing captured when started; stop them before changing their routing.
Disabling does not revoke the onchain approval. Existing resting orders are not
rewritten or cancelled. Users manage exchange approvals separately.

## Supported order paths

The common signing boundary attaches the builder to market, limit and IOC
orders, including ticket/preset orders, chart quick/HUD/Quick Trade actions,
position closes, NUKE, wallet-cluster legs, and client-side TWAP slices.
Perpetual trades, including HIP-3, charge the configured builder fee on both
sides. Spot and outcome buys do not incur builder fees; sells do.

Chase starts/adoption and manual order moves are rejected while builder routing
is enabled. Hyperliquid's documented `modify`/`batchModify` actions have no
builder field, and attribution retention has not been verified. Users can
cancel and submit a new order explicitly. Subaccount/vault orders are also
rejected with builder routing enabled until approval ownership/inheritance is
verified. These restrictions never block cancellation or leverage updates.
After disabling, normal modification becomes available; attribution on a
previously tagged order that is subsequently modified is not guaranteed.

Every wallet-cluster member must have a recent approval check before a batch is
admitted. Missing, insufficient or malformed approval rejects the batch before
any leg is dispatched. This does not make exchange execution atomic: subsequent
exchange errors or revocations may still cause partial fills/execution.

## Implementation boundaries

- `builder_codes/model.rs`: validated, normalized builder address and integer
  fee units (one unit = 0.1 bp = 0.001%). Parsing uses integer arithmetic.
- `builder_codes/api.rs`: native Hyperliquid approval reads, a bounded 60-second
  account/builder cache, explicit refresh, and batch admission checks. Cold
  placement checks have a three-second bound; Settings checks allow 20 seconds.
  Exchange rejection invalidates the cached approval. No request retries by
  dropping the builder.
- `settings_state/builder.rs`, `settings_update/builder.rs`, and
  `settings_views/integrations/builder.rs`: drafts, committed settings,
  account-scoped status and generation-checked callbacks.
- `signing/model/agent_key.rs`: immutable captured credential, target and builder
  routing. Invalid enabled settings remain a blocked placement policy, allowing
  cancellation while preventing an accidental untagged order.
- `signing/client.rs` and `signing/actions/`: builder preflight and the signed
  action field `builder: { b, f }`, after `grouping` in canonical MessagePack
  field order. Disabled routing preserves the original action bytes.

Local preflight failures are definitive rejections, not transport-unknown
results. They must not trigger reconciliation for an order never submitted.
They refresh account snapshots invalidated during placement admission. A TWAP
stops without consuming its slice when builder preflight fails.
Actual transport failures retain the existing client-order-ID reconciliation.
Settings changes do not change captured in-flight order or TWAP contexts.

Fee estimates include the builder charge where applicable. Hyperliquid's fill
`fee` already includes `builderFee`; realized P&L continues to count that total
once. Builder charges are not added again to journal or account fill totals.

## Browser approval security

`builder_codes/approval/` serves the embedded `assets/builder-approval.html` on
an ephemeral IPv4 loopback port for at most five minutes. Sessions have random
unguessable URLs, immutable account/builder/fee values, strict Host/Origin
checks, bounded requests, CSP, no remote scripts, and no caching. Private keys
never enter Kerosene. Signatures are sent by the browser directly to the fixed
official exchange endpoint, without `vaultAddress` or `expiresAfter`.

The main-wallet action uses EIP-712 `HyperliquidTransaction:ApproveBuilderFee`,
not the L1 Agent signing domain. Wallet account and chain are rechecked after
signing. The local completion callback is not trusted as proof of approval;
Kerosene independently rereads `maxBuilderFee` for the captured account and
settings generation. Ambiguous submission never automatically resubmits the
signed approval.

## Persistence and validation

`KeroseneConfig.builder_code` stores `{ enabled, address, fee_tenths_bps }`.
Missing fields default to disabled. Approval results, browser sessions and
draft input are runtime-only. Address-bearing Debug and Message values are
redacted; the builder address is intentionally stored as a non-secret setting.

Regression tests cover legacy config, exact fee conversion, canonical signing
bytes/hashes, disabled payload compatibility, account/config capture, stale
callbacks, cancellation, batch admission, automation snapshots, fee estimates,
loopback HTTP validation and browser approval payloads. Live wallet approval and
trade/fill attribution require a funded, explicitly authorized test environment;
unit tests do not place trades or sign with real credentials.

Focused validation commands:

```sh
cargo test builder
node --test tests/manual/builder-approval.test.mjs
```

The JavaScript harness executes the actual embedded page with mocked wallet and
HTTP providers. It covers typed data, account/chain changes, rejected signatures,
expiry, malformed responses and ambiguous submissions without wallet access.

## Protocol references

- [Builder codes](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/builder-codes):
  approvals, fee units, builder eligibility (100 USDC perps account value and
  standard account abstraction), and fee applicability.
- [Exchange endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/exchange-endpoint):
  order, modify and approval action shapes.
- [Info endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint):
  approval ceilings and inclusive fill fees.
- [HIP-4](https://hyperliquid.gitbook.io/hyperliquid-docs/hyperliquid-improvement-proposals-hips/hip-4-outcome-markets):
  outcome sell-side builder fees.
