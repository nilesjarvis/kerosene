use crate::agent_state::AgentPrompt;

use futures::channel::mpsc;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock, mpsc as std_mpsc};
use std::time::Duration;
use zeroize::Zeroizing;

const EXTENSION_SOURCE: &str = include_str!("../assets/agent/kerosene.ts");
const RUNTIME_POLL_INTERVAL: Duration = Duration::from_millis(100);

// ---------------------------------------------------------------------------
// Pi RPC Runtime
// ---------------------------------------------------------------------------

pub(crate) struct AgentRuntimeConfig {
    pub(crate) generation: u64,
    pub(crate) model: String,
    pub(crate) api_key: Zeroizing<String>,
    pub(crate) workspace_dir: PathBuf,
}

#[derive(Clone)]
pub(crate) enum AgentRuntimeEvent {
    Ready {
        generation: u64,
    },
    Thinking {
        generation: u64,
    },
    TextDelta {
        generation: u64,
        delta: String,
        total_tokens: Option<u64>,
        total_cost_usd: Option<f64>,
    },
    ToolStarted {
        generation: u64,
        call_id: String,
        name: String,
    },
    ToolFinished {
        generation: u64,
        call_id: String,
        is_error: bool,
    },
    Settled {
        generation: u64,
    },
    Error {
        generation: u64,
        message: String,
    },
    Exited {
        generation: u64,
    },
}

impl AgentRuntimeEvent {
    pub(crate) fn generation(&self) -> u64 {
        match self {
            Self::Ready { generation }
            | Self::Thinking { generation }
            | Self::TextDelta { generation, .. }
            | Self::ToolStarted { generation, .. }
            | Self::ToolFinished { generation, .. }
            | Self::Settled { generation }
            | Self::Error { generation, .. }
            | Self::Exited { generation } => *generation,
        }
    }
}

impl fmt::Debug for AgentRuntimeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready { generation } => f.debug_tuple("Ready").field(generation).finish(),
            Self::Thinking { generation } => f.debug_tuple("Thinking").field(generation).finish(),
            Self::TextDelta { generation, .. } => f
                .debug_struct("TextDelta")
                .field("generation", generation)
                .field("delta", &"<redacted>")
                .finish(),
            Self::ToolStarted {
                generation, name, ..
            } => f
                .debug_struct("ToolStarted")
                .field("generation", generation)
                .field("name", name)
                .finish(),
            Self::ToolFinished {
                generation,
                is_error,
                ..
            } => f
                .debug_struct("ToolFinished")
                .field("generation", generation)
                .field("is_error", is_error)
                .finish(),
            Self::Settled { generation } => f.debug_tuple("Settled").field(generation).finish(),
            Self::Error { generation, .. } => f
                .debug_struct("Error")
                .field("generation", generation)
                .field("message", &"<redacted>")
                .finish(),
            Self::Exited { generation } => f.debug_tuple("Exited").field(generation).finish(),
        }
    }
}

enum AgentRuntimeCommand {
    Prompt(AgentPrompt),
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

