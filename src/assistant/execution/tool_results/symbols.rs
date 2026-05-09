use crate::api::fetch_exchange_symbols;

// ---------------------------------------------------------------------------
// Assistant Tool Symbol Resolution
// ---------------------------------------------------------------------------

pub(super) async fn resolve_valid_symbol(symbol: String) -> Result<String, String> {
    if symbol.trim().is_empty() {
        return Err("Missing symbol".to_string());
    }
    let universe = fetch_exchange_symbols().await?;
    if let Some(exact) = universe
        .iter()
        .find(|s| s.key.eq_ignore_ascii_case(&symbol) || s.ticker.eq_ignore_ascii_case(&symbol))
    {
        return Ok(exact.key.clone());
    }
    Err(format!(
        "Unknown symbol '{}'. Try using ${{HYPE}} or ${{BTC}} style ticker mentions.",
        symbol
    ))
}
