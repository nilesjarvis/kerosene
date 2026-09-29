use reqwest::Url;
use std::collections::HashSet;
use std::path::Path;

const DEFAULT_LLAMA_CPP_PORTS: [u16; 2] = [8080, 8081];
const MAX_DISCOVERED_ENDPOINTS: usize = 12;

pub(super) fn detection_candidates() -> Result<Vec<String>, String> {
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
        "" | "/v1" => url.set_path("/v1"),
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

#[cfg(test)]
mod tests;
