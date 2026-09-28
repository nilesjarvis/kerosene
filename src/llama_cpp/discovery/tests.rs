use super::*;

#[test]
fn loopback_normalization_preserves_supported_paths_and_rejects_other_components() {
    for host in ["127.0.0.1", "localhost", "[::1]"] {
        for path in ["", "/", "///", "/v1", "/v1/", "/v1///"] {
            let input = format!(" \thttp://{host}:35677{path}\n");
            assert_eq!(
                normalize_loopback_base_url(&input),
                Some(format!("http://{host}:35677/v1"))
            );
        }
        for suffix in [
            "/v2",
            "/v1/models",
            "/V1",
            "/v1?",
            "/v1#",
            "/v1?a=1",
            "/v1#id",
        ] {
            assert!(normalize_loopback_base_url(&format!("http://{host}:35677{suffix}")).is_none());
        }
    }
    for input in [
        "https://localhost:35677/v1",
        "http://user@localhost/v1",
        "http://user:pass@localhost/v1",
        "http://localhost.example/v1",
        "http://127.0.0.2/v1",
        "http://0.0.0.0/v1",
        "http://[::]/v1",
    ] {
        assert!(normalize_loopback_base_url(input).is_none());
    }
}

#[test]
fn process_endpoint_parsing_preserves_argument_precedence_and_host_mapping() {
    for (host, expected) in [
        ("0.0.0.0", Some("127.0.0.1")),
        ("*", Some("127.0.0.1")),
        ("127.0.0.1", Some("127.0.0.1")),
        ("localhost", Some("localhost")),
        ("::", Some("[::1]")),
        ("::1", Some("[::1]")),
        ("[::1]", Some("[::1]")),
        ("example.com", None),
        ("127.0.0.2", None),
    ] {
        let arguments = ["llama-server", "--host", host, "--port=35677"].map(str::to_string);
        assert_eq!(
            endpoint_from_process_arguments(&arguments),
            expected.map(|host| format!("http://{host}:35677/v1"))
        );
    }
    for (options, port) in [
        (vec![], 8080),
        (vec!["-p", "35677"], 35677),
        (vec!["--port", "35677", "--port=35678"], 35677),
        (vec!["-p", "35677", "--port=35678"], 35678),
        (vec!["--port=bad", "-p", "35677"], 8080),
        (vec!["-p", "35677", "--port"], 35677),
        (vec!["--port", "--host=localhost"], 8080),
        (vec!["--port=65536"], 8080),
        (vec!["--port=0"], 0),
    ] {
        let mut arguments = vec!["llama-server".to_string()];
        arguments.extend(options.into_iter().map(str::to_string));
        let host = if arguments.iter().any(|value| value == "--host=localhost") {
            "localhost"
        } else {
            "127.0.0.1"
        };
        assert_eq!(
            endpoint_from_process_arguments(&arguments),
            Some(format!("http://{host}:{port}/v1"))
        );
    }
    for (executable, accepted) in [
        ("/opt/llama.cpp/llama-server", true),
        ("llama-server.exe", true),
        ("LLAMA-SERVER", true),
        ("/opt/llama.cpp/server", true),
        ("server", false),
        ("/opt/other/server", false),
        ("llama-client", false),
    ] {
        assert_eq!(
            endpoint_from_process_arguments(&[executable.to_string()]).is_some(),
            accepted
        );
    }
    assert!(endpoint_from_process_arguments(&[]).is_none());
}

#[test]
fn extracts_dynamic_loopback_port_from_llama_server_process() {
    let arguments = vec![
        "/opt/llama.cpp/llama-server".to_string(),
        "-m".to_string(),
        "/models/private-name.gguf".to_string(),
        "--port".to_string(),
        "35677".to_string(),
        "--host=0.0.0.0".to_string(),
    ];
    assert_eq!(
        endpoint_from_process_arguments(&arguments).as_deref(),
        Some("http://127.0.0.1:35677/v1")
    );
}

#[test]
fn explicit_detection_url_is_restricted_to_loopback() {
    assert_eq!(
        normalize_loopback_base_url("http://localhost:8080").as_deref(),
        Some("http://localhost:8080/v1")
    );
    assert!(normalize_loopback_base_url("https://127.0.0.1:8080/v1").is_none());
    assert!(normalize_loopback_base_url("http://example.com:8080/v1").is_none());
    assert!(normalize_loopback_base_url("http://user:pass@127.0.0.1:8080/v1").is_none());
}
