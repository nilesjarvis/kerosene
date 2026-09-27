use std::{future::Future, time::Duration};

/// Request transport recovery before notifying the consumer, even when that
/// consumer has already closed. Pause only after both operations succeed.
pub(super) async fn emit_after_reconnect<T, Emit, Fut>(
    request_reconnect: impl FnOnce() -> bool,
    event: T,
    emit: Emit,
    pause: Duration,
) -> bool
where
    Emit: FnOnce(T) -> Fut,
    Fut: Future<Output = bool>,
{
    if !request_reconnect() {
        return false;
    }
    if !emit(event).await {
        return false;
    }
    if !pause.is_zero() {
        tokio::time::sleep(pause).await;
    }
    true
}

#[cfg(test)]
mod tests;
