use reqwest::{Response, ResponseBuilderExt};

// Info endpoints return bounded JSON snapshots, not streaming downloads.
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

/// Finish the body while the caller still owns the route and concurrency lease.
/// Preserve reqwest's response API so existing readers retain their decoding behavior.
pub(super) async fn buffer(mut response: Response) -> Result<Response, String> {
    let too_large = || "Hyperliquid proxy response exceeded the size limit".to_string();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Hyperliquid proxy response body failed or timed out".to_string())?
    {
        if chunk.len() > MAX_RESPONSE_BYTES - body.len() {
            return Err(too_large());
        }
        body.extend_from_slice(&chunk);
    }
    let mut buffered = http::Response::builder()
        .status(response.status())
        .version(response.version())
        .url(response.url().clone())
        .body(body)
        .map_err(|_| "Could not buffer Hyperliquid proxy response".to_string())?;
    *buffered.headers_mut() = response.headers().clone();
    buffered
        .extensions_mut()
        .extend(std::mem::take(response.extensions_mut()));
    Ok(buffered.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unknown_length_response_cannot_exceed_the_body_limit() {
        let chunks = futures::stream::iter(
            [vec![0; MAX_RESPONSE_BYTES], vec![0]]
                .into_iter()
                .map(Ok::<_, std::io::Error>),
        );
        let response: Response = http::Response::new(reqwest::Body::wrap_stream(chunks)).into();
        assert!(response.content_length().is_none());
        let error = buffer(response).await.expect_err("oversized stream");
        assert!(error.contains("size limit"));
    }
}
