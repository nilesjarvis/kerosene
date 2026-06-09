use crate::app_state::TradingTerminal;
use crate::config::ReadDataProvider;

// ---------------------------------------------------------------------------
// Read Data Provider Selection
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn hydromancer_read_provider_enabled(&self) -> bool {
        self.read_data_provider == ReadDataProvider::Hydromancer
            && !self.hydromancer_api_key.trim().is_empty()
    }

    pub(crate) fn hydromancer_read_provider_key(&self) -> Option<String> {
        self.hydromancer_read_provider_enabled()
            .then(|| self.hydromancer_api_key.trim().to_string())
    }
}

pub(crate) fn fallback_warning(scope: &str, error: &str) -> String {
    format!(
        "Hydromancer {scope} failed; used Hyperliquid fallback: {}",
        provider_error_summary(error)
    )
}

fn provider_error_summary(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("401")
        || lower.contains("403")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("invalid api key")
        || lower.contains("invalid token")
        || lower.contains("authentication")
    {
        return "authentication failed".to_string();
    }

    crate::helpers::text_excerpt(error, 160)
}
