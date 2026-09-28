use crate::agent_state::AgentPrompt;
use crate::config::AssistantProvider;
use crate::llama_cpp::{LlamaCppServer, pi_models_config};

use futures::channel::mpsc;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock, mpsc as std_mpsc};
use std::time::Duration;

mod model;
mod rpc;
mod tool_details;

pub(crate) use model::{AgentRuntimeConfig, AgentRuntimeEvent};
use rpc::{extension_ui_response_value, parse_rpc_event, write_rpc_command};

const EXTENSION_SOURCE: &str = include_str!("../assets/agent/kerosene.ts");
const RUNTIME_POLL_INTERVAL: Duration = Duration::from_millis(100);
const PI_RPC_ARGS: [&str; 5] = ["--mode", "rpc", "--no-session", "--thinking", "medium"];
const PI_TOOL_ALLOWLIST: &str = "kerosene_data,kerosene_set_chart_indicators,kerosene_manage_chart_drawings,kerosene_market_data,kerosene_activity,kerosene_journal,kerosene_calculate,kerosene_risk,kerosene_positioning,kerosene_pnl_card_match,kerosene_ohlcv,kerosene_sessions";

// ---------------------------------------------------------------------------
// Pi RPC Runtime
// ---------------------------------------------------------------------------

enum AgentRuntimeCommand {
    Prompt(AgentPrompt),
    InspectContext,
    ExtensionUiResponse {
        request_id: String,
        value: Option<String>,
    },
    Abort,
    Shutdown,
}

struct ActiveRuntime {
    generation: u64,
    sender: std_mpsc::Sender<AgentRuntimeCommand>,
}

fn active_runtime() -> &'static Mutex<Option<ActiveRuntime>> {
    static ACTIVE_RUNTIME: OnceLock<Mutex<Option<ActiveRuntime>>> = OnceLock::new();
    ACTIVE_RUNTIME.get_or_init(|| Mutex::new(None))
}

pub(crate) fn runtime_stream(
    config: AgentRuntimeConfig,
) -> mpsc::UnboundedReceiver<AgentRuntimeEvent> {
    let (event_sender, event_receiver) = mpsc::unbounded();
    let (command_sender, command_receiver) = std_mpsc::channel();
    let generation = config.generation;

    if let Ok(mut active) = active_runtime().lock()
        && let Some(previous) = active.replace(ActiveRuntime {
            generation: config.generation,
            sender: command_sender,
        })
    {
        let _ = previous.sender.send(AgentRuntimeCommand::Shutdown);
    }

    let spawn_error_sender = event_sender.clone();
    if let Err(error) = std::thread::Builder::new()
        .name("kerosene-pi-rpc".to_string())
        .spawn(move || run_runtime(config, command_receiver, event_sender))
    {
        emit(
            &spawn_error_sender,
            AgentRuntimeEvent::Error {
                generation,
                message: format!("Could not create the Pi runtime thread: {error}"),
            },
        );
    }

    event_receiver
}

pub(crate) fn send_prompt(generation: u64, prompt: AgentPrompt) -> Result<(), String> {
    send_command(generation, AgentRuntimeCommand::Prompt(prompt))
}

pub(crate) fn abort(generation: u64) {
    let _ = send_command(generation, AgentRuntimeCommand::Abort);
}

pub(crate) fn inspect_context(generation: u64) -> Result<(), String> {
    send_command(generation, AgentRuntimeCommand::InspectContext)
}

pub(crate) fn respond_to_extension_ui(
    generation: u64,
    request_id: String,
    value: Option<String>,
) -> Result<(), String> {
    send_command(
        generation,
        AgentRuntimeCommand::ExtensionUiResponse { request_id, value },
    )
}

pub(crate) fn shutdown(generation: u64) {
    let _ = send_command(generation, AgentRuntimeCommand::Shutdown);
    if let Ok(mut active) = active_runtime().lock()
        && active
            .as_ref()
            .is_some_and(|runtime| runtime.generation == generation)
    {
        *active = None;
    }
}

fn send_command(generation: u64, command: AgentRuntimeCommand) -> Result<(), String> {
    let active = active_runtime()
        .lock()
        .map_err(|_| "Pi runtime coordinator is unavailable".to_string())?;
    let runtime = active
        .as_ref()
        .filter(|runtime| runtime.generation == generation)
        .ok_or_else(|| "Pi runtime is not running".to_string())?;
    runtime
        .sender
        .send(command)
        .map_err(|_| "Pi runtime stopped unexpectedly".to_string())
}

