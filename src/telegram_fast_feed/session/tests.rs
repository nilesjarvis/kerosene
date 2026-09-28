use super::*;
use crate::telegram_fast_feed::DropGuard;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::oneshot;

#[test]
fn session_file_error_display_redacts_parent_path() {
    let rendered = redacted_session_file_display(Path::new(
        "/home/alice/.config/kerosene/telegram_fast.session-wal",
    ));

    assert_eq!(rendered, "<config-dir>/telegram_fast.session-wal");
    assert!(!rendered.contains("/home/alice"));
}

#[tokio::test]
async fn telegram_pool_shutdown_returns_without_abort_when_task_completes() {
    let task = tokio::spawn(async {});

    let aborted = shutdown_telegram_pool_task(task, Duration::from_secs(1)).await;

    assert!(!aborted);
}

#[tokio::test]
async fn telegram_pool_shutdown_aborts_after_timeout() {
    let (started_tx, started_rx) = oneshot::channel();
    let aborted = Arc::new(AtomicBool::new(false));
    let aborted_for_task = Arc::clone(&aborted);
    let task = tokio::spawn(async move {
        let _guard = DropGuard::new(move || {
            aborted_for_task.store(true, Ordering::SeqCst);
        });
        let _ = started_tx.send(());
        futures::future::pending::<()>().await;
    });
    started_rx.await.expect("task should start");

    let did_abort = shutdown_telegram_pool_task(task, Duration::from_millis(1)).await;

    assert!(did_abort);
    assert!(aborted.load(Ordering::SeqCst));
}
