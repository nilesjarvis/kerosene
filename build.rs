use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=assets/kerosene.png");
    println!("cargo:rerun-if-changed=Cargo.toml");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }

    let icon_path = generated_icon_path()?;
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(icon_path.to_string_lossy().as_ref())
        .set("InternalName", "kerosene.exe")
        .set("OriginalFilename", "kerosene.exe")
        .set("ProductName", "Kerosene")
        .set("FileDescription", "Kerosene Trading Terminal")
        .set("CompanyName", "Kerosene Contributors")
        .set("LegalCopyright", "Copyright 2026 Kerosene Contributors")
        .set_manifest(windows_manifest());
    resource.compile()
}

fn generated_icon_path() -> io::Result<PathBuf> {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "CARGO_MANIFEST_DIR is unavailable")
    })?);
    let png_path = manifest_dir.join("assets").join("kerosene.png");
    let png = fs::read(&png_path)?;

    let out_dir = PathBuf::from(
        env::var_os("OUT_DIR")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "OUT_DIR is unavailable"))?,
    );
    let ico_path = out_dir.join("kerosene.ico");
    write_png_backed_ico(&ico_path, &png)?;
    Ok(ico_path)
}

fn write_png_backed_ico(path: &Path, png: &[u8]) -> io::Result<()> {
    const ICO_HEADER_LEN: u32 = 22;
    let png_len = u32::try_from(png.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "icon PNG is too large"))?;
    let mut ico = Vec::with_capacity(ICO_HEADER_LEN as usize + png.len());

    ico.extend_from_slice(&0_u16.to_le_bytes()); // Reserved.
    ico.extend_from_slice(&1_u16.to_le_bytes()); // Icon resource.
    ico.extend_from_slice(&1_u16.to_le_bytes()); // One image.
    ico.push(0); // Width 256.
    ico.push(0); // Height 256.
    ico.push(0); // No palette.
    ico.push(0); // Reserved.
    ico.extend_from_slice(&1_u16.to_le_bytes()); // Color planes.
    ico.extend_from_slice(&32_u16.to_le_bytes()); // Bits per pixel.
    ico.extend_from_slice(&png_len.to_le_bytes());
    ico.extend_from_slice(&ICO_HEADER_LEN.to_le_bytes());
    ico.extend_from_slice(png);

    fs::write(path, ico)
}

fn windows_manifest() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2, PerMonitor</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>"#
}
