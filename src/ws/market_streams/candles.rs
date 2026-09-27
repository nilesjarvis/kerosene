use crate::api::Candle;
use crate::ws::{SubscriptionGuard, WsCommand, WsStream, get_manager};

use super::{KeyedCandleStreamEvent, SpaghettiCandleStreamEvent};
use futures::SinkExt as _;
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// Candle Streams
// ---------------------------------------------------------------------------

enum CandleStreamEvent {
    Item(Candle),
    Lagged { skipped: u64 },
    Unavailable(String),
}

fn ws_candle_stream(params: &(String, String)) -> WsStream<CandleStreamEvent> {
    let coin = params.0.clone();
    let interval = params.1.clone();

    Box::pin(iced::stream::channel(128, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_manager();

        let topic = format!("candle:{}:{}", coin, interval);
        let payload = serde_json::json!({
            "method": "subscribe",
            "subscription": {
                "type": "candle",
                "coin": coin,
                "interval": interval,
            }
        });
        let subscription = (topic.clone(), payload.clone());

        if cmd_tx
            .send(WsCommand::Subscribe {
                topic: topic.clone(),
                payload,
            })
            .is_err()
        {
            return;
        }
        let reconnect_tx = cmd_tx.clone();
        let _guard = SubscriptionGuard {
            cmd_tx,
            subscriptions: vec![subscription],
        };

        let mut watchdog = crate::ws::market_streams::CandleWatchdog::new(&coin, &interval);
        loop {
            let Some(message) = watchdog.recv(&mut msg_rx).await else {
                let _ = reconnect_tx.send(WsCommand::Resubscribe {
                    topic: topic.clone(),
                });
                if output
                    .send(CandleStreamEvent::Unavailable(
                        "Candle stream quiet · verifying history".to_string(),
                    ))
                    .await
                    .is_err()
                {
                    return;
                }
                continue;
            };
            match message {
                Ok(msg) => {
                    if msg.channel == "error"
                        && watchdog.report_error_due()
                        && output
                            .send(CandleStreamEvent::Unavailable(
                                "Candle subscription unavailable · check provider limits"
                                    .to_string(),
                            ))
                            .await
                            .is_err()
                    {
                        return;
                    }
                    if msg.channel == "candle"
                        && msg.data.get("s").and_then(|v| v.as_str()) == Some(&coin)
                        && msg.data.get("i").and_then(|v| v.as_str()) == Some(&interval)
                        && let Ok(candle) = serde_json::from_value::<Candle>((*msg.data).clone())
                        && crate::api::is_valid_candle(&candle)
                    {
                        watchdog.mark_valid();
                        if output.send(CandleStreamEvent::Item(candle)).await.is_err() {
                            return;
                        }
                    }
                }

                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    // Local consumer lag is repaired from history. It does not
                    // imply that the shared transport (or other topics) failed.
                    if output
                        .send(CandleStreamEvent::Lagged { skipped })
                        .await
                        .is_err()
                    {
                        return;
                    }
                }

                Err(error) if crate::ws::broadcast_receiver_closed(&error) => {
                    return;
                }
                Err(_error) => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }))
}

pub fn ws_candle_stream_keyed(params: &(u64, String, String)) -> WsStream<KeyedCandleStreamEvent> {
    let chart_id = params.0;
    let coin = params.1.clone();
    let interval = params.2.clone();
    let pair = (params.1.clone(), params.2.clone());
    let inner = ws_candle_stream(&pair);
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        CandleStreamEvent::Item(candle) => {
            KeyedCandleStreamEvent::Item(chart_id, coin.clone(), interval.clone(), None, candle)
        }
        CandleStreamEvent::Unavailable(reason) => KeyedCandleStreamEvent::Unavailable {
            id: chart_id,
            symbol: coin.clone(),
            interval: interval.clone(),
            hydromancer_key_generation: None,
            reason,
        },
        CandleStreamEvent::Lagged { skipped } => KeyedCandleStreamEvent::Lagged {
            id: chart_id,
            symbol: coin.clone(),
            interval: interval.clone(),
            hydromancer_key_generation: None,
            skipped,
        },
    }))
}

