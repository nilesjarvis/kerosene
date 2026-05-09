#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION=$(grep '^version' "$ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')
ARCH=$(uname -m)

RED='\033[0;31m'
GREEN='\033[0;32m'
BOLD='\033[1m'
RESET='\033[0m'

info()  { echo -e "${GREEN}[+]${RESET} $*"; }
error() { echo -e "${RED}[!]${RESET} $*" >&2; }
bold()  { echo -e "${BOLD}$*${RESET}"; }

check_free_space() {
    local path="$1"
    local min_mb="$2"
    local avail_kb
    avail_kb=$(df -Pk "$path" | awk 'NR==2 {print $4}')
    local avail_mb=$((avail_kb / 1024))
    if [ "$avail_mb" -lt "$min_mb" ]; then
        error "Low disk space: ${avail_mb}MB available, ${min_mb}MB required"
        error "Try: cargo clean"
        return 1
    fi
}

usage() {
    cat <<EOF
Usage: $(basename "$0") [deb|appimage|exe|all]

Build distributable packages for Kerosene.

Commands:
  deb        Build a .deb package (requires cargo-deb)
  appimage   Build an .AppImage (requires appimagetool)
  exe        Build a Windows .exe (requires mingw-w64)
  all        Build .deb, .AppImage, and .exe (default)

Options:
  -h, --help   Show this help message

The release binary is built automatically if not already present.
Output files are placed in target/.
EOF
    exit 0
}

# -----------------------------------------------------------------------
# Build release binary if needed
# -----------------------------------------------------------------------
ensure_release_binary() {
    if [ ! -f "$ROOT/target/release/kerosene" ]; then
        info "Building release binary..."
        cargo build --release --manifest-path "$ROOT/Cargo.toml"
    else
        info "Release binary already exists, skipping build."
        info "Run 'cargo build --release' manually to rebuild."
    fi
}

# -----------------------------------------------------------------------
# .deb
# -----------------------------------------------------------------------
build_deb() {
    bold "=== Building .deb package ==="

    if ! command -v cargo-deb &>/dev/null; then
        info "Installing cargo-deb..."
        cargo install cargo-deb
    fi

    ensure_release_binary

    info "Packaging .deb..."
    cargo deb --no-build --manifest-path "$ROOT/Cargo.toml"

    DEB=$(ls -t "$ROOT/target/debian/"*.deb 2>/dev/null | head -1)
    if [ -n "$DEB" ]; then
        info "Done: $DEB ($(du -h "$DEB" | cut -f1))"
    else
        error "Failed to find .deb output"
        return 1
    fi
}

# -----------------------------------------------------------------------
# .AppImage
# -----------------------------------------------------------------------
build_appimage() {
    bold "=== Building .AppImage ==="

    # Locate or download appimagetool
    APPIMAGETOOL=""
    if command -v appimagetool &>/dev/null; then
        APPIMAGETOOL="appimagetool"
    elif [ -x "$ROOT/target/appimagetool" ]; then
        APPIMAGETOOL="$ROOT/target/appimagetool"
    else
        info "Downloading appimagetool..."
        mkdir -p "$ROOT/target"
        wget -q "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage" \
            -O "$ROOT/target/appimagetool"
        chmod +x "$ROOT/target/appimagetool"
        APPIMAGETOOL="$ROOT/target/appimagetool"
    fi

    ensure_release_binary

    # Create AppDir
    APPDIR=$(mktemp -d)
    trap "rm -rf '$APPDIR'" EXIT

    info "Assembling AppDir..."
    mkdir -p "$APPDIR/usr/bin"
    cp "$ROOT/target/release/kerosene" "$APPDIR/usr/bin/"
    cp "$ROOT/assets/kerosene.desktop"  "$APPDIR/"
    cp "$ROOT/assets/kerosene.png"      "$APPDIR/"

    # Icon in standard hicolor path (some desktop environments look here)
    mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
    cp "$ROOT/assets/kerosene.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/"

    cat > "$APPDIR/AppRun" << 'APPRUN'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"
exec "${HERE}/usr/bin/kerosene" "$@"
APPRUN
    chmod +x "$APPDIR/AppRun"

    OUTPUT="$ROOT/target/Kerosene-${VERSION}-${ARCH}.AppImage"
    # Remove previous AppImage to avoid "Text file busy" if it's still mapped
    rm -f "$OUTPUT"
    info "Running appimagetool..."
    ARCH="$ARCH" "$APPIMAGETOOL" --no-appstream "$APPDIR" "$OUTPUT" 2>&1 | tail -3

    if [ -f "$OUTPUT" ]; then
        chmod +x "$OUTPUT"
        info "Done: $OUTPUT ($(du -h "$OUTPUT" | cut -f1))"
    else
        error "Failed to create AppImage"
        return 1
    fi
}

# -----------------------------------------------------------------------
# Windows .exe
# -----------------------------------------------------------------------
build_exe() {
    bold "=== Building Windows .exe ==="

    local target="x86_64-pc-windows-gnu"
    local target_dir="$ROOT/target/$target/release"
    local binary_path="$target_dir/kerosene.exe"
    local output="$ROOT/target/Kerosene-${VERSION}-x86_64-windows.exe"

    if ! command -v rustup &>/dev/null; then
        error "rustup is required to add the Windows target"
        return 1
    fi

    check_free_space "$ROOT" 4096

    if ! rustup target list --installed | grep -q "^${target}$"; then
        info "Installing Rust target ${target}..."
        rustup target add "$target"
    fi

    if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
        error "Missing linker: x86_64-w64-mingw32-gcc"
        error "Install mingw-w64 (example: 'sudo apt install mingw-w64')"
        return 1
    fi

    info "Building Windows release binary..."
    cargo build --release --target "$target" --manifest-path "$ROOT/Cargo.toml"

    if [ ! -f "$binary_path" ]; then
        error "Failed to build Windows binary at $binary_path"
        return 1
    fi

    cp "$binary_path" "$output"
    info "Done: $output ($(du -h "$output" | cut -f1))"
}

# -----------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------
case "${1:---help}" in
    -h|--help) usage ;;
    deb)       build_deb ;;
    appimage)  build_appimage ;;
    exe)       build_exe ;;
    all)       build_deb; build_appimage; build_exe ;;
    *)
        error "Unknown command: $1"
        usage
        ;;
esac
