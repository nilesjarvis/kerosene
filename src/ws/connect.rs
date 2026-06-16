use std::future::Future;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConnectAttempt<T, E> {
    Finished(Result<T, E>),
    TimedOut,
}

pub(crate) async fn connect_with_timeout<F, T, E>(
    connect: F,
    timeout: Duration,
) -> ConnectAttempt<T, E>
where
    F: Future<Output = Result<T, E>>,
{
    match tokio::time::timeout(timeout, connect).await {
        Ok(result) => ConnectAttempt::Finished(result),
        Err(_) => ConnectAttempt::TimedOut,
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectAttempt, connect_with_timeout};
    use std::future::{pending, ready};
    use std::time::Duration;

    #[tokio::test]
    async fn connect_with_timeout_preserves_completed_result() {
        let result =
            connect_with_timeout(ready(Ok::<_, ()>("connected")), Duration::from_secs(60)).await;

        assert_eq!(result, ConnectAttempt::Finished(Ok("connected")));
    }

    #[tokio::test]
    async fn connect_with_timeout_returns_timeout_for_pending_connect() {
        let result =
            connect_with_timeout(pending::<Result<(), ()>>(), Duration::from_millis(1)).await;

        assert_eq!(result, ConnectAttempt::TimedOut);
    }
}
