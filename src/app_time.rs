use crate::app_state::TradingTerminal;

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

pub(crate) fn cooldown_heat(first_seen_ms: u64, now_ms: u64, cooldown_ms: u64) -> f32 {
    if first_seen_ms == 0 {
        return 0.0;
    }

    let age_ms = now_ms.saturating_sub(first_seen_ms);
    if age_ms >= cooldown_ms {
        0.0
    } else {
        1.0 - (age_ms as f32 / cooldown_ms as f32)
    }
}

impl TradingTerminal {
    pub(crate) fn now_ms() -> u64 {
        now_ms()
    }
}
