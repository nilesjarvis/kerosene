use reqwest::Url;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

const DEFAULT_LLAMA_CPP_PORTS: [u16; 2] = [8080, 8081];
const MAX_DISCOVERED_ENDPOINTS: usize = 12;
const MAX_DISCOVERED_MODELS: usize = 16;
const MAX_MODEL_ID_CHARS: usize = 200;

// ---------------------------------------------------------------------------
// Local llama.cpp Discovery
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LlamaCppModel {
    pub(crate) id: String,
    pub(crate) context_window: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LlamaCppServer {
    /// OpenAI-compatible API root, always normalized to a loopback `/v1` URL.
    pub(crate) base_url: String,
    pub(crate) models: Vec<LlamaCppModel>,
    pub(crate) supports_tools: bool,
    pub(crate) supports_vision: bool,
    pub(crate) supports_reasoning: bool,
}

impl LlamaCppServer {
    pub(crate) fn primary_model(&self) -> Option<&LlamaCppModel> {
        self.models.first()
    }

    pub(crate) fn endpoint_label(&self) -> String {
        Url::parse(&self.base_url)
            .ok()
            .and_then(|url| {
                let host = url.host_str()?;
                Some(match url.port() {
                    Some(port) => format!("{host}:{port}"),
                    None => host.to_string(),
                })
            })
            .unwrap_or_else(|| "local machine".to_string())
    }
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<RawModel>,
}

#[derive(Debug, Deserialize)]
struct RawModel {
    id: String,
    #[serde(default)]
    meta: RawModelMeta,
}

#[derive(Debug, Default, Deserialize)]
struct RawModelMeta {
    n_ctx: Option<u64>,
}

pub(crate) async fn detect_server() -> Result<Option<LlamaCppServer>, String> {
    let candidates = detection_candidates()?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(250))
        .timeout(Duration::from_millis(850))
        .no_proxy()
        .build()
        .map_err(|error| format!("Could not prepare local model detection: {error}"))?;

    for base_url in candidates {
        if let Some(server) = probe_server(&client, &base_url).await {
            return Ok(Some(server));
        }
    }
    Ok(None)
}

fn detection_candidates() -> Result<Vec<String>, String> {
    let mut candidates = Vec::new();
    if let Some(configured) = std::env::var_os("KEROSENE_LLAMA_CPP_URL")
        && !configured.is_empty()
    {
        let configured = configured.to_string_lossy();
        candidates.push(normalize_loopback_base_url(&configured).ok_or_else(|| {
            "KEROSENE_LLAMA_CPP_URL must be an HTTP loopback URL without credentials, query, or fragment"
                .to_string()
        })?);
    }

    candidates.extend(process_endpoints());
    candidates.extend(
        DEFAULT_LLAMA_CPP_PORTS
            .into_iter()
            .map(|port| format!("http://127.0.0.1:{port}/v1")),
    );

    let mut seen = HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates.truncate(MAX_DISCOVERED_ENDPOINTS);
    Ok(candidates)
}

