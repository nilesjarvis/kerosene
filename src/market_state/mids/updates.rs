use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// All-Mids Updates
// ---------------------------------------------------------------------------

pub(super) fn apply_mids_update<IsMuted>(
    all_mids: &mut HashMap<String, f64>,
    all_mids_updated_at_ms: &mut HashMap<String, u64>,
    flashes: &mut HashMap<String, (u64, i8)>,
    mids: HashMap<String, f64>,
    now_ms: u64,
    mut is_muted: IsMuted,
) where
    IsMuted: FnMut(&str) -> bool,
{
    for (key, new_price) in mids {
        if !new_price.is_finite() || new_price <= 0.0 {
            continue;
        }
        if is_muted(&key) {
            continue;
        }
        if let Some(&old_price) = all_mids.get(&key)
            && (new_price - old_price).abs() > f64::EPSILON
        {
            let direction = if new_price > old_price { 1 } else { -1 };
            flashes.insert(key.clone(), (now_ms, direction));
        }
        all_mids.insert(key.clone(), new_price);
        all_mids_updated_at_ms.insert(key, now_ms);
    }
}
