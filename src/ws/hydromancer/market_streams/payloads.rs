use serde_json::Value;

// Market streams accept a direct item, a wrapped item, or a data batch.
// Return slices into the frame so consumers can filter before decoding.
fn direct_or_batched_items<'a>(value: &'a Value, fields: [&str; 2]) -> Option<&'a [Value]> {
    let is_item = |item: &Value| fields.iter().all(|field| item.get(*field).is_some());
    if is_item(value) {
        return Some(std::slice::from_ref(value));
    }
    let data = value.get("data")?;
    if is_item(data) {
        Some(std::slice::from_ref(data))
    } else {
        data.as_array().map(Vec::as_slice)
    }
}

pub(super) fn l2_book_items(value: &Value) -> &[Value] {
    direct_or_batched_items(value, ["coin", "levels"])
        .or_else(|| value.get("books")?.as_array().map(Vec::as_slice))
        .unwrap_or_default()
}

pub(super) fn active_asset_ctx_items(value: &Value) -> &[Value] {
    direct_or_batched_items(value, ["coin", "ctx"]).unwrap_or_default()
}

pub(super) fn candle_items(value: &Value) -> &[Value] {
    direct_or_batched_items(value, ["s", "i"])
        .or_else(|| value.get("candle").map(std::slice::from_ref))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
