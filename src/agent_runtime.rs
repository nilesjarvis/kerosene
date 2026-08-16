use crate::agent_state::AgentPrompt;

use futures::channel::mpsc;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock, mpsc as std_mpsc};
use std::time::Duration;
use zeroize::Zeroizing;

const EXTENSION_SOURCE: &str = include_str!("../assets/agent/kerosene.ts");
const RUNTIME_POLL_INTERVAL: Duration = Duration::from_millis(100);
const PI_RPC_ARGS: [&str; 3] = ["--mode", "rpc", "--no-session"];
const PI_TOOL_ALLOWLIST: &str = "kerosene_data,kerosene_market_data,kerosene_activity,kerosene_journal,kerosene_calculate,kerosene_risk,kerosene_positioning,kerosene_ohlcv,kerosene_sessions";

// ---------------------------------------------------------------------------
// Pi RPC Runtime
// ---------------------------------------------------------------------------

pub(crate) struct AgentRuntimeConfig {
    pub(crate) generation: u64,
    pub(crate) model: String,
    pub(crate) api_key: Zeroizing<String>,
    pub(crate) hyperdash_api_key: Zeroizing<String>,
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
        detail: Option<String>,
    },
    ToolFinished {
        generation: u64,
        call_id: String,
        is_error: bool,
    },
    ModelContext {
        generation: u64,
        model: Option<String>,
        context_window: Option<u64>,
    },
    ContextUsage {
        generation: u64,
        context_tokens: Option<u64>,
        context_window: Option<u64>,
    },
    Settled {
        generation: u64,
        total_tokens: Option<u64>,
        total_cost_usd: Option<f64>,
        has_visible_text: Option<bool>,
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
            | Self::ModelContext { generation, .. }
            | Self::ContextUsage { generation, .. }
            | Self::Settled { generation, .. }
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
            Self::ModelContext {
                generation,
                model,
                context_window,
            } => f
                .debug_struct("ModelContext")
                .field("generation", generation)
                .field("model", &model.as_ref().map(|_| "<redacted>"))
                .field("context_window", context_window)
                .finish(),
            Self::ContextUsage {
                generation,
                context_tokens,
                context_window,
            } => f
                .debug_struct("ContextUsage")
                .field("generation", generation)
                .field("context_tokens", context_tokens)
                .field("context_window", context_window)
                .finish(),
            Self::Settled { generation, .. } => f.debug_tuple("Settled").field(generation).finish(),
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
    InspectContext,
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
        .args(PI_RPC_ARGS)
        .arg("--provider")
        .arg("openrouter")
        .arg("--model")
        .arg(config.model.trim())
        .arg("--tools")
        .arg(PI_TOOL_ALLOWLIST)
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
                let request = json!({
                    "type": "prompt",
                    "message": prompt.as_str(),
                });
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
        "agent_settled" => {
            let (total_tokens, total_cost_usd) = rpc_usage(value);
            Some(AgentRuntimeEvent::Settled {
                generation,
                total_tokens,
                total_cost_usd,
                has_visible_text: None,
            })
        }
        "agent_end" if value.get("willRetry").and_then(Value::as_bool) != Some(true) => {
            let (total_tokens, total_cost_usd) = agent_end_usage(value);
            Some(AgentRuntimeEvent::Settled {
                generation,
                total_tokens,
                total_cost_usd,
                has_visible_text: Some(agent_end_has_visible_text(value)),
            })
        }
        "message_update"
            if value
                .pointer("/assistantMessageEvent/type")
                .and_then(Value::as_str)
                == Some("text_delta") =>
        {
            let (total_tokens, total_cost_usd) = rpc_usage(value);
            Some(AgentRuntimeEvent::TextDelta {
                generation,
                delta: value
                    .pointer("/assistantMessageEvent/delta")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                total_tokens,
                total_cost_usd,
            })
        }
        "tool_execution_start" => {
            let name = value
                .get("toolName")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string();
            Some(AgentRuntimeEvent::ToolStarted {
                generation,
                call_id: value
                    .get("toolCallId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                detail: tool_call_detail(&name, value.get("args")),
                name,
            })
        }
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
        "response"
            if value.get("success").and_then(Value::as_bool) == Some(true)
                && value.get("command").and_then(Value::as_str) == Some("get_state") =>
        {
            Some(AgentRuntimeEvent::ModelContext {
                generation,
                model: value
                    .pointer("/data/model/id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                context_window: value
                    .pointer("/data/model/contextWindow")
                    .and_then(Value::as_u64),
            })
        }
        "response"
            if value.get("success").and_then(Value::as_bool) == Some(true)
                && value.get("command").and_then(Value::as_str) == Some("get_session_stats") =>
        {
            Some(AgentRuntimeEvent::ContextUsage {
                generation,
                context_tokens: value
                    .pointer("/data/contextUsage/tokens")
                    .and_then(Value::as_u64),
                context_window: value
                    .pointer("/data/contextUsage/contextWindow")
                    .and_then(Value::as_u64),
            })
        }
        "response"
            if value.get("success").and_then(Value::as_bool) == Some(false)
                && matches!(
                    value.get("command").and_then(Value::as_str),
                    Some("get_state" | "get_session_stats")
                ) =>
        {
            None
        }
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

fn tool_call_detail(name: &str, args: Option<&Value>) -> Option<String> {
    let args = args?.as_object()?;
    let field = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .map(|value| bounded_tool_value(value, 32))
    };
    let symbols = || summarized_symbols(args.get("symbols"));

    let segments = match name {
        "kerosene_data" => vec![field("section").map(|value| title_case(&value))?],
        "kerosene_market_data" => vec![symbols()?, "Current mids and market metadata".to_string()],
        "kerosene_activity" => {
            let mut segments = Vec::new();
            push_field(&mut segments, field("kind"), title_case);
            push_field(&mut segments, field("mode"), title_case);
            push_optional(&mut segments, field("symbol"));
            if let Some(limit) = args.get("limit").and_then(Value::as_u64) {
                segments.push(format!("Up to {limit} rows"));
            }
            segments
        }
        "kerosene_journal" => {
            let mut segments = Vec::new();
            push_field(&mut segments, field("operation"), journal_operation_label);
            push_field(&mut segments, field("metric"), metric_label);
            push_optional(&mut segments, field("symbol"));
            push_field(&mut segments, field("status"), title_case);
            if let Some(limit) = args.get("limit").and_then(Value::as_u64) {
                segments.push(format!("Up to {limit} trades"));
            }
            segments
        }
        "kerosene_calculate" => {
            let mut segments = Vec::new();
            push_field(&mut segments, field("operation"), calculation_label);
            push_optional(&mut segments, field("symbol"));
            push_optional(
                &mut segments,
                field("interval").map(|value| format!("{value} candles")),
            );
            if let Some(shock) = args.get("shock_pct").and_then(Value::as_f64) {
                segments.push(format!("{shock:+.1}% shock"));
            }
            segments
        }
        "kerosene_risk" => vec!["Clearinghouse, spot, portfolio, and income scopes".to_string()],
        "kerosene_positioning" => {
            let mut segments = Vec::new();
            push_optional(&mut segments, symbols());
            push_field(&mut segments, field("timeframe"), timeframe_label);
            segments
        }
        "kerosene_ohlcv" => {
            let mut segments = Vec::new();
            push_optional(&mut segments, field("symbol"));
            push_optional(
                &mut segments,
                field("interval").map(|value| format!("{value} candles")),
            );
            if let Some(limit) = args.get("limit").and_then(Value::as_u64) {
                segments.push(format!("Up to {limit} rows"));
            }
            segments
        }
        "kerosene_sessions" => {
            let mut segments = Vec::new();
            push_optional(&mut segments, field("symbol"));
            if let Some(days) = args.get("lookback_days").and_then(Value::as_u64) {
                segments.push(format!("{days}-day lookback"));
            }
            segments
        }
        _ => Vec::new(),
    };

    (!segments.is_empty()).then(|| segments.join(" · "))
}

fn push_optional(segments: &mut Vec<String>, value: Option<String>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        segments.push(value);
    }
}

fn push_field(
    segments: &mut Vec<String>,
    value: Option<String>,
    label: impl FnOnce(&str) -> String,
) {
    if let Some(value) = value {
        segments.push(label(&value));
    }
}

fn summarized_symbols(value: Option<&Value>) -> Option<String> {
    let symbols = value?.as_array()?;
    let visible = symbols
        .iter()
        .filter_map(Value::as_str)
        .take(3)
        .map(|symbol| bounded_tool_value(symbol, 24))
        .collect::<Vec<_>>();
    if visible.is_empty() {
        return None;
    }

    let hidden = symbols.len().saturating_sub(visible.len());
    let mut summary = visible.join(", ");
    if hidden > 0 {
        summary.push_str(&format!(" +{hidden}"));
    }
    Some(summary)
}

fn bounded_tool_value(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    let mut chars = value.chars();
    let mut bounded = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        let _ = bounded.pop();
        bounded.push('…');
    }
    bounded
}

