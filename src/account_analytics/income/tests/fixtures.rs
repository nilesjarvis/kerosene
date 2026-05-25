use serde_json::Value;
use std::collections::HashMap;

pub(super) async fn income_server(
    responses: HashMap<&'static str, (&'static str, Value)>,
) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(error) => panic!("bind income fixture server: {error}"),
    };
    let addr = match listener.local_addr() {
        Ok(addr) => addr,
        Err(error) => panic!("read income fixture server address: {error}"),
    };
    let responses: HashMap<String, (String, String)> = responses
        .into_iter()
        .map(|(ty, (status, body))| (ty.to_string(), (status.to_string(), body.to_string())))
        .collect();

    tokio::spawn(async move {
        for _ in 0..4 {
            let (mut socket, _) = match listener.accept().await {
                Ok(connection) => connection,
                Err(error) => panic!("accept income fixture connection: {error}"),
            };
            let responses = responses.clone();
            tokio::spawn(async move {
                let mut buf = vec![0_u8; 8192];
                let n = match socket.read(&mut buf).await {
                    Ok(size) => size,
                    Err(error) => panic!("read income fixture request: {error}"),
                };
                let request = String::from_utf8_lossy(&buf[..n]);
                let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
                let ty = serde_json::from_str::<Value>(body)
                    .ok()
                    .and_then(|raw| raw.get("type").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_default();
                let (status_line, response_body) =
                    responses.get(&ty).cloned().unwrap_or_else(|| {
                        (
                            "500 Internal Server Error".to_string(),
                            "unknown request".to_string(),
                        )
                    });
                let response = format!(
                    "HTTP/1.1 {status_line}\r\n\
                    content-type: application/json\r\n\
                    content-length: {}\r\n\
                    connection: close\r\n\r\n\
                    {response_body}",
                    response_body.len()
                );
                if let Err(error) = socket.write_all(response.as_bytes()).await {
                    panic!("write income fixture response: {error}");
                }
            });
        }
    });

    format!("http://{addr}/info")
}

pub(super) fn healthy_income_responses() -> HashMap<&'static str, (&'static str, Value)> {
    HashMap::from([
        (
            "allBorrowLendReserveStates",
            (
                "200 OK",
                serde_json::json!([[
                    0,
                    {
                        "borrowYearlyRate": "0.01",
                        "supplyYearlyRate": "0.02",
                        "oraclePx": "1"
                    }
                ]]),
            ),
        ),
        (
            "borrowLendUserState",
            (
                "200 OK",
                serde_json::json!({
                    "tokenToState": [[
                        0,
                        {"borrow": {"value": "0"}, "supply": {"value": "100"}}
                    ]],
                    "health": "healthy",
                    "healthFactor": "10"
                }),
            ),
        ),
        ("userBorrowLendInterest", ("200 OK", serde_json::json!([]))),
        (
            "spotMeta",
            (
                "200 OK",
                serde_json::json!({"tokens": [{"index": 0, "name": "USDC"}]}),
            ),
        ),
    ])
}
