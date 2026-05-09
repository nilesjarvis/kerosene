use reqwest::StatusCode;

use super::response_snippet;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// HyperDash Error Messages
// ---------------------------------------------------------------------------

pub(super) fn hyperdash_http_error(scope: &str, status: StatusCode, body: &str) -> String {
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return format!(
            "HyperDash {scope} authentication failed (HTTP {status}). Check the API key in Settings > Integrations."
        );
    }

    let snippet = response_snippet(body);
    format!("HyperDash {scope} request failed (HTTP {status}): {snippet}")
}

pub(super) fn hyperdash_graphql_error(scope: &str, messages: Vec<String>) -> String {
    let joined = messages.join("; ");
    if is_auth_failure_message(&joined) {
        return format!(
            "HyperDash {scope} authentication failed. Check the API key in Settings > Integrations."
        );
    }
    format!("HyperDash {scope} error: {joined}")
}

pub(super) fn hyperdash_missing_data_error(scope: &str) -> String {
    format!(
        "HyperDash {scope} returned no data. Check the API key in Settings > Integrations if this persists."
    )
}

fn is_auth_failure_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("unauthorized")
        || lower.contains("unauthenticated")
        || lower.contains("forbidden")
        || lower.contains("authentication")
        || lower.contains("authorization")
        || lower.contains("invalid api key")
        || lower.contains("invalid token")
        || lower.contains("api key")
        || lower.contains("token")
}
