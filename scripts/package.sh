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
Usage: $(basename "$0") [deb|appimage|macos|all]

Build distributable packages for Kerosene.

Commands:
  deb        Build a .deb package (requires cargo-deb)
  appimage   Build an .AppImage (requires appimagetool)
  macos      Build a macOS .dmg (requires macOS built-in tooling)
  all        Build .deb and .AppImage (default)

Options:
  -h, --help   Show this help message

The release binary is built automatically if not already present.
Output files are placed in target/.

Windows release artifacts are built on Windows/MSVC with:
  pwsh ./scripts/package-windows.ps1

macOS release artifacts are built on macOS with:
  ./scripts/package-macos.sh
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
# Main
# -----------------------------------------------------------------------
case "${1:-all}" in
    -h|--help) usage ;;
    deb)       build_deb ;;
    appimage)  build_appimage ;;
    macos)     "$ROOT/scripts/package-macos.sh" ;;
    all)       build_deb; build_appimage ;;
    *)
        error "Unknown command: $1"
        usage
        ;;
esac
