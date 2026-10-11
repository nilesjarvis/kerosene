#[cfg(not(target_os = "windows"))]
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

#[cfg(not(target_os = "windows"))]
pub(super) fn open(url: &str) -> Result<(), String> {
    let command = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut child = Command::new(command)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Could not open your browser for wallet approval.".to_string())?;
    // Reap the short-lived platform launcher without holding up the listener.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(target_os = "windows")]
pub(super) fn open(url: &str) -> Result<(), String> {
    let operation = "open\0".encode_utf16().collect::<Vec<_>>();
    let url = url
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize > 32 {
        Ok(())
    } else {
        Err("Could not open your browser for wallet approval.".to_string())
    }
}
