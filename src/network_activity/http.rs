use super::{ActivityEntry, ActivityKind, Provider, record, safe_operation};
use reqwest::{Client, Request, RequestBuilder, Response};
use serde::Deserialize;
use std::time::Instant;

// Borrow only the discriminants; ignore signatures, account data and other payload fields.
#[derive(Default, Deserialize)]
struct Operation<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<&'a str>,
    #[serde(borrow)]
    action: Option<Action<'a>>,
    #[serde(rename = "operationName", borrow)]
    graphql: Option<&'a str>,
}

#[derive(Deserialize)]
struct Action<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<&'a str>,
}

fn request_metadata(request: &Request, proxied: bool) -> ActivityEntry {
    let url = request.url();
    let provider = match url.host_str().unwrap_or_default() {
        "api.hyperliquid.xyz" | "api.hyperliquid-testnet.xyz" => Provider::Hyperliquid,
        "api.hydromancer.xyz" => Provider::Hydromancer,
        "api.hyperdash.com" => Provider::Hyperdash,
        "api.hypurrscan.io" => Provider::Hypurrscan,
        "openrouter.ai" => Provider::OpenRouter,
        "t.me" | "api.telegram.org" => Provider::Telegram,
        "api.x.com" | "api.twitter.com" => Provider::X,
        "www.sec.gov" | "data.sec.gov" => Provider::Sec,
        "nfs.faireconomy.media" => Provider::Calendar,
        "api.hyperunit.xyz" => Provider::Unit,
        "localhost" | "127.0.0.1" | "[::1]" => Provider::Local,
        _ => Provider::Other,
    };
    let operation = match (provider, url.path()) {
        (Provider::Hyperliquid | Provider::Hydromancer, "/info" | "/exchange") => {
            let operation = request
                .body()
                .and_then(|body| body.as_bytes())
                .and_then(|bytes| serde_json::from_slice::<Operation<'_>>(bytes).ok())
                .unwrap_or_default();
            operation
                .action
                .and_then(|action| action.kind)
                .or(operation.kind)
                .map(safe_operation)
                .unwrap_or("other")
        }
        (Provider::Hyperdash, _) => {
            let operation = request
                .body()
                .and_then(|body| body.as_bytes())
                .and_then(|bytes| serde_json::from_slice::<Operation<'_>>(bytes).ok())
                .unwrap_or_default();
            match operation.graphql {
                Some("GetTickerPositions") => "GetTickerPositions",
                Some("GetPerpDeltas") => "GetPerpDeltas",
                Some("GetLiquidationLevels") => "GetLiquidationLevels",
                Some("GetHistoricalLiquidationLevel") => "GetHistoricalLiquidationLevel",
                _ => "graphql",
            }
        }
        (Provider::OpenRouter, "/api/v1/chat/completions") => "chat/completions",
        (Provider::OpenRouter, "/api/v1/models") => "models",
        (Provider::OpenRouter, _) => "account",
        (Provider::Hypurrscan, "/unstakingQueue") => "unstakingQueue",
        (Provider::Calendar, _) => "calendar",
        (Provider::Telegram, _) => "feed/media",
        (Provider::X, _) => "feed/media",
        (Provider::Sec, _) => "filings",
        (Provider::Unit, _) => "transfers",
        (Provider::Local, _) => "local request",
        _ => "request",
    };
    let mut entry = ActivityEntry::new(provider, ActivityKind::HttpSend, operation);
    entry.method = match request.method().as_str() {
        "GET" => "GET",
        "POST" => "POST",
        "PUT" => "PUT",
        "DELETE" => "DELETE",
        "PATCH" => "PATCH",
        "HEAD" => "HEAD",
        _ => "HTTP",
    };
    entry.proxied = proxied;
    entry
}

struct PendingRequest {
    entry: ActivityEntry,
    started: Instant,
    completed: bool,
}

impl PendingRequest {
    fn start(request: &Request, proxied: bool) -> Self {
        let mut entry = request_metadata(request, proxied);
        let request_id = record(entry.clone());
        entry.request_id = Some(request_id);
        Self {
            entry,
            started: Instant::now(),
            completed: false,
        }
    }

    fn finish(&mut self, kind: ActivityKind) {
        self.completed = true;
        self.entry.kind = kind;
        self.entry.elapsed_ms =
            Some(self.started.elapsed().as_millis().min(u64::MAX as u128) as u64);
        record(self.entry.clone());
    }
}

impl Drop for PendingRequest {
    fn drop(&mut self) {
        if !self.completed {
            self.finish(ActivityKind::HttpCancelled);
        }
    }
}

/// One record per application transport attempt, including each proxy retry.
/// Completion measures time to response headers; response bodies remain untouched.
pub(crate) async fn execute(
    client: &Client,
    request: Request,
    proxied: bool,
) -> reqwest::Result<Response> {
    let mut pending = PendingRequest::start(&request, proxied);
    let result = client.execute(request).await;
    pending.finish(match &result {
        Ok(response) => ActivityKind::HttpResponse(response.status().as_u16()),
        Err(_) => ActivityKind::HttpFailed,
    });
    result
}

pub(crate) trait HttpRequestExt {
    fn send_observed(self) -> impl Future<Output = reqwest::Result<Response>> + Send;
}

impl HttpRequestExt for RequestBuilder {
    async fn send_observed(self) -> reqwest::Result<Response> {
        let (client, request) = self.build_split();
        execute(&client, request?, false).await
    }
}

#[cfg(test)]
mod tests;
