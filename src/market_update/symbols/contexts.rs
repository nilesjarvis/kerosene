use crate::api::{WatchlistContext, WatchlistContextsResponse};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use iced::Task;

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

impl TradingTerminal {
    pub(super) fn apply_symbol_search_contexts_loaded(
        &mut self,
        request_id: u64,
        requested_symbols: Vec<String>,
        requested_at: u64,
        result: Result<crate::api::WatchlistContextsResponse, String>,
    ) -> Task<Message> {
        if request_id != self.symbol_search_contexts_request_id {
            return Task::none();
        }

        self.symbol_search_contexts_request_id =
            self.symbol_search_contexts_request_id.saturating_add(1);
        let refresh_pending = self.symbol_search_contexts_refresh_pending;
        self.symbol_search_contexts_refresh_pending = false;
        self.symbol_search_contexts_request_symbols.clear();

        let result = result.map(|mut response| {
            let requested_symbols: std::collections::HashSet<String> =
                requested_symbols.into_iter().collect();
            if !response.partial_errors.is_empty() {
                let mut merged = std::mem::take(&mut self.symbol_search_ctxs);
                merged.extend(response.contexts);
                response.contexts = merged;
            }
            response
                .contexts
                .retain(|symbol, _| requested_symbols.contains(symbol));
            response
        });

        apply_contexts_loaded(
            &mut self.symbol_search_contexts_loading,
            &mut self.symbol_search_contexts_last_fetch_ms,
            &mut self.symbol_search_ctxs,
            &mut self.symbol_search_status,
            requested_at,
            result,
        );
        self.refresh_symbol_search_results();

        if refresh_pending {
            return self.request_symbol_search_context_refresh(true);
        }

        Task::none()
    }
}
