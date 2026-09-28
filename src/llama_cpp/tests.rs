use super::*;
use std::io::{Read, Write};

#[test]
fn model_ids_keep_trimmed_unicode_and_reject_controls_or_excess_length() {
    for character in ['a', 'é', '界', '🦀'] {
        for length in [0, 1, 199, 200, 201, 500] {
            let id = character.to_string().repeat(length);
            let padded = format!(" \t\r\n{id}\n\t ");
            let expected = (1..=200).contains(&length).then_some(id);
            assert_eq!(bounded_model_id(&padded), expected);
        }
    }
    for id in [
        "",
        " \t\r\n",
        "bad\0id",
        "bad\tid",
        "bad\nid",
        "bad\u{85}id",
    ] {
        assert!(bounded_model_id(id).is_none());
    }
    assert_eq!(
        bounded_model_id(" model / 名称 ").as_deref(),
        Some("model / 名称")
    );
}

#[test]
fn pi_config_keeps_local_provider_zero_cost_and_tool_compatible() {
    let server = LlamaCppServer {
        base_url: "http://127.0.0.1:35677/v1".to_string(),
        models: vec![LlamaCppModel {
            id: "local-model.gguf".to_string(),
            context_window: Some(30_720),
        }],
        supports_tools: true,
        supports_vision: true,
        supports_reasoning: true,
    };
    let config = pi_models_config(&server);
    assert_eq!(
        config.pointer("/providers/llamacpp/baseUrl"),
        Some(&json!("http://127.0.0.1:35677/v1"))
    );
    assert_eq!(
        config.pointer("/providers/llamacpp/models/0/contextWindow"),
        Some(&json!(30_720))
    );
    assert_eq!(
        config.pointer("/providers/llamacpp/models/0/input/1"),
        Some(&json!("image"))
    );
    assert_eq!(
        config.pointer("/providers/llamacpp/models/0/cost/input"),
        Some(&json!(0))
    );
}

#[tokio::test]
async fn probe_verifies_llama_props_and_model_catalog() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let address = listener.local_addr().expect("mock server address");
    let server_thread = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe request");
            let mut request = [0_u8; 2_048];
            let read = stream.read(&mut request).expect("read probe request");
            let request = String::from_utf8_lossy(&request[..read]);
            let body = if request.starts_with("GET /props ") {
                json!({
                    "model_path": "/not/exposed/model.gguf",
                    "default_generation_settings": { "n_ctx": 30_720 },
                    "chat_template_caps": {
                        "supports_tools": true,
                        "supports_tool_calls": true,
                        "supports_preserve_reasoning": true
                    },
                    "modalities": { "vision": true }
                })
            } else {
                json!({
                    "data": [{
                        "id": "verified-local.gguf",
                        "meta": { "n_ctx": 30_720 }
                    }]
                })
            };
            let body = serde_json::to_vec(&body).expect("serialize mock response");
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write response headers");
            stream.write_all(&body).expect("write response body");
        }
    });

    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("build probe client");
    let detected = probe_server(&client, &format!("http://{address}/v1"))
        .await
        .expect("verified llama.cpp server");
    server_thread.join().expect("mock server thread");

    assert!(detected.supports_tools);
    assert!(detected.supports_vision);
    assert!(detected.supports_reasoning);
    assert_eq!(
        detected.primary_model(),
        Some(&LlamaCppModel {
            id: "verified-local.gguf".to_string(),
            context_window: Some(30_720),
        })
    );
}
