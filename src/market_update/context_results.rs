use crate::api::{WatchlistContext, WatchlistContextsResponse};
use std::collections::{HashMap, HashSet};

/// Scope a completed request to symbols that are still displayed. Complete
/// responses replace requested values; partial responses retain omitted values.
///
/// Successful results take ownership of retained cache entries; callers must
/// apply the returned response. Errors prune removed symbols, and an entirely
/// obsolete request completes successfully without reporting its old error.
pub(super) fn scope_context_response(
    contexts: &mut HashMap<String, WatchlistContext>,
    current_symbols: &HashSet<String>,
    requested_symbols: Vec<String>,
    result: Result<WatchlistContextsResponse, String>,
) -> Result<WatchlistContextsResponse, String> {
    let requested_symbols: HashSet<_> = requested_symbols.into_iter().collect();
    match result {
        Ok(mut response) => {
            let partial = !response.partial_errors.is_empty();
            let mut scoped_contexts = std::mem::take(contexts);
            scoped_contexts.retain(|symbol, _| {
                current_symbols.contains(symbol) && (partial || !requested_symbols.contains(symbol))
            });
            scoped_contexts.extend(response.contexts.into_iter().filter(|(symbol, _)| {
                current_symbols.contains(symbol) && requested_symbols.contains(symbol)
            }));
            response.contexts = scoped_contexts;
            Ok(response)
        }
        Err(error) => {
            contexts.retain(|symbol, _| current_symbols.contains(symbol));
            if current_symbols.is_disjoint(&requested_symbols) {
                Ok(std::mem::take(contexts).into())
            } else {
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests;
