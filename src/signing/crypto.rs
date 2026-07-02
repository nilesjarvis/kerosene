use k256::ecdsa::{RecoveryId, Signature, SigningKey, signature::hazmat::PrehashSigner};
use serde_json::Value;
use sha3::{Digest, Keccak256};
use zeroize::Zeroizing;

/// Parse an agent private key exactly the way order signing does, without
/// signing anything. A key accepted here cannot fail key decoding later in
/// `sign_l1_action`.
pub(crate) fn validate_agent_key(private_key_hex: &str) -> Result<(), String> {
    signing_key_from_hex(private_key_hex).map(|_| ())
}

/// Derive the agent wallet address for a private key so the user can
/// cross-check a pasted key against the API wallet shown by Hyperliquid.
pub(crate) fn agent_wallet_address_for_key(private_key_hex: &str) -> Result<String, String> {
    let signing_key = signing_key_from_hex(private_key_hex)?;
    let public_key = signing_key.verifying_key().to_encoded_point(false);
    let hash = keccak256(&public_key.as_bytes()[1..]);
    Ok(format!("0x{}", hex::encode(&hash[12..])))
}

fn signing_key_from_hex(private_key_hex: &str) -> Result<SigningKey, String> {
    let trimmed = private_key_hex.trim();
    let mut key_bytes = Zeroizing::new([0u8; 32]);
    hex::decode_to_slice(
        trimmed.strip_prefix("0x").unwrap_or(trimmed),
        key_bytes.as_mut(),
    )
    .map_err(|e| format!("Invalid private key hex: {e}"))?;
    SigningKey::from_bytes(key_bytes.as_slice().into()).map_err(|e| format!("Invalid key: {e}"))
}

/// Compute the action hash: keccak256(msgpack(action) ++ nonce_be_bytes ++ vault_flag
/// [++ expires_after_marker ++ expires_after_be_bytes]).
pub(super) fn action_hash_bytes(
    packed: &[u8],
    vault_address: Option<&str>,
    nonce: u64,
    expires_after: Option<u64>,
) -> Result<[u8; 32], String> {
    let mut data = Vec::with_capacity(packed.len() + 18);
    data.extend_from_slice(packed);
    data.extend_from_slice(&nonce.to_be_bytes());
    match vault_address {
        None => data.push(0x00),
        Some(addr) => {
            data.push(0x01);
            let addr_bytes = hex::decode(addr.strip_prefix("0x").unwrap_or(addr))
                .map_err(|e| format!("Invalid vault address hex: {e}"))?;
            if addr_bytes.len() != 20 {
                return Err(format!(
                    "Invalid vault address length: expected 20 bytes, got {}",
                    addr_bytes.len()
                ));
            }
            data.extend_from_slice(&addr_bytes);
        }
    }
    if let Some(expires_after) = expires_after {
        data.push(0x00);
        data.extend_from_slice(&expires_after.to_be_bytes());
    }
    Ok(keccak256(&data))
}

/// Construct the EIP-712 typed data hash for the "Agent" phantom type.
/// Domain: {name: "Exchange", version: "1", chainId: 1337, verifyingContract: 0x0}
/// Type: Agent(string source, bytes32 connectionId)
fn eip712_hash(phantom_agent_source: &str, connection_id: &[u8; 32]) -> [u8; 32] {
    let domain_type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let domain_sep = keccak256_abi(&[
        &domain_type_hash,
        &keccak256(b"Exchange"),
        &keccak256(b"1"),
        &uint256_bytes(1337),
        &address_bytes(&[0u8; 20]),
    ]);

    let agent_type_hash = keccak256(b"Agent(string source,bytes32 connectionId)");
    let struct_hash = keccak256_abi(&[
        &agent_type_hash,
        &keccak256(phantom_agent_source.as_bytes()),
        connection_id,
    ]);

    let mut final_data = Vec::with_capacity(66);
    final_data.extend_from_slice(b"\x19\x01");
    final_data.extend_from_slice(&domain_sep);
    final_data.extend_from_slice(&struct_hash);
    keccak256(&final_data)
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

fn keccak256_abi(parts: &[&[u8; 32]]) -> [u8; 32] {
    let mut data = Vec::with_capacity(parts.len() * 32);
    for p in parts {
        data.extend_from_slice(*p);
    }
    keccak256(&data)
}

fn uint256_bytes(val: u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[24..32].copy_from_slice(&val.to_be_bytes());
    buf
}

fn address_bytes(addr: &[u8; 20]) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[12..32].copy_from_slice(addr);
    buf
}

/// Sign an L1 action with the agent wallet private key.
pub(super) fn sign_l1_action(
    private_key_hex: &str,
    msgpack_bytes: &[u8],
    vault_address: Option<&str>,
    nonce: u64,
    expires_after: Option<u64>,
) -> Result<Value, String> {
    let signing_key = signing_key_from_hex(private_key_hex)?;

    let hash = action_hash_bytes(msgpack_bytes, vault_address, nonce, expires_after)?;
    let phantom_agent_source = "a"; // mainnet
    let digest = eip712_hash(phantom_agent_source, &hash);

    let (signature, recovery_id): (Signature, RecoveryId) = signing_key
        .sign_prehash(&digest)
        .map_err(|e| format!("Signing failed: {e}"))?;

    let r = hex::encode(&signature.to_bytes()[..32]);
    let s = hex::encode(&signature.to_bytes()[32..64]);
    let v = recovery_id.to_byte() + 27;

    Ok(serde_json::json!({
        "r": format!("0x{r}"),
        "s": format!("0x{s}"),
        "v": v
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // secp256k1 private key 0x…01 has the well-known Ethereum address below.
    const KEY_ONE: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";
    const KEY_ONE_ADDRESS: &str = "0x7e5f4552091a69125d5dfcb7b8c2659029395bdf";

    #[test]
    fn validate_agent_key_accepts_valid_key_with_and_without_prefix() {
        assert!(validate_agent_key(KEY_ONE).is_ok());
        assert!(validate_agent_key(KEY_ONE.strip_prefix("0x").unwrap()).is_ok());
        assert!(validate_agent_key(&format!("  {KEY_ONE}  ")).is_ok());
    }

    #[test]
    fn validate_agent_key_rejects_bad_hex_and_wrong_length() {
        let error = validate_agent_key("0xzz").expect_err("bad hex should fail");
        assert!(error.contains("Invalid private key hex"));

        let error = validate_agent_key("0x1234").expect_err("short key should fail");
        assert!(error.contains("Invalid private key hex"));

        let error = validate_agent_key(&format!("{KEY_ONE}00")).expect_err("long key should fail");
        assert!(error.contains("Invalid private key hex"));
    }

    #[test]
    fn validate_agent_key_rejects_zero_scalar() {
        let zero = format!("0x{}", "0".repeat(64));
        let error = validate_agent_key(&zero).expect_err("zero key should fail");
        assert!(error.contains("Invalid key"));
    }

    #[test]
    fn agent_wallet_address_matches_known_vector() {
        let address =
            agent_wallet_address_for_key(KEY_ONE).expect("valid key should derive an address");
        assert_eq!(address, KEY_ONE_ADDRESS);
    }

    #[test]
    fn agent_wallet_address_rejects_invalid_key() {
        assert!(agent_wallet_address_for_key("not-a-key").is_err());
    }
}
