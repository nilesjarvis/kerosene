//! Hydromancer L4 wire format: little-endian header, then Zstandard-compressed
//! MessagePack [coin, bids, asks], with orders [user, px, sz, oid].
use super::{HYDROMANCER_API_URL, KEROSENE_USER_AGENT};
use crate::api::{CLIENT, proxy::HyperliquidRequestExt};
use crate::market_state::book_inspector::{
    Block, Diff, MAX_ORDERS, RestingOrder, Snapshot, parse_decimal,
};
use serde::Deserialize;
use std::io::{Cursor, Read};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use zeroize::Zeroizing;

const MAX_RESPONSE: usize = 32 * 1024 * 1024;
const MAX_DECOMPRESSED: usize = 64 * 1024 * 1024;
// The endpoint permits ten snapshots per five minutes. This process-wide gate
// survives window closes, market changes and credential rotation.
const SNAPSHOT_SPACING: Duration = Duration::from_secs(31);
static NEXT_SNAPSHOT: Mutex<Option<Instant>> = Mutex::new(None);

pub(crate) fn claim_snapshot_slot() -> Result<(), Duration> {
    let mut next = NEXT_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
    reserve_snapshot_at(&mut next, Instant::now())
}

fn reserve_snapshot_at(next: &mut Option<Instant>, now: Instant) -> Result<(), Duration> {
    if let Some(delay) = next
        .and_then(|at| at.checked_duration_since(now))
        .filter(|delay| !delay.is_zero())
    {
        return Err(delay);
    }
    *next = Some(now + SNAPSHOT_SPACING);
    Ok(())
}

fn defer_snapshot_slot(delay: Duration) {
    let mut next = NEXT_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
    let at = Instant::now() + delay;
    *next = Some(next.map_or(at, |current| current.max(at)));
}

pub(crate) struct FetchError {
    pub(crate) message: String,
    pub(crate) permanent: bool,
}

impl From<String> for FetchError {
    fn from(message: String) -> Self {
        Self {
            message,
            permanent: false,
        }
    }
}

pub(crate) async fn fetch_snapshot(
    coin: String,
    api_key: Zeroizing<String>,
) -> Result<Snapshot, FetchError> {
    fetch_snapshot_from(coin, api_key, HYDROMANCER_API_URL).await
}

async fn fetch_snapshot_from(
    coin: String,
    api_key: Zeroizing<String>,
    url: &str,
) -> Result<Snapshot, FetchError> {
    // Extend the gate at actual dispatch: subscription setup can take seconds.
    defer_snapshot_slot(SNAPSHOT_SPACING);
    let mut response = CLIENT
        .post(url)
        .header(reqwest::header::USER_AGENT, KEROSENE_USER_AGENT)
        .bearer_auth(api_key.trim())
        .timeout(Duration::from_secs(20))
        .json(&serde_json::json!({"type": "l4Book", "coins": [coin]}))
        .send_info()
        .await
        .map_err(|_| "Hydromancer L4 request failed".to_string())?;
    let status = response.status().as_u16();
    if status == 429 {
        defer_snapshot_slot(retry_after(response.headers(), SystemTime::now()));
    }
    if !response.status().is_success() {
        return Err(FetchError { permanent: matches!(status, 400 | 401 | 403), message: match status {
            401 | 403 => "Hydromancer L4 access denied. Check the API key and L4 permissions in Settings > Integrations.".into(),
            400 => "Hydromancer does not support this L4 market request.".into(),
            404 => "No L4 snapshot available for this market yet.".into(),
            429 => "Hydromancer L4 snapshot rate limit reached.".into(),
            _ => format!("Hydromancer L4 request failed (HTTP {status})."),
        }});
    }
    if response
        .content_length()
        .is_some_and(|n| n > MAX_RESPONSE as u64)
    {
        return Err("L4 snapshot exceeds the response limit".to_string().into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Could not read L4 snapshot".to_string())?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE {
            return Err("L4 snapshot exceeds the response limit".to_string().into());
        }
        bytes.extend_from_slice(&chunk);
    }
    tokio::task::spawn_blocking(move || decode_snapshot(&bytes, &coin))
        .await
        .map_err(|_| "L4 snapshot decoder interrupted".to_string())?
        .map_err(FetchError::from)
}

fn retry_after(headers: &reqwest::header::HeaderMap, now: SystemTime) -> Duration {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .parse::<u64>()
                .ok()
                .map(Duration::from_secs)
                .or_else(|| {
                    chrono::DateTime::parse_from_rfc2822(value)
                        .ok()
                        .and_then(|date| {
                            SystemTime::from(date.with_timezone(&chrono::Utc))
                                .duration_since(now)
                                .ok()
                        })
                })
        })
        .unwrap_or(Duration::from_secs(300))
        .clamp(SNAPSHOT_SPACING, Duration::from_secs(3600))
}

#[derive(Deserialize)]
struct RawOrder {
    user: String,
    px: String,
    sz: String,
    oid: u64,
}

#[derive(Deserialize)]
struct RawMarket {
    coin: String,
    bids: Vec<RawOrder>,
    asks: Vec<RawOrder>,
}

