//! Demand-driven L4 reconstruction, isolated from rendering and order execution.
use super::HydromancerStreamKey;
use super::manager::{
    HydromancerCommand, HydromancerRoutedMessage, HydromancerSubscriptionGuard,
    get_hydromancer_l4_manager,
};
use crate::hydromancer_api::l4_book::{
    FetchError, claim_snapshot_slot, decode_block, fetch_snapshot,
};
use crate::market_state::book_inspector::{
    Block, Book, InspectorEvent, InspectorStatus, MAX_ORDERS, Selection, Snapshot,
};
use crate::ws::WsStream;
use futures::{SinkExt, channel::mpsc};
use iced::window;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::sync::{broadcast, watch};
use tokio::time::Instant;

const STREAM_TIMEOUT: Duration = Duration::from_secs(15);
const PAINT_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub(crate) struct InspectorStreamConfig {
    pub(crate) key: HydromancerStreamKey,
    pub(crate) window_id: window::Id,
    pub(crate) run_id: u64,
    pub(crate) coin: String,
    pub(crate) selection: watch::Receiver<Selection>,
}

impl Hash for InspectorStreamConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.window_id.hash(state);
        self.run_id.hash(state);
        self.coin.hash(state);
    }
}

struct SessionError {
    message: String,
    permanent: bool,
}
impl From<String> for SessionError {
    fn from(message: String) -> Self {
        Self {
            message,
            permanent: false,
        }
    }
}

