use crate::api::{WatchlistContext, WatchlistContextsResponse};
use crate::helpers::redact_sensitive_response_text;

use std::collections::HashMap;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Symbol Search Contexts
// ---------------------------------------------------------------------------

const SYMBOL_SEARCH_CONTEXT_FAILURE_PREFIX: &str = "24h volume refresh failed:";
const SYMBOL_SEARCH_CONTEXT_PARTIAL_PREFIX: &str = "24h volume refresh partially failed:";

pub(super) fn apply_contexts_loaded(
    loading: &mut bool,
    last_fetch_ms: &mut Option<u64>,
    contexts: &mut HashMap<String, WatchlistContext>,
    status: &mut Option<(String, bool)>,
    requested_at: u64,
    result: Result<WatchlistContextsResponse, String>,
) {
    *loading = false;

    match result {
        Ok(response) => {
            *last_fetch_ms = Some(requested_at);
            *contexts = response.contexts;
            *status = if response.partial_errors.is_empty() {
                None
            } else {
                Some((
                    format!(
                        "{SYMBOL_SEARCH_CONTEXT_PARTIAL_PREFIX} {}",
                        redact_sensitive_response_text(&response.partial_errors.join("; "))
                    ),
                    true,
                ))
            };
        }
        Err(error) => {
            *status = Some((
                format!(
                    "{SYMBOL_SEARCH_CONTEXT_FAILURE_PREFIX} {}",
                    redact_sensitive_response_text(&error)
                ),
                true,
            ));
        }
    }
}
