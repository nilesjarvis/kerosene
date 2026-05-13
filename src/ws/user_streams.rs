mod events;
mod model;
mod routing;
mod subscriptions;

use futures::SinkExt as _;
use tokio::sync::broadcast;

use super::{SubscriptionGuard, WsCommand, get_manager};
use events::parse_user_stream_message;
use routing::normalize_ws_user_address;
use subscriptions::build_user_stream_subscriptions;

pub use model::{KeyedUserData, WsUserData};

#[allow(clippy::ptr_arg)]
pub fn ws_user_data_stream(
    params: &(Option<String>, Vec<String>),
) -> std::pin::Pin<Box<dyn futures::Stream<Item = KeyedUserData> + Send>> {
    let addr = params.0.as_deref().and_then(normalize_ws_user_address);
    let dexes = params.1.clone();

    Box::pin(iced::stream::channel(20, async move |mut output| {
        let (cmd_tx, mut msg_rx) = get_manager();

        let mut topics = Vec::new();
        for (topic, payload) in build_user_stream_subscriptions(addr.as_deref(), &dexes) {
            if cmd_tx
                .send(WsCommand::Subscribe {
                    topic: topic.clone(),
                    payload,
                })
                .is_err()
            {
                return;
            }
            topics.push(topic);
        }

        let _guard = SubscriptionGuard { cmd_tx, topics };

        loop {
            match msg_rx.recv().await {
                Ok(msg) => {
                    if let Some(update) = parse_user_stream_message(
                        msg.channel.as_str(),
                        msg.data.as_ref(),
                        addr.as_deref(),
                        addr.clone(),
                    ) && output.send(update).await.is_err()
                    {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    // Falling behind the 10k-frame broadcast buffer means
                    // we lost order/fill/position updates. Trading code
                    // must NOT silently continue from an unknown state —
                    // surface the lag so the downstream handler can force
                    // a full account refresh.
                    if output
                        .send((addr.clone(), WsUserData::Lagged { skipped }))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }))
}
