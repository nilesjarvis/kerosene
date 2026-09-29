#[cfg(not(target_os = "windows"))]
use std::process::Command;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

pub(super) fn agent_link_is_allowed(uri: &str) -> bool {
    let uri = uri.to_ascii_lowercase();
    uri.starts_with("https://") || uri.starts_with("http://")
}

pub(super) async fn open_agent_link(uri: String) -> Result<(), String> {
    open_agent_link_with_system(&uri)
}

#[cfg(target_os = "macos")]
fn open_agent_link_with_system(uri: &str) -> Result<(), String> {
    Command::new("open")
        .arg(uri)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("open command failed: {error}"))
}

#[cfg(target_os = "windows")]
fn open_agent_link_with_system(uri: &str) -> Result<(), String> {
    let operation = "open\0".encode_utf16().collect::<Vec<_>>();
    let uri = uri
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            uri.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    let status = result as isize;
    if status > 32 {
        Ok(())
    } else {
        Err(format!("Windows URL launch failed with status {status}"))
    }
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn open_agent_link_with_system(uri: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(uri)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("xdg-open command failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_links_only_allow_http_schemes() {
        assert!(agent_link_is_allowed("https://example.com/report"));
        assert!(agent_link_is_allowed("HTTP://example.com/report"));
        assert!(!agent_link_is_allowed("file:///tmp/private"));
        assert!(!agent_link_is_allowed("javascript:alert(1)"));
    }
}