fn title_case(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn journal_operation_label(value: &str) -> String {
    match value {
        "best" => "Best trades".to_string(),
        "worst" => "Worst trades".to_string(),
        "summary" => "Performance summary".to_string(),
        "list" => "Recent trades".to_string(),
        _ => title_case(value),
    }
}

fn metric_label(value: &str) -> String {
    match value {
        "net_pnl" => "Net PnL".to_string(),
        "gross_pnl" => "Gross PnL".to_string(),
        "return_on_entry_pct" => "Return on entry".to_string(),
        "net_pnl_per_volume_pct" => "Net PnL per volume".to_string(),
        _ => title_case(value),
    }
}

fn calculation_label(value: &str) -> String {
    match value {
        "exposure" => "Exposure".to_string(),
        "liquidation_buffers" => "Liquidation buffers".to_string(),
        "stress" => "Stress test".to_string(),
        "fill_aggregation" => "Fill aggregation".to_string(),
        "funding_aggregation" => "Funding aggregation".to_string(),
        "portfolio_reconciliation" => "Portfolio reconciliation".to_string(),
        "market_statistics" => "Market statistics".to_string(),
        _ => title_case(value),
    }
}

fn timeframe_label(value: &str) -> String {
    match value {
        "FIFTEEN_MINUTES" => "15-minute change".to_string(),
        "ONE_HOUR" => "1-hour change".to_string(),
        "FOUR_HOURS" => "4-hour change".to_string(),
        _ => title_case(value),
    }
}

fn rpc_usage(value: &Value) -> (Option<u64>, Option<f64>) {
    let usage = value
        .get("usage")
        .or_else(|| value.pointer("/message/usage"));
    let total_tokens = usage
        .and_then(|usage| usage.get("totalTokens"))
        .and_then(Value::as_u64);
    let total_cost_usd = usage
        .and_then(|usage| usage.pointer("/cost/total"))
        .and_then(Value::as_f64);
    (total_tokens, total_cost_usd)
}

fn agent_end_usage(value: &Value) -> (Option<u64>, Option<f64>) {
    let Some(messages) = value.get("messages").and_then(Value::as_array) else {
        return rpc_usage(value);
    };

    let mut total_tokens = None::<u64>;
    let mut total_cost_usd = None::<f64>;
    for message in messages {
        let (message_tokens, message_cost_usd) = rpc_usage(message);
        if let Some(message_tokens) = message_tokens {
            total_tokens = Some(
                total_tokens
                    .unwrap_or_default()
                    .saturating_add(message_tokens),
            );
        }
        if let Some(message_cost_usd) = message_cost_usd {
            total_cost_usd = Some(total_cost_usd.unwrap_or_default() + message_cost_usd);
        }
    }
    (total_tokens, total_cost_usd)
}

fn agent_end_has_visible_text(value: &Value) -> bool {
    value
        .get("messages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .rev()
        .find(|message| message.get("role").and_then(Value::as_str) == Some("assistant"))
        .is_some_and(|message| {
            if let Some(text) = message.get("content").and_then(Value::as_str) {
                return !text.trim().is_empty();
            }
            message
                .get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .any(|part| {
                    part.get("type").and_then(Value::as_str) == Some("text")
                        && part
                            .get("text")
                            .and_then(Value::as_str)
                            .is_some_and(|text| !text.trim().is_empty())
                })
        })
}

#[cfg(test)]
mod context_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_rpc_arguments_match_current_cli_contract() {
        assert_eq!(PI_RPC_ARGS, ["--mode", "rpc", "--no-session"]);
        let tools = PI_TOOL_ALLOWLIST.split(',').collect::<Vec<_>>();
        assert_eq!(tools.len(), 9);
        assert!(tools.contains(&"kerosene_journal"));
        assert!(tools.iter().all(|tool| tool.starts_with("kerosene_")));
        assert!(
            !tools
                .iter()
                .any(|tool| matches!(*tool, "bash" | "read" | "write" | "edit"))
        );
    }

    #[test]
    fn packaged_pi_candidates_cover_supported_install_layouts() {
        let linux = packaged_pi_candidates(Path::new("/usr/bin"), "pi");
        let bundled_linux = PathBuf::from("/usr/lib/kerosene/pi/pi");
        let legacy_linux = PathBuf::from("/usr/bin/pi");
        assert!(linux.contains(&bundled_linux));
        assert!(
            linux.iter().position(|path| path == &bundled_linux)
                < linux.iter().position(|path| path == &legacy_linux)
        );

        let macos =
            packaged_pi_candidates(Path::new("/Applications/Kerosene.app/Contents/MacOS"), "pi");
        assert!(macos.contains(&PathBuf::from(
            "/Applications/Kerosene.app/Contents/Resources/pi/pi"
        )));

        let portable = packaged_pi_candidates(Path::new("/opt/kerosene"), "pi.exe");
        assert!(portable.contains(&PathBuf::from("/opt/kerosene/pi/pi.exe")));
    }

    #[test]
    fn embedded_extension_contains_evidence_and_validation_contracts() {
        for requirement in [
            "Ground every material claim in evidence retrieved during the current turn.",
            "Never present an inference, hypothesis, or prior-turn value as a current fact.",
            "If sources conflict, expose the conflict instead of silently choosing one.",
            "Do not invent confidence percentages",
            "market_statistics",
            "excluded instead of treated as zero",
        ] {
            assert!(
                EXTENSION_SOURCE.contains(requirement),
                "missing embedded Assistant contract: {requirement}"
            );
        }
        assert!(
            !EXTENSION_SOURCE.contains("numericOrZero"),
            "financial tool code must not silently coerce missing values to zero"
        );
    }

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
    fn parses_nested_message_usage_from_current_pi_events() {
        let value = json!({
            "type": "message_update",
            "assistantMessageEvent": { "type": "text_delta", "delta": "reply" },
            "message": {
                "usage": { "totalTokens": 84, "cost": { "total": 0.002 } }
            }
        });

        assert!(matches!(
            parse_rpc_event(8, &value),
            Some(AgentRuntimeEvent::TextDelta {
                generation: 8,
                total_tokens: Some(84),
                total_cost_usd: Some(cost),
                ..
            }) if (cost - 0.002).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn parses_tool_call_with_human_readable_request_detail() {
        let value = json!({
            "type": "tool_execution_start",
            "toolCallId": "call-1",
            "toolName": "kerosene_ohlcv",
            "args": {
                "symbol": "BTC",
                "interval": "1h",
                "limit": 200,
                "private_key": "must-not-be-rendered"
            }
        });

        assert!(matches!(
            parse_rpc_event(12, &value),
            Some(AgentRuntimeEvent::ToolStarted {
                generation: 12,
                call_id,
                name,
                detail: Some(detail),
            }) if call_id == "call-1"
                && name == "kerosene_ohlcv"
                && detail == "BTC · 1h candles · Up to 200 rows"
                && !detail.contains("must-not-be-rendered")
        ));
    }

    #[test]
    fn tool_call_detail_compacts_long_symbol_lists() {
        let args = json!({
            "symbols": ["BTC", "ETH", "SOL", "HYPE", "DOGE"]
        });

        assert_eq!(
            tool_call_detail("kerosene_market_data", Some(&args)).as_deref(),
            Some("BTC, ETH, SOL +2 · Current mids and market metadata")
        );
    }

    #[test]
    fn tool_runtime_debug_omits_request_detail() {
        let event = AgentRuntimeEvent::ToolStarted {
            generation: 1,
            call_id: "call-1".to_string(),
            name: "kerosene_data".to_string(),
            detail: Some("private account context".to_string()),
        };
        let debug = format!("{event:?}");
        assert!(!debug.contains("private account context"));
        assert!(debug.contains("kerosene_data"));
    }

    #[test]
    fn current_pi_agent_end_settles_and_aggregates_session_usage() {
        let value = json!({
            "type": "agent_end",
            "willRetry": false,
            "messages": [
                { "role": "user", "content": [] },
                {
                    "role": "assistant",
                    "usage": { "totalTokens": 50, "cost": { "total": 0.001 } }
                },
                {
                    "role": "assistant",
                    "usage": { "totalTokens": 25, "cost": { "total": 0.0005 } }
                }
            ]
        });

        assert!(matches!(
            parse_rpc_event(9, &value),
            Some(AgentRuntimeEvent::Settled {
                generation: 9,
                total_tokens: Some(75),
                total_cost_usd: Some(cost),
                has_visible_text: Some(false),
            }) if (cost - 0.0015).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn retrying_agent_end_does_not_mark_the_turn_settled() {
        let value = json!({
            "type": "agent_end",
            "willRetry": true,
            "messages": []
        });

        assert!(parse_rpc_event(10, &value).is_none());
    }

    #[test]
    fn current_pi_agent_end_reports_visible_answer_text() {
        let value = json!({
            "type": "agent_end",
            "willRetry": false,
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        { "type": "thinking", "thinking": "hidden" },
                        { "type": "text", "text": "## Visible answer" }
                    ]
                }
            ]
        });

        assert!(matches!(
            parse_rpc_event(11, &value),
            Some(AgentRuntimeEvent::Settled {
                generation: 11,
                has_visible_text: Some(true),
                ..
            })
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