pub fn ws_spaghetti_candle_stream(
    params: &(
        u64,
        u64,
        String,
        crate::timeframe::Timeframe,
        Option<crate::spaghetti::Session>,
        Option<crate::timeframe::Timeframe>,
    ),
) -> WsStream<SpaghettiCandleStreamEvent> {
    let spaghetti_id = params.0;
    let instance_epoch = params.1;
    let coin = params.2.clone();
    let timeframe = params.3;
    let session = params.4;
    let session_granularity = params.5;
    let pair = (params.2.clone(), params.3.api_str().to_string());
    let inner = ws_candle_stream(&pair);
    Box::pin(futures::StreamExt::map(inner, move |event| match event {
        CandleStreamEvent::Item(candle) => SpaghettiCandleStreamEvent::Item {
            id: spaghetti_id,
            instance_epoch,
            symbol: coin.clone(),
            timeframe,
            hydromancer_key_generation: None,
            session,
            session_granularity,
            candle,
        },
        CandleStreamEvent::Unavailable(reason) => SpaghettiCandleStreamEvent::Unavailable {
            id: spaghetti_id,
            instance_epoch,
            symbol: coin.clone(),
            timeframe,
            hydromancer_key_generation: None,
            session,
            session_granularity,
            reason,
        },
        CandleStreamEvent::Lagged { skipped } => SpaghettiCandleStreamEvent::Lagged {
            id: spaghetti_id,
            instance_epoch,
            symbol: coin.clone(),
            timeframe,
            hydromancer_key_generation: None,
            session,
            session_granularity,
            skipped,
        },
    }))
}

/// Measures a candle topic's progress, independent of socket traffic. The
/// absolute deadline also prevents a continuous stream of unrelated messages
/// from starving the timer.
pub(in crate::ws) struct CandleWatchdog {
    silence: std::time::Duration,
    deadline: tokio::time::Instant,
    last_valid: Option<tokio::time::Instant>,
    last_error: Option<tokio::time::Instant>,
}

impl CandleWatchdog {
    pub(in crate::ws) fn new(symbol: &str, interval: &str) -> Self {
        let sparse = symbol.starts_with('@')
            || symbol.starts_with('#')
            || symbol.contains('/')
            || interval == "1M";
        Self::with_silence(std::time::Duration::from_secs(if sparse {
            300
        } else {
            90
        }))
    }
    fn with_silence(silence: std::time::Duration) -> Self {
        Self {
            silence,
            deadline: tokio::time::Instant::now() + silence,
            last_valid: None,
            last_error: None,
        }
    }
    pub(in crate::ws) fn mark_valid(&mut self) {
        let now = tokio::time::Instant::now();
        self.last_valid = Some(now);
        self.deadline = now + self.silence;
    }
    pub(in crate::ws) fn report_error_due(&mut self) -> bool {
        let now = tokio::time::Instant::now();
        if self
            .last_valid
            .is_some_and(|at| now.duration_since(at) < std::time::Duration::from_secs(15))
            || self
                .last_error
                .is_some_and(|at| now.duration_since(at) < std::time::Duration::from_secs(60))
        {
            return false;
        }
        self.last_error = Some(now);
        true
    }
    pub(in crate::ws) async fn recv<T: Clone>(
        &mut self,
        receiver: &mut broadcast::Receiver<T>,
    ) -> Option<Result<T, broadcast::error::RecvError>> {
        tokio::select! {
            biased;
            _ = tokio::time::sleep_until(self.deadline) => {
                self.deadline = tokio::time::Instant::now() + self.silence;
                None
            }
            message = receiver.recv() => Some(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn unrelated_traffic_cannot_hide_a_silent_candle_topic() {
        let (tx, mut rx) = broadcast::channel(16);
        let mut watchdog = CandleWatchdog::with_silence(std::time::Duration::from_millis(10));
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                tx.send("allMids").expect("receiver");
                if watchdog.recv(&mut rx).await.is_none() {
                    break;
                }
            }
        })
        .await
        .expect("quiet topic must time out even while other topics arrive");
    }
    #[tokio::test]
    async fn valid_candles_reset_deadline_and_unscoped_errors_are_throttled() {
        let mut watchdog = CandleWatchdog::new("BTC", "1m");
        assert!(watchdog.report_error_due());
        assert!(!watchdog.report_error_due());
        let before = watchdog.deadline;
        watchdog.mark_valid();
        assert!(watchdog.deadline >= before);
        assert!(!watchdog.report_error_due());
    }
}
