#[cfg(test)]
use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};

#[cfg(not(test))]
fn secret_warnings() -> &'static Mutex<Vec<String>> {
    static WARNINGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    WARNINGS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(test)]
thread_local! {
    static SECRET_WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[cfg(not(test))]
pub(crate) fn push_secret_warning(message: String) {
    if let Ok(mut warnings) = secret_warnings().lock() {
        warnings.push(message);
    }
}

#[cfg(test)]
pub(crate) fn push_secret_warning(message: String) {
    SECRET_WARNINGS.with(|warnings| warnings.borrow_mut().push(message));
}

#[cfg(not(test))]
pub fn take_secret_warnings() -> Vec<String> {
    secret_warnings()
        .lock()
        .map(|mut warnings| std::mem::take(&mut *warnings))
        .unwrap_or_default()
}

#[cfg(test)]
pub fn take_secret_warnings() -> Vec<String> {
    SECRET_WARNINGS.with(|warnings| std::mem::take(&mut *warnings.borrow_mut()))
}

#[cfg(test)]
pub(crate) fn secret_warning_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("secret warning test lock should not be poisoned")
}

#[cfg(test)]
mod tests {
    use super::{push_secret_warning, secret_warning_test_lock, take_secret_warnings};

    #[test]
    fn secret_warnings_are_isolated_by_test_thread() {
        let _warning_guard = secret_warning_test_lock();
        let _ = take_secret_warnings();

        push_secret_warning("main warning".to_string());
        let worker_warnings = std::thread::spawn(|| {
            push_secret_warning("worker warning".to_string());
            take_secret_warnings()
        })
        .join()
        .expect("secret warning worker should finish");

        assert_eq!(worker_warnings, vec!["worker warning".to_string()]);
        assert_eq!(take_secret_warnings(), vec!["main warning".to_string()]);
        assert!(take_secret_warnings().is_empty());
    }
}