fn run_runtime(
    config: AgentRuntimeConfig,
    command_receiver: std_mpsc::Receiver<AgentRuntimeCommand>,
    event_sender: mpsc::UnboundedSender<AgentRuntimeEvent>,
) {
    let generation = config.generation;
    let extension_path = config.workspace_dir.join("kerosene-extension.ts");
    if let Err(error) = prepare_runtime_files(&config.workspace_dir, &extension_path) {
        emit(
            &event_sender,
            AgentRuntimeEvent::Error {
                generation,
                message: error,
            },
        );
        return;
    }

    let snapshot_path = config.workspace_dir.join("snapshot.json");
    let pi_config_dir = config.workspace_dir.join("pi-config");
    if let Err(error) = std::fs::create_dir_all(&pi_config_dir) {
        emit(
            &event_sender,
            AgentRuntimeEvent::Error {
                generation,
                message: format!("Could not prepare Pi configuration: {error}"),
            },
        );
        return;
    }

    if config.provider == AssistantProvider::LlamaCpp {
        let Some(server) = config.local_server.as_ref() else {
            emit(
                &event_sender,
                AgentRuntimeEvent::Error {
                    generation,
                    message: "The selected local llama.cpp server is no longer available"
                        .to_string(),
                },
            );
            return;
        };
        if let Err(error) = write_local_models_config(&pi_config_dir, server) {
            emit(
                &event_sender,
                AgentRuntimeEvent::Error {
                    generation,
                    message: error,
                },
            );
            return;
        }
    }

    let mut command = Command::new(pi_binary());
    command
        .args(PI_RPC_ARGS)
        .arg("--provider")
        .arg(match config.provider {
            AssistantProvider::OpenRouter => "openrouter",
            AssistantProvider::LlamaCpp => "llamacpp",
        })
        .arg("--model")
        .arg(config.model.trim())
        .arg("--tools")
        .arg(PI_TOOL_ALLOWLIST)
        .arg("--extension")
        .arg(&extension_path)
        .current_dir(&config.workspace_dir)
        .env_remove("OPENROUTER_API_KEY")
        .env("KEROSENE_AGENT_SNAPSHOT", &snapshot_path)
        .env("PI_CODING_AGENT_DIR", &pi_config_dir)
        .env("PI_SKIP_VERSION_CHECK", "1")
        .env("PI_TELEMETRY", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if config.provider == AssistantProvider::OpenRouter {
        command.env("OPENROUTER_API_KEY", config.api_key.as_str());
    }
    if !config.hyperdash_api_key.trim().is_empty() {
        command.env(
            "KEROSENE_AGENT_HYPERDASH_API_KEY",
            config.hyperdash_api_key.as_str(),
        );
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let message = if error.kind() == std::io::ErrorKind::NotFound {
                "The Kerosene Assistant component is missing or could not be launched. Reinstall or update Kerosene. Developers can set KEROSENE_PI_BINARY to a Pi executable.".to_string()
            } else {
                format!("Could not start Pi: {error}")
            };
            emit(
                &event_sender,
                AgentRuntimeEvent::Error {
                    generation,
                    message,
                },
            );
            return;
        }
    };

    let Some(mut stdin) = child.stdin.take() else {
        emit(
            &event_sender,
            AgentRuntimeEvent::Error {
                generation,
                message: "Pi did not expose an RPC input stream".to_string(),
            },
        );
        return;
    };
    let Some(stdout) = child.stdout.take() else {
        emit(
            &event_sender,
            AgentRuntimeEvent::Error {
                generation,
                message: "Pi did not expose an RPC output stream".to_string(),
            },
        );
        return;
    };

    let stdout_sender = event_sender.clone();
    let stdout_thread = std::thread::spawn(move || {
        for line in BufReader::new(stdout).split(b'\n') {
            let Ok(mut line) = line else {
                break;
            };
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if line.is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_slice::<Value>(&line)
                && let Some(event) = parse_rpc_event(generation, &value)
            {
                emit(&stdout_sender, event);
            }
        }
    });

    let stderr_thread = child.stderr.take().map(|stderr| {
        std::thread::spawn(move || {
            let mut recent = String::new();
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if !line.trim().is_empty() {
                    recent = line;
                }
            }
            recent
        })
    });

    emit(&event_sender, AgentRuntimeEvent::Ready { generation });

    let mut requested_shutdown = false;
    loop {
        match command_receiver.recv_timeout(RUNTIME_POLL_INTERVAL) {
            Ok(AgentRuntimeCommand::Prompt(prompt)) => {
                let images = prompt
                    .images()
                    .iter()
                    .map(|image| {
                        json!({
                            "type": "image",
                            "data": image.data.as_str(),
                            "mimeType": image.mime_type,
                        })
                    })
                    .collect::<Vec<_>>();
                let mut request = json!({
                    "type": "prompt",
                    "message": prompt.as_str(),
                });
                if !images.is_empty()
                    && let Some(request) = request.as_object_mut()
                {
                    request.insert("images".to_string(), Value::Array(images));
                }
                if write_rpc_command(&mut stdin, &request).is_err() {
                    break;
                }
            }
            Ok(AgentRuntimeCommand::InspectContext) => {
                if write_rpc_command(&mut stdin, &json!({ "type": "get_state" })).is_err()
                    || write_rpc_command(&mut stdin, &json!({ "type": "get_session_stats" }))
                        .is_err()
                {
                    break;
                }
            }
            Ok(AgentRuntimeCommand::ExtensionUiResponse { request_id, value }) => {
                let response = extension_ui_response_value(request_id, value);
                if write_rpc_command(&mut stdin, &response).is_err() {
                    break;
                }
            }
            Ok(AgentRuntimeCommand::Abort) => {
                let _ = write_rpc_command(&mut stdin, &json!({ "type": "abort" }));
            }
            Ok(AgentRuntimeCommand::Shutdown) | Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                let _ = write_rpc_command(&mut stdin, &json!({ "type": "abort" }));
                requested_shutdown = true;
                break;
            }
            Err(std_mpsc::RecvTimeoutError::Timeout) => {}
        }

        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(_) => break,
        }
    }

    drop(stdin);
    if requested_shutdown {
        let _ = child.kill();
    }
    let status = child.wait().ok();
    let _ = stdout_thread.join();
    let stderr = stderr_thread
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default();

    if !requested_shutdown && status.is_some_and(|status| !status.success()) {
        let detail = stderr.trim();
        let message = if detail.is_empty() {
            "Pi exited before completing the session".to_string()
        } else {
            format!("Pi exited: {detail}")
        };
        emit(
            &event_sender,
            AgentRuntimeEvent::Error {
                generation,
                message,
            },
        );
    }
    emit(&event_sender, AgentRuntimeEvent::Exited { generation });
}

