use sha3::{Digest, Keccak256};

// ---------------------------------------------------------------------------
// TWAP Identifiers
// ---------------------------------------------------------------------------

pub(crate) fn twap_child_cloid(
    account_address: &str,
    twap_id: u64,
    started_at_ms: u64,
    slice_index: u32,
) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(account_address.as_bytes());
    hasher.update(twap_id.to_be_bytes());
    hasher.update(started_at_ms.to_be_bytes());
    hasher.update(slice_index.to_be_bytes());
    let hash = hasher.finalize();

    let mut cloid = String::with_capacity(34);
    cloid.push_str("0x");
    for byte in hash.iter().take(16) {
        use std::fmt::Write;
        let _ = write!(cloid, "{byte:02x}");
    }
    cloid
}
