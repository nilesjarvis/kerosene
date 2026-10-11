use std::collections::HashMap;

use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::ApprovalSession;

const MAX_REQUEST_BYTES: usize = 16 * 1024;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum Completion {
    Approved,
    Submitted,
    Cancelled,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompletionBody {
    status: Completion,
}

struct Request {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

pub(super) async fn handle_connection(
    stream: &mut TcpStream,
    session: &ApprovalSession,
) -> Option<Completion> {
    let Ok(request) = read_request(stream).await else {
        let _ = respond(
            stream,
            session,
            "400 Bad Request",
            "text/plain",
            "Invalid request",
        )
        .await;
        return None;
    };
    if !request_is_allowed(&request, session) {
        let _ = respond(stream, session, "403 Forbidden", "text/plain", "Forbidden").await;
        return None;
    }
    if request.method == "GET" && request.path == session.path {
        let _ = respond(
            stream,
            session,
            "200 OK",
            "text/html; charset=utf-8",
            &session.html,
        )
        .await;
        return None;
    }
    if request.method == "POST" && request.path == session.completion_path {
        let Ok(body) = serde_json::from_slice::<CompletionBody>(&request.body) else {
            let _ = respond(
                stream,
                session,
                "400 Bad Request",
                "text/plain",
                "Invalid completion",
            )
            .await;
            return None;
        };
        let _ = respond(stream, session, "200 OK", "application/json", "{}").await;
        return Some(body.status);
    }
    let _ = respond(stream, session, "404 Not Found", "text/plain", "Not found").await;
    None
}

fn request_is_allowed(request: &Request, session: &ApprovalSession) -> bool {
    if request.headers.get("host") != Some(&session.host)
        || request
            .headers
            .get("sec-fetch-site")
            .is_some_and(|site| site == "cross-site")
        || request
            .headers
            .get("origin")
            .is_some_and(|origin| origin != &session.origin)
    {
        return false;
    }
    match request.method.as_str() {
        "GET" => request.body.is_empty(),
        "POST" => {
            request.headers.get("origin") == Some(&session.origin)
                && request.headers.get("content-type").map(String::as_str)
                    == Some("application/json")
                && request.body.len() <= 256
        }
        _ => false,
    }
}

async fn read_request(stream: &mut TcpStream) -> Result<Request, ()> {
    let mut bytes = Vec::with_capacity(2048);
    let header_end = loop {
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        read_more(stream, &mut bytes).await?;
    };
    let mut request = parse_head(&bytes[..header_end])?;
    let length = match request.headers.get("content-length") {
        Some(value) if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) => {
            value.parse::<usize>().map_err(|_| ())?
        }
        None if request.method == "GET" => 0,
        _ => return Err(()),
    };
    let total = header_end
        .checked_add(length)
        .filter(|size| *size <= MAX_REQUEST_BYTES)
        .ok_or(())?;
    while bytes.len() < total {
        read_more(stream, &mut bytes).await?;
    }
    // Reject pipelined bytes: every connection serves one bounded request.
    if bytes.len() != total {
        return Err(());
    }
    request.body = bytes.split_off(header_end);
    Ok(request)
}

async fn read_more(stream: &mut TcpStream, bytes: &mut Vec<u8>) -> Result<(), ()> {
    let remaining = MAX_REQUEST_BYTES
        .checked_sub(bytes.len())
        .filter(|size| *size > 0)
        .ok_or(())?;
    let mut chunk = [0_u8; 2048];
    let limit = remaining.min(chunk.len());
    let count = stream.read(&mut chunk[..limit]).await.map_err(|_| ())?;
    if count == 0 {
        return Err(());
    }
    bytes.extend_from_slice(&chunk[..count]);
    Ok(())
}

fn parse_head(bytes: &[u8]) -> Result<Request, ()> {
    let head = std::str::from_utf8(bytes).map_err(|_| ())?;
    let head = head.strip_suffix("\r\n\r\n").ok_or(())?;
    let mut lines = head.split("\r\n");
    let first = lines.next().ok_or(())?.split(' ').collect::<Vec<_>>();
    if first.len() != 3
        || !matches!(first[0], "GET" | "POST")
        || !first[1].starts_with('/')
        || !first[1].bytes().all(|byte| byte.is_ascii_graphic())
        || first[2] != "HTTP/1.1"
    {
        return Err(());
    }
    let mut headers = HashMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(())?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || value
                .bytes()
                .any(|byte| byte.is_ascii_control() && byte != b'\t')
            || headers.len() >= 32
            || headers
                .insert(name.to_ascii_lowercase(), value.trim().to_string())
                .is_some()
        {
            return Err(());
        }
    }
    if headers.contains_key("transfer-encoding") || !headers.contains_key("host") {
        return Err(());
    }
    Ok(Request {
        method: first[0].to_string(),
        path: first[1].to_string(),
        headers,
        body: Vec::new(),
    })
}

async fn respond(
    stream: &mut TcpStream,
    session: &ApprovalSession,
    status: &str,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    // The script nonce is also the private session token. Never disclose it in
    // an error or completion response accessible without the page's exact URL.
    let policy = if status == "200 OK" && content_type == "text/html; charset=utf-8" {
        format!(
            "default-src 'none'; script-src 'nonce-{}'; style-src 'nonce-{}'; connect-src 'self' https://api.hyperliquid.xyz; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
            session.token, session.token,
        )
    } else {
        "default-src 'none'; frame-ancestors 'none'".to_string()
    };
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nCross-Origin-Resource-Policy: same-origin\r\nContent-Security-Policy: {policy}\r\n\r\n",
        body.len(),
    );
    stream.write_all(headers.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    stream.shutdown().await
}

#[cfg(test)]
mod tests;
