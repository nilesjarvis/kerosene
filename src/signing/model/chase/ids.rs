use sha3::{Digest, Keccak256};
use std::fmt::Write as _;

// ---------------------------------------------------------------------------
// Chase Identifiers
// ---------------------------------------------------------------------------

pub fn chase_place_cloid(
    account_address: &str,
    chase_id: u64,
    started_at_ms: u64,
    place_attempt: u32,
) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(b"kerosene:chase-place");
    hasher.update(account_address.as_bytes());
    hasher.update(chase_id.to_be_bytes());
    hasher.update(started_at_ms.to_be_bytes());
    hasher.update(place_attempt.to_be_bytes());

    let digest = hasher.finalize();
    let mut cloid = String::with_capacity(34);
    cloid.push_str("0x");
    for byte in digest.iter().take(16) {
        let _ = write!(cloid, "{byte:02x}");
    }
    cloid
}
