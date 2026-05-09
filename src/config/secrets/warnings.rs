use std::sync::{Mutex, OnceLock};

fn secret_warnings() -> &'static Mutex<Vec<String>> {
    static WARNINGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    WARNINGS.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn push_secret_warning(message: String) {
    if let Ok(mut warnings) = secret_warnings().lock() {
        warnings.push(message);
    }
}

pub fn take_secret_warnings() -> Vec<String> {
    secret_warnings()
        .lock()
        .map(|mut warnings| std::mem::take(&mut *warnings))
        .unwrap_or_default()
}