pub(crate) fn ws_book_inspector(config: &InspectorStreamConfig) -> WsStream<InspectorEvent> {
    let mut config = config.clone();
    Box::pin(iced::stream::channel(4, async move |mut output| {
        loop {
            while let Err(delay) = claim_snapshot_slot() {
                if output
                    .send(InspectorEvent::Status(InspectorStatus::Waiting(
                        delay.as_secs().saturating_add(1),
                    )))
                    .await
                    .is_err()
                {
                    return;
                }
                tokio::time::sleep(delay.min(Duration::from_secs(1))).await;
            }
            if output
                .send(InspectorEvent::Status(InspectorStatus::Synchronizing))
                .await
                .is_err()
            {
                return;
            }
            match run_session(&mut config, &mut output).await {
                Ok(()) => return,
                Err(error) => {
                    if output
                        .send(InspectorEvent::Status(if error.permanent {
                            InspectorStatus::Unavailable(error.message)
                        } else {
                            InspectorStatus::Recovering(error.message)
                        }))
                        .await
                        .is_err()
                    {
                        return;
                    }
                    if error.permanent {
                        std::future::pending::<()>().await;
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }))
}

async fn run_session(
    config: &mut InspectorStreamConfig,
    output: &mut mpsc::Sender<InspectorEvent>,
) -> Result<(), SessionError> {
    let (commands, mut incoming) = get_hydromancer_l4_manager(config.key.clone());
    let topic = format!("l4BookUpdates:{}", config.coin);
    let payload = serde_json::json!({"method": "subscribe", "subscription": {
        "type": "l4BookUpdates", "coins": [&config.coin], "merge": false
    }});
    commands
        .send(HydromancerCommand::Subscribe {
            topic: topic.clone(),
            payload: payload.clone(),
        })
        .map_err(|_| "L4 connection unavailable".to_string())?;
    let _guard = HydromancerSubscriptionGuard::new(commands, vec![(topic, payload)]);
    // A market-scoped frame proves the subscription is active. Fetching before
    // this boundary can lose orders between the snapshot and the subscription.
    let first = tokio::time::timeout(
        STREAM_TIMEOUT,
        next_block(&mut incoming, &config.coin, false),
    )
    .await
    .map_err(|_| "No L4 updates for this market; check L4 access".to_string())??;
    let mut book = synchronize(
        &mut incoming,
        &config.coin,
        first,
        fetch_snapshot(config.coin.clone(), config.key.api_key_for_task()),
    )
    .await?;
    let mut last_block = Instant::now();
    let mut paint = tokio::time::interval(PAINT_INTERVAL);
    paint.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            block = next_block(&mut incoming, &config.coin, true) => {
                let block = block?;
                if block.height > book.height() {
                    book.apply(block)?;
                    last_block = Instant::now();
                }
            }
            changed = config.selection.changed() => {
                if changed.is_err() { return Ok(()); }
                // The paint tick picks up the newest selection without restarting
                // the market stream or consuming another REST snapshot.
            }
            _ = paint.tick() => {
                if last_block.elapsed() >= STREAM_TIMEOUT {
                    return Err("L4 stream stalled; resynchronizing".to_string().into());
                }
                let selection = *config.selection.borrow_and_update();
                let page = book.page(selection);
                let now = crate::ws::telemetry::now_ms();
                if now.saturating_sub(page.timestamp_ms) > STREAM_TIMEOUT.as_millis() as u64
                    || page.timestamp_ms > now.saturating_add(5_000) {
                    return Err("L4 data is stale; resynchronizing".to_string().into());
                }
                // UI backpressure can drop a derived page, never a book delta.
                if let Err(error) = output.try_send(InspectorEvent::Page(page))
                    && error.is_disconnected() { return Ok(()); }
            }
        }
    }
}

async fn synchronize(
    incoming: &mut broadcast::Receiver<HydromancerRoutedMessage>,
    coin: &str,
    first: Block,
    snapshot: impl std::future::Future<Output = Result<Snapshot, FetchError>>,
) -> Result<Book, SessionError> {
    let mut buffered = vec![first];
    let mut diff_count = buffered[0].diffs.len();
    tokio::pin!(snapshot);
    let snapshot = loop {
        tokio::select! {
            result = &mut snapshot => break result.map_err(|error| SessionError { message: error.message, permanent: error.permanent })?,
            block = next_block(incoming, coin, true) => {
                let block = block?;
                diff_count = diff_count.saturating_add(block.diffs.len());
                if buffered.len() >= 4096 || diff_count > MAX_ORDERS {
                    return Err("L4 bootstrap buffer overflow; resynchronizing".to_string().into());
                }
                buffered.push(block);
            }
        }
    };
    let mut book = tokio::task::spawn_blocking(move || Book::from_snapshot(snapshot))
        .await
        .map_err(|_| "L4 reconstruction interrupted".to_string())??;
    let snapshot_height = book.height();
    for block in buffered {
        book.apply(block)?;
    }
    // Do not label a REST snapshot as live until its successor has been applied.
    let catchup_deadline = Instant::now() + STREAM_TIMEOUT;
    while book.height() <= snapshot_height {
        let block = tokio::time::timeout_at(catchup_deadline, next_block(incoming, coin, true))
            .await
            .map_err(|_| "L4 stream stalled; resynchronizing".to_string())??;
        book.apply(block)?;
    }
    Ok(book)
}

async fn next_block(
    incoming: &mut broadcast::Receiver<HydromancerRoutedMessage>,
    coin: &str,
    initialized: bool,
) -> Result<Block, SessionError> {
    loop {
        let message = incoming
            .recv()
            .await
            .map_err(|_| "L4 stream interrupted or lagged; resynchronizing".to_string())?;
        if message.msg_type == "l4BookUpdates" {
            if let Some(coins) = message.data.get("coins") {
                let Some(coins) = coins.as_array() else {
                    return Err("Invalid L4 market scope".to_string().into());
                };
                if !coins.iter().any(|value| value.as_str() == Some(coin)) {
                    continue;
                }
            }
            return decode_block(&message.data, coin).map_err(SessionError::from);
        }
        if let Some(error) = control_error(&message, initialized) {
            return Err(error);
        }
    }
}

fn control_error(message: &HydromancerRoutedMessage, initialized: bool) -> Option<SessionError> {
    let kind = message.msg_type.as_str();
    let value = &message.data;
    if matches!(
        kind,
        "error" | "subscriptionResponse" | "subscriptionUpdate"
    ) {
        // Ignore rejections explicitly scoped to an unrelated subscription.
        if value
            .pointer("/subscription/type")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|scope| scope != "l4BookUpdates")
        {
            return None;
        }
        let failed = value
            .get("failed")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|failures| {
                failures.iter().any(|failure| {
                    failure.get("type").and_then(serde_json::Value::as_str) == Some("l4BookUpdates")
                        || failure
                            .pointer("/subscription/type")
                            .and_then(serde_json::Value::as_str)
                            == Some("l4BookUpdates")
                })
            });
        if kind == "error"
            || failed
            || value.get("success").and_then(serde_json::Value::as_bool) == Some(false)
            || value.get("error").is_some_and(|v| !v.is_null())
        {
            return Some(SessionError { message: "L4 subscription rejected. Check Hydromancer access and stream limits, then Retry.".into(), permanent: true });
        }
    }
    if matches!(kind, "disconnected" | "reconnecting")
        || (initialized && matches!(kind, "connected" | "reconnected" | "connecting"))
    {
        return Some(
            "L4 connection interrupted; resynchronizing"
                .to_string()
                .into(),
        );
    }
    None
}

#[cfg(test)]
mod tests;
