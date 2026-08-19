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
Usage: $(basename "$0") [deb|rpm|appimage|macos|all]

Build distributable packages for Kerosene.

Commands:
  deb        Build a .deb package (requires cargo-deb)
  rpm        Build a .rpm package (requires rpmbuild)
  appimage   Build an .AppImage (requires appimagetool)
  macos      Build a macOS .dmg (requires macOS built-in tooling)
  all        Build .deb, .rpm, and .AppImage (default)

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
    info "Building release binary..."
    cargo build --release --manifest-path "$ROOT/Cargo.toml"
}

stage_pi_bundle() {
    info "Staging pinned Pi runtime..."
    local source_bundle
    local staged_bundle="$ROOT/target/package-assets/pi"
    source_bundle="$(bash "$ROOT/scripts/fetch-pi.sh")"

    rm -rf "$staged_bundle"
    mkdir -p "$staged_bundle/theme"
    install -m 755 "$source_bundle/pi" "$staged_bundle/pi"
    install -m 644 "$source_bundle/package.json" "$staged_bundle/package.json"
    install -m 644 "$source_bundle/theme/dark.json" "$staged_bundle/theme/dark.json"
    install -m 644 "$source_bundle/theme/light.json" "$staged_bundle/theme/light.json"
    install -m 644 "$source_bundle/theme/theme-schema.json" "$staged_bundle/theme/theme-schema.json"
}

assert_deb_contains_pi() {
    local package="$1"
    if ! dpkg-deb --contents "$package" | grep 'usr/lib/kerosene/pi/pi$' >/dev/null; then
        error "The .deb package does not contain the pinned Pi runtime"
        return 1
    fi
}

assert_rpm_contains_pi() {
    local package="$1"
    if ! rpm -qlp "$package" | grep '^/usr/lib/kerosene/pi/pi$' >/dev/null; then
        error "The .rpm package does not contain the pinned Pi runtime"
        return 1
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
    stage_pi_bundle

    info "Packaging .deb..."
    cargo deb --no-build --manifest-path "$ROOT/Cargo.toml"

    DEB=$(ls -t "$ROOT/target/debian/"*.deb 2>/dev/null | head -1)
    if [ -n "$DEB" ]; then
        assert_deb_contains_pi "$DEB"
        info "Done: $DEB ($(du -h "$DEB" | cut -f1))"
    else
        error "Failed to find .deb output"
        return 1
    fi
}

