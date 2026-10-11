mod http;
mod launch;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use k256::elliptic_curve::rand_core::{OsRng, RngCore};
use serde_json::{Value, json};
use tokio::net::TcpListener;

use crate::builder_codes::{BuilderFee, normalize_builder_address};

const SESSION_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const APPROVAL_HTML: &str = include_str!("../../assets/builder-approval.html");

/// Open an isolated browser session for main-wallet approval. Browser success
/// only requests an independent exchange approval check; it is not approval proof.
pub(crate) async fn approve_in_wallet(
    account_address: String,
    builder: BuilderFee,
) -> Result<(), String> {
    let account = normalize_builder_address(&account_address)
        .map_err(|_| "A valid main account is required for builder approval.".to_string())?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|_| "Could not start the local wallet approval page.".to_string())?;
    let host = listener
        .local_addr()
        .map_err(|_| "Could not read the wallet approval page address.".to_string())?
        .to_string();
    let mut random = [0_u8; 32];
    OsRng
        .try_fill_bytes(&mut random)
        .map_err(|_| "Could not securely initialize wallet approval.".to_string())?;
    let token = hex::encode(random);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "The system clock is invalid for wallet approval.".to_string())?
        .as_millis();
    let nonce = u64::try_from(nonce)
        .ok()
        .filter(|nonce| *nonce <= 9_007_199_254_440_991)
        .ok_or_else(|| "The system clock is invalid for wallet approval.".to_string())?;
    let session = ApprovalSession::new(host, token, &account, &builder, nonce);
    launch::open(&format!("{}{}", session.origin, session.path))?;

    tokio::time::timeout(SESSION_TIMEOUT, run_session(listener, &session))
        .await
        .map_err(|_| {
            "Wallet approval timed out. Check approval in Settings if you already signed."
                .to_string()
        })?
}

async fn run_session(listener: TcpListener, session: &ApprovalSession) -> Result<(), String> {
    loop {
        let (mut stream, peer) = listener
            .accept()
            .await
            .map_err(|_| "The local wallet approval page disconnected.".to_string())?;
        if !peer.ip().is_loopback() {
            continue;
        }
        // Requests run inside the session deadline, with no detached handlers.
        // An incomplete connection cannot keep this listener alive indefinitely.
        if let Ok(Some(completion)) = tokio::time::timeout(
            REQUEST_TIMEOUT,
            http::handle_connection(&mut stream, session),
        )
        .await
        {
            return match completion {
                http::Completion::Approved | http::Completion::Submitted => Ok(()),
                http::Completion::Cancelled => Err("Wallet approval was cancelled.".to_string()),
            };
        }
    }
}

struct ApprovalSession {
    host: String,
    origin: String,
    path: String,
    completion_path: String,
    token: String,
    html: String,
}

impl ApprovalSession {
    fn new(host: String, token: String, account: &str, builder: &BuilderFee, nonce: u64) -> Self {
        let path = format!("/{token}");
        let completion_path = format!("{path}/complete");
        let config = approval_config(account, builder, nonce, &completion_path);
        let html = APPROVAL_HTML
            .replace("__KEROSENE_NONCE__", &token)
            .replace("__KEROSENE_CONFIG__", &script_json(&config));
        Self {
            origin: format!("http://{host}"),
            host,
            path,
            completion_path,
            token,
            html,
        }
    }
}

fn approval_config(
    account: &str,
    builder: &BuilderFee,
    nonce: u64,
    completion_path: &str,
) -> Value {
    json!({
        "account": account,
        "feeBps": builder.fee_bps_text(),
        "expiresAt": nonce + SESSION_TIMEOUT.as_millis() as u64,
        "completionPath": completion_path,
        "action": {
            "type": "approveBuilderFee",
            "hyperliquidChain": "Mainnet",
            "maxFeeRate": builder.max_fee_rate(),
            "builder": builder.address(),
            "nonce": nonce,
        },
        "domain": {
            "name": "HyperliquidSignTransaction",
            "version": "1",
            "verifyingContract": "0x0000000000000000000000000000000000000000",
        },
        "primaryType": "HyperliquidTransaction:ApproveBuilderFee",
        "types": {
            "EIP712Domain": [
                {"name": "name", "type": "string"},
                {"name": "version", "type": "string"},
                {"name": "chainId", "type": "uint256"},
                {"name": "verifyingContract", "type": "address"},
            ],
            "HyperliquidTransaction:ApproveBuilderFee": [
                {"name": "hyperliquidChain", "type": "string"},
                {"name": "maxFeeRate", "type": "string"},
                {"name": "builder", "type": "address"},
                {"name": "nonce", "type": "uint64"},
            ],
        },
    })
}

fn script_json(value: &Value) -> String {
    value
        .to_string()
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

#[cfg(test)]
mod tests;