    let mut command = Command::new(pi_binary());
    command
        .args(["--mode", "rpc", "--no-session", "--no-approve"])
        .arg("--provider")
        .arg("openrouter")
        .arg("--model")
        .arg(config.model.trim())
        .arg("--tools")
        .arg("kerosene_data")
        .arg("--extension")
        .arg(&extension_path)
        .current_dir(&config.workspace_dir)
        .env("OPENROUTER_API_KEY", config.api_key.as_str())
        .env("KEROSENE_AGENT_SNAPSHOT", &snapshot_path)
        .env("PI_CODING_AGENT_DIR", &pi_config_dir)
        .env("PI_SKIP_VERSION_CHECK", "1")
        .env("PI_TELEMETRY", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let message = if error.kind() == std::io::ErrorKind::NotFound {
                "Pi was not found. Install @earendil-works/pi-coding-agent or set KEROSENE_PI_BINARY to the Pi executable.".to_string()
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
                let request = json!({
                    "type": "prompt",
                    "message": prompt.as_str(),
                });
                if write_rpc_command(&mut stdin, &request).is_err() {
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

fn pi_binary() -> OsString {
    if let Some(binary) = std::env::var_os("KEROSENE_PI_BINARY") {
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
        let candidates = [
            binary_dir.join(executable_name),
            binary_dir.join("resources").join(executable_name),
            binary_dir
                .parent()
                .map(|parent| parent.join("Resources").join(executable_name))
                .unwrap_or_default(),
        ];
        if let Some(candidate) = candidates.into_iter().find(|path| path.is_file()) {
            return candidate.into_os_string();
        }
    }

    OsString::from(executable_name)
}

fn write_rpc_command(stdin: &mut impl Write, value: &Value) -> std::io::Result<()> {
    serde_json::to_writer(&mut *stdin, value)?;
    stdin.write_all(b"\n")?;
    stdin.flush()
}

fn emit(sender: &mpsc::UnboundedSender<AgentRuntimeEvent>, event: AgentRuntimeEvent) {
    let _ = sender.unbounded_send(event);
}

fn parse_rpc_event(generation: u64, value: &Value) -> Option<AgentRuntimeEvent> {
    match value.get("type")?.as_str()? {
        "agent_start" => Some(AgentRuntimeEvent::Thinking { generation }),
        "agent_settled" => Some(AgentRuntimeEvent::Settled { generation }),
        "message_update"
            if value
                .pointer("/assistantMessageEvent/type")
                .and_then(Value::as_str)
                == Some("text_delta") =>
        {
            Some(AgentRuntimeEvent::TextDelta {
                generation,
                delta: value
                    .pointer("/assistantMessageEvent/delta")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                total_tokens: value.pointer("/usage/totalTokens").and_then(Value::as_u64),
                total_cost_usd: value.pointer("/usage/cost/total").and_then(Value::as_f64),
            })
        }
        "tool_execution_start" => Some(AgentRuntimeEvent::ToolStarted {
            generation,
            call_id: value
                .get("toolCallId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            name: value
                .get("toolName")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string(),
        }),
        "tool_execution_end" => Some(AgentRuntimeEvent::ToolFinished {
            generation,
            call_id: value
                .get("toolCallId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            is_error: value
                .get("isError")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        "response" if value.get("success").and_then(Value::as_bool) == Some(false) => {
            Some(AgentRuntimeEvent::Error {
                generation,
                message: value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("Pi rejected the request")
                    .to_string(),
            })
        }
        "extension_error" => Some(AgentRuntimeEvent::Error {
            generation,
            message: value
                .get("error")
                .and_then(Value::as_str)
                .or_else(|| value.get("message").and_then(Value::as_str))
                .unwrap_or("The Kerosene Pi extension failed")
                .to_string(),
        }),
        "message_end"
            if value.pointer("/message/stopReason").and_then(Value::as_str) == Some("error") =>
        {
            Some(AgentRuntimeEvent::Error {
                generation,
                message: value
                    .pointer("/message/errorMessage")
                    .and_then(Value::as_str)
                    .unwrap_or("The model request failed")
                    .to_string(),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_delta_and_usage() {
        let value = json!({
            "type": "message_update",
            "usage": { "totalTokens": 42, "cost": { "total": 0.001 } },
            "assistantMessageEvent": { "type": "text_delta", "delta": "secret reply" }
        });

        assert!(matches!(
            parse_rpc_event(7, &value),
            Some(AgentRuntimeEvent::TextDelta {
                generation: 7,
                delta,
                total_tokens: Some(42),
                total_cost_usd: Some(cost),
            }) if delta == "secret reply" && (cost - 0.001).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn runtime_event_debug_redacts_model_text() {
        let event = AgentRuntimeEvent::TextDelta {
            generation: 1,
            delta: "private portfolio answer".to_string(),
            total_tokens: None,
            total_cost_usd: None,
        };
        let debug = format!("{event:?}");
        assert!(!debug.contains("private portfolio answer"));
        assert!(debug.contains("<redacted>"));
    }
}