# -----------------------------------------------------------------------
# .rpm
# -----------------------------------------------------------------------
build_rpm() {
    bold "=== Building .rpm package ==="

    if ! command -v rpmbuild &>/dev/null; then
        error "rpmbuild not found."
        error "Install RPM build tools first, then rerun this command."
        error "Fedora/RHEL: sudo dnf install rpm-build"
        error "openSUSE:    sudo zypper install rpm-build"
        error "Debian/Ubuntu: sudo apt install rpm"
        return 1
    fi

    ensure_release_binary
    stage_pi_bundle

    RPM_TOPDIR="$ROOT/target/rpmbuild"
    RPM_OUTDIR="$ROOT/target/rpm"
    RPM_SPEC="$RPM_TOPDIR/SPECS/kerosene.spec"

    info "Assembling RPM build tree..."
    rm -rf "$RPM_TOPDIR/BUILD" "$RPM_TOPDIR/BUILDROOT" "$RPM_TOPDIR/RPMS" "$RPM_TOPDIR/SRPMS"
    mkdir -p \
        "$RPM_TOPDIR/BUILD" \
        "$RPM_TOPDIR/BUILDROOT" \
        "$RPM_TOPDIR/RPMS" \
        "$RPM_TOPDIR/SOURCES" \
        "$RPM_TOPDIR/SPECS" \
        "$RPM_TOPDIR/SRPMS" \
        "$RPM_OUTDIR"

    cat > "$RPM_SPEC" <<EOF
Name:           kerosene
Version:        $VERSION
Release:        1%{?dist}
Summary:        Hyperliquid Trading Terminal
License:        MIT

%global debug_package %{nil}

%description
Kerosene is a desktop trading terminal for Hyperliquid.

%prep

%build

%install
rm -rf "%{buildroot}"
install -Dm0755 "$ROOT/target/release/kerosene" "%{buildroot}/usr/bin/kerosene"
install -Dm0755 "$ROOT/target/package-assets/pi/pi" "%{buildroot}/usr/lib/kerosene/pi/pi"
install -Dm0644 "$ROOT/target/package-assets/pi/package.json" "%{buildroot}/usr/lib/kerosene/pi/package.json"
install -Dm0644 "$ROOT/target/package-assets/pi/theme/dark.json" "%{buildroot}/usr/lib/kerosene/pi/theme/dark.json"
install -Dm0644 "$ROOT/target/package-assets/pi/theme/light.json" "%{buildroot}/usr/lib/kerosene/pi/theme/light.json"
install -Dm0644 "$ROOT/target/package-assets/pi/theme/theme-schema.json" "%{buildroot}/usr/lib/kerosene/pi/theme/theme-schema.json"
install -Dm0644 "$ROOT/assets/kerosene.desktop" "%{buildroot}/usr/share/applications/kerosene.desktop"
install -Dm0644 "$ROOT/assets/kerosene.png" "%{buildroot}/usr/share/icons/hicolor/256x256/apps/kerosene.png"
install -Dm0644 "$ROOT/assets/kerosene.svg" "%{buildroot}/usr/share/icons/hicolor/scalable/apps/kerosene.svg"
install -Dm0644 "$ROOT/LICENSE" "%{buildroot}/usr/share/licenses/kerosene/LICENSE"
install -Dm0644 "$ROOT/packaging/pi/LICENSE" "%{buildroot}/usr/share/licenses/kerosene/PI-LICENSE"

%files
%license /usr/share/licenses/kerosene/LICENSE
/usr/bin/kerosene
/usr/lib/kerosene/pi/pi
/usr/lib/kerosene/pi/package.json
/usr/lib/kerosene/pi/theme/dark.json
/usr/lib/kerosene/pi/theme/light.json
/usr/lib/kerosene/pi/theme/theme-schema.json
/usr/share/applications/kerosene.desktop
/usr/share/icons/hicolor/256x256/apps/kerosene.png
/usr/share/icons/hicolor/scalable/apps/kerosene.svg
/usr/share/licenses/kerosene/PI-LICENSE
EOF

    info "Packaging .rpm..."
    rpmbuild \
        --define "_topdir $RPM_TOPDIR" \
        --define "_build_id_links none" \
        -bb "$RPM_SPEC"

    RPM=$(find "$RPM_TOPDIR/RPMS" -type f -name "*.rpm" -printf "%T@ %p\n" \
        | sort -nr \
        | head -1 \
        | cut -d' ' -f2-)
    if [ -n "$RPM" ]; then
        cp "$RPM" "$RPM_OUTDIR/"
        RPM="$RPM_OUTDIR/$(basename "$RPM")"
        assert_rpm_contains_pi "$RPM"
        info "Done: $RPM ($(du -h "$RPM" | cut -f1))"
    else
        error "Failed to find .rpm output"
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
    stage_pi_bundle

    # Create AppDir
    APPDIR=$(mktemp -d)
    trap "rm -rf '$APPDIR'" EXIT

    info "Assembling AppDir..."
    mkdir -p "$APPDIR/usr/bin"
    cp "$ROOT/target/release/kerosene" "$APPDIR/usr/bin/"
    mkdir -p "$APPDIR/usr/lib/kerosene"
    cp -R "$ROOT/target/package-assets/pi" "$APPDIR/usr/lib/kerosene/"
    cp "$ROOT/assets/kerosene.desktop"  "$APPDIR/"
    cp "$ROOT/assets/kerosene.png"      "$APPDIR/"

    mkdir -p "$APPDIR/usr/share/licenses/kerosene"
    cp "$ROOT/packaging/pi/LICENSE" "$APPDIR/usr/share/licenses/kerosene/PI-LICENSE"

    if [ ! -x "$APPDIR/usr/lib/kerosene/pi/pi" ]; then
        error "The AppImage staging tree does not contain the pinned Pi runtime"
        return 1
    fi

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
    rpm)       build_rpm ;;
    appimage)  build_appimage ;;
    macos)     "$ROOT/scripts/package-macos.sh" ;;
    all)       build_deb; build_rpm; build_appimage ;;
    *)
        error "Unknown command: $1"
        usage
        ;;
esac
