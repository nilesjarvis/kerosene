use k256::ecdsa::{RecoveryId, Signature, SigningKey, signature::hazmat::PrehashSigner};
use serde_json::Value;
use sha3::{Digest, Keccak256};

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
    let key_bytes = hex::decode(
        private_key_hex
            .strip_prefix("0x")
            .unwrap_or(private_key_hex),
    )
    .map_err(|e| format!("Invalid private key hex: {e}"))?;
    let signing_key = SigningKey::from_bytes(key_bytes.as_slice().into())
        .map_err(|e| format!("Invalid key: {e}"))?;

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