async fn probe_server(client: &reqwest::Client, base_url: &str) -> Option<LlamaCppServer> {
    let root_url = base_url.strip_suffix("/v1")?;
    let props = client
        .get(format!("{root_url}/props"))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<Value>()
        .await
        .ok()?;
    if !looks_like_llama_cpp_props(&props) {
        return None;
    }

    let catalog = client
        .get(format!("{base_url}/models"))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<ModelsResponse>()
        .await
        .ok()?;

    let fallback_context = props
        .pointer("/default_generation_settings/n_ctx")
        .and_then(Value::as_u64);
    let models = catalog
        .data
        .into_iter()
        .take(MAX_DISCOVERED_MODELS)
        .filter_map(|model| {
            let id = bounded_model_id(&model.id)?;
            Some(LlamaCppModel {
                id,
                context_window: model.meta.n_ctx.or(fallback_context),
            })
        })
        .collect::<Vec<_>>();
    if models.is_empty() {
        return None;
    }

    let supports_tools = props
        .pointer("/chat_template_caps/supports_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && props
            .pointer("/chat_template_caps/supports_tool_calls")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    Some(LlamaCppServer {
        base_url: base_url.to_string(),
        models,
        supports_tools,
        supports_vision: props
            .pointer("/modalities/vision")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        supports_reasoning: props
            .pointer("/chat_template_caps/supports_preserve_reasoning")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn looks_like_llama_cpp_props(props: &Value) -> bool {
    props.get("chat_template_caps").is_some()
        && props.get("default_generation_settings").is_some()
        && (props.get("model_path").is_some() || props.get("build_info").is_some())
}

fn bounded_model_id(id: &str) -> Option<String> {
    let id = id.trim();
    if id.is_empty() || id.chars().any(char::is_control) {
        return None;
    }
    let mut chars = id.chars();
    let bounded = chars.by_ref().take(MAX_MODEL_ID_CHARS).collect::<String>();
    (chars.next().is_none()).then_some(bounded)
}

fn normalize_loopback_base_url(value: &str) -> Option<String> {
    let mut url = Url::parse(value.trim()).ok()?;
    if url.scheme() != "http"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let host = url.host_str()?;
    if !matches!(host, "127.0.0.1" | "::1" | "[::1]" | "localhost") {
        return None;
    }
    match url.path().trim_end_matches('/') {
        "" => url.set_path("/v1"),
        "/v1" => url.set_path("/v1"),
        _ => return None,
    }
    Some(url.to_string().trim_end_matches('/').to_string())
}

fn process_endpoints() -> Vec<String> {
    process_command_lines()
        .into_iter()
        .filter_map(|arguments| endpoint_from_process_arguments(&arguments))
        .collect()
}

fn endpoint_from_process_arguments(arguments: &[String]) -> Option<String> {
    let executable = Path::new(arguments.first()?)
        .file_name()?
        .to_string_lossy()
        .to_ascii_lowercase();
    if !matches!(
        executable.as_str(),
        "llama-server" | "llama-server.exe" | "server"
    ) || (executable == "server"
        && !arguments
            .first()
            .is_some_and(|value| value.to_ascii_lowercase().contains("llama")))
    {
        return None;
    }

    let port = argument_value(arguments, "--port")
        .or_else(|| argument_value(arguments, "-p"))
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(8080);
    let host = argument_value(arguments, "--host").unwrap_or("127.0.0.1");
    let host = match host {
        "0.0.0.0" | "*" => "127.0.0.1",
        "::" => "::1",
        value if matches!(value, "127.0.0.1" | "::1" | "[::1]" | "localhost") => value,
        _ => return None,
    };
    let url_host = if host == "[::1]" {
        host.to_string()
    } else if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    normalize_loopback_base_url(&format!("http://{url_host}:{port}/v1"))
}

fn argument_value<'a>(arguments: &'a [String], key: &str) -> Option<&'a str> {
    for (index, argument) in arguments.iter().enumerate() {
        if argument == key {
            return arguments.get(index + 1).map(String::as_str);
        }
        if let Some(value) = argument
            .strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
        {
            return Some(value);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn process_command_lines() -> Vec<Vec<String>> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .chars()
                .all(|character| character.is_ascii_digit())
        })
        .filter_map(|entry| {
            let comm = std::fs::read_to_string(entry.path().join("comm")).ok()?;
            if !comm.trim().eq_ignore_ascii_case("llama-server") {
                return None;
            }
            let command_line = std::fs::read(entry.path().join("cmdline")).ok()?;
            Some(
                command_line
                    .split(|byte| *byte == 0)
                    .filter(|part| !part.is_empty())
                    .map(|part| String::from_utf8_lossy(part).into_owned())
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn process_command_lines() -> Vec<Vec<String>> {
    let Ok(output) = std::process::Command::new("pgrep")
        .args(["-lf", "llama-server"])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut arguments = split_quoted_command_line(line);
            if arguments
                .first()
                .is_some_and(|value| value.chars().all(|character| character.is_ascii_digit()))
            {
                arguments.remove(0);
            }
            (!arguments.is_empty()).then_some(arguments)
        })
        .collect()
}

#[cfg(target_os = "windows")]
fn process_command_lines() -> Vec<Vec<String>> {
    let Ok(output) = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_Process -Filter \"Name='llama-server.exe'\" | Select-Object -ExpandProperty CommandLine",
        ])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains("llama-server"))
        .map(split_quoted_command_line)
        .collect()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn split_quoted_command_line(line: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in line.chars() {
        match character {
            '"' => quoted = !quoted,
            value if value.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    arguments.push(std::mem::take(&mut current));
                }
            }
            value => current.push(value),
        }
    }
    if !current.is_empty() {
        arguments.push(current);
    }
    arguments
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn process_command_lines() -> Vec<Vec<String>> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// Pi Provider Configuration
// ---------------------------------------------------------------------------

pub(crate) fn pi_models_config(server: &LlamaCppServer) -> Value {
    let models = server
        .models
        .iter()
        .map(|model| {
            let mut value = json!({
                "id": model.id,
                "name": format!("{} (Local)", model.id),
                "reasoning": server.supports_reasoning,
                "input": if server.supports_vision {
                    vec!["text", "image"]
                } else {
                    vec!["text"]
                },
                "cost": {
                    "input": 0,
                    "output": 0,
                    "cacheRead": 0,
                    "cacheWrite": 0
                }
            });
            if let Some(context_window) = model.context_window {
                value["contextWindow"] = json!(context_window);
                value["maxTokens"] = json!(context_window.saturating_div(4).clamp(1_024, 16_384));
            }
            value
        })
        .collect::<Vec<_>>();
    json!({
        "providers": {
            "llamacpp": {
                "name": "llama.cpp (Local)",
                "baseUrl": server.base_url,
                "api": "openai-completions",
                "apiKey": "local",
                "compat": {
                    "supportsDeveloperRole": false,
                    "supportsReasoningEffort": false
                },
                "models": models
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

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
}