fn prepare_runtime_files(workspace_dir: &PathBuf, extension_path: &PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(workspace_dir)
        .map_err(|error| format!("Could not create the assistant workspace: {error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(workspace_dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("Could not secure the assistant workspace: {error}"))?;
    }

    std::fs::write(extension_path, EXTENSION_SOURCE)
        .map_err(|error| format!("Could not prepare the Kerosene Pi extension: {error}"))
}

fn write_local_models_config(pi_config_dir: &Path, server: &LlamaCppServer) -> Result<(), String> {
    let path = pi_config_dir.join("models.json");
    let bytes = serde_json::to_vec_pretty(&pi_models_config(server))
        .map_err(|error| format!("Could not serialize the local model configuration: {error}"))?;
    let mut options = std::fs::OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("Could not prepare the local model configuration: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("Could not write the local model configuration: {error}"))
}

fn pi_binary() -> OsString {
    if let Some(binary) = std::env::var_os("KEROSENE_PI_BINARY")
        && !binary.is_empty()
    {
        return binary;
    }

    let executable_name = if cfg!(target_os = "windows") {
        "pi.exe"
    } else {
        "pi"
    };
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(binary_dir) = current_exe.parent()
    {
        let candidates = packaged_pi_candidates(binary_dir, executable_name);
        if let Some(candidate) = candidates.into_iter().find(|path| path.is_file()) {
            return candidate.into_os_string();
        }
    }

    OsString::from(executable_name)
}

fn packaged_pi_candidates(binary_dir: &Path, executable_name: &str) -> Vec<PathBuf> {
    let mut candidates = vec![
        binary_dir.join("pi").join(executable_name),
        binary_dir
            .join("resources")
            .join("pi")
            .join(executable_name),
    ];

    if let Some(prefix) = binary_dir.parent() {
        candidates.push(prefix.join("Resources").join("pi").join(executable_name));
        candidates.push(
            prefix
                .join("lib")
                .join("kerosene")
                .join("pi")
                .join(executable_name),
        );
        candidates.push(prefix.join("Resources").join(executable_name));
    }

    candidates.push(binary_dir.join(executable_name));
    candidates.push(binary_dir.join("resources").join(executable_name));

    candidates
}

fn emit(sender: &mpsc::UnboundedSender<AgentRuntimeEvent>, event: AgentRuntimeEvent) {
    let _ = sender.unbounded_send(event);
}

#[cfg(test)]
mod tests;