fn order(raw: RawOrder, is_bid: bool) -> Result<RestingOrder, String> {
    let wallet = raw.user;
    if wallet.len() != 42
        || !wallet.starts_with("0x")
        || !wallet[2..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Invalid L4 wallet address".into());
    }
    let price = parse_decimal(&raw.px)?;
    let size = parse_decimal(&raw.sz)?;
    if size == 0 {
        return Err("Invalid zero-size resting L4 order".into());
    }
    Ok(RestingOrder {
        oid: raw.oid,
        wallet: wallet.to_ascii_lowercase().into(),
        is_bid,
        price,
        size,
    })
}

fn read_array<const N: usize>(cursor: &mut Cursor<&[u8]>) -> Result<[u8; N], String> {
    let mut bytes = [0; N];
    cursor
        .read_exact(&mut bytes)
        .map_err(|_| "Truncated L4 snapshot".to_string())?;
    Ok(bytes)
}

pub(crate) fn decode_snapshot(bytes: &[u8], coin: &str) -> Result<Snapshot, String> {
    if bytes.len() > MAX_RESPONSE {
        return Err("L4 snapshot exceeds the response limit".into());
    }
    let mut cursor = Cursor::new(bytes);
    let count = u32::from_le_bytes(read_array(&mut cursor)?);
    let height = u64::from_le_bytes(read_array(&mut cursor)?);
    let timestamp_ms = u64::from_le_bytes(read_array(&mut cursor)?);
    if count != 1 || height == 0 || timestamp_ms == 0 {
        return Err("L4 snapshot missing the requested market or block metadata".into());
    }
    let length = u32::from_le_bytes(read_array(&mut cursor)?) as usize;
    let start = cursor.position() as usize;
    if length != bytes.len().saturating_sub(start) {
        return Err("Invalid L4 snapshot length".into());
    }
    // Streaming decode bounds both output and the decoder's history window.
    let mut decoder = zstd::stream::read::Decoder::new(&bytes[start..])
        .map_err(|_| "Invalid L4 compression".to_string())?;
    decoder
        .window_log_max(26)
        .map_err(|_| "Invalid L4 compression window".to_string())?;
    let mut decoded = Vec::new();
    decoder
        .take(MAX_DECOMPRESSED as u64 + 1)
        .read_to_end(&mut decoded)
        .map_err(|_| "Invalid L4 compressed snapshot".to_string())?;
    if decoded.len() > MAX_DECOMPRESSED {
        return Err("L4 snapshot exceeds the decoded limit".into());
    }
    let mut de = rmp_serde::Deserializer::new(Cursor::new(decoded.as_slice()));
    de.set_max_depth(16);
    let market = RawMarket::deserialize(&mut de)
        .map_err(|_| "Invalid L4 MessagePack snapshot".to_string())?;
    if de.position() != decoded.len() as u64 || market.coin != coin {
        return Err("L4 snapshot market mismatch or trailing data".into());
    }
    if market.bids.len().saturating_add(market.asks.len()) > MAX_ORDERS {
        return Err("L4 book exceeds the supported order limit".into());
    }
    let orders = market
        .bids
        .into_iter()
        .map(|raw| order(raw, true))
        .chain(market.asks.into_iter().map(|raw| order(raw, false)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Snapshot {
        height,
        timestamp_ms,
        orders,
    })
}

#[derive(Deserialize)]
struct RawBlock {
    height: u64,
    timestamp: u64,
    diffs: Vec<RawDiff>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RawDiff {
    New {
        coin: String,
        oid: u64,
        user: String,
        side: String,
        px: String,
        sz: String,
        #[serde(rename = "insertBefore")]
        insert_before: Option<u64>,
    },
    Update {
        coin: String,
        oid: u64,
        sz: String,
    },
    Remove {
        coin: String,
        oid: u64,
    },
}

pub(crate) fn decode_block(value: &serde_json::Value, coin: &str) -> Result<Block, String> {
    let raw = RawBlock::deserialize(value.get("data").unwrap_or(value))
        .map_err(|_| "Invalid L4 stream update".to_string())?;
    if raw.height == 0 || raw.timestamp == 0 || raw.diffs.len() > MAX_ORDERS {
        return Err("Invalid L4 block metadata or update limit".into());
    }
    let mut diffs = Vec::new();
    for diff in raw.diffs {
        match diff {
            RawDiff::New {
                coin: market,
                oid,
                user,
                side,
                px,
                sz,
                insert_before,
            } if market == coin => {
                let is_bid = match side.as_str() {
                    "B" => true,
                    "A" => false,
                    _ => return Err("Invalid L4 order side".into()),
                };
                diffs.push(Diff::New(
                    order(RawOrder { user, px, sz, oid }, is_bid)?,
                    insert_before,
                ));
            }
            RawDiff::Update {
                coin: market,
                oid,
                sz,
            } if market == coin => diffs.push(Diff::Update(oid, parse_decimal(&sz)?)),
            RawDiff::Remove { coin: market, oid } if market == coin => {
                diffs.push(Diff::Remove(oid))
            }
            _ => {}
        }
    }
    Ok(Block {
        height: raw.height,
        timestamp_ms: raw.timestamp,
        diffs,
    })
}

#[cfg(test)]
mod tests;
