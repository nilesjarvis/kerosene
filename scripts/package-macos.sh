#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="Kerosene"
BINARY_NAME="kerosene"
BUNDLE_ID="${BUNDLE_ID:-com.kerosene.tradingterminal}"
MIN_MACOS="${MACOSX_DEPLOYMENT_TARGET:-11.0}"
VERSION=$(awk -F '"' '/^version =/ { print $2; exit }' "$ROOT/Cargo.toml")
ARCH=$(uname -m)

BUILD_DIR="$ROOT/target/release"
PACKAGE_DIR="$ROOT/target/macos"
APP_BUNDLE="$PACKAGE_DIR/$APP_NAME.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
STAGING_DIR="$PACKAGE_DIR/dmg-root"
DMG_PATH="$ROOT/target/$APP_NAME-$VERSION-macos-$ARCH.dmg"
PLIST_TEMPLATE="$ROOT/packaging/macos/Info.plist.in"
ICON_SOURCE="$ROOT/assets/kerosene.png"

RED='\033[0;31m'
GREEN='\033[0;32m'
BOLD='\033[1m'
RESET='\033[0m'

info() { echo -e "${GREEN}[+]${RESET} $*"; }
error() { echo -e "${RED}[!]${RESET} $*" >&2; }
bold() { echo -e "${BOLD}$*${RESET}"; }

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        error "Missing required command: $1"
        exit 1
    fi
}

create_icon() {
    info "Generating app icon..."
    sips -s format icns "$ICON_SOURCE" --out "$RESOURCES_DIR/$APP_NAME.icns" >/dev/null
}

write_plist() {
    info "Writing Info.plist..."
    sed \
        -e "s|__BUNDLE_ID__|$BUNDLE_ID|g" \
        -e "s|__VERSION__|$VERSION|g" \
        -e "s|__MIN_MACOS__|$MIN_MACOS|g" \
        "$PLIST_TEMPLATE" > "$CONTENTS_DIR/Info.plist"
    plutil -lint "$CONTENTS_DIR/Info.plist" >/dev/null
}

assemble_app() {
    info "Assembling $APP_NAME.app..."
    rm -rf "$APP_BUNDLE"
    mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

    install -m 755 "$BUILD_DIR/$BINARY_NAME" "$MACOS_DIR/$BINARY_NAME"
    write_plist
    create_icon
}

sign_app() {
    info "Applying ad-hoc code signature..."
    codesign --force --deep --sign - "$APP_BUNDLE"
    codesign --verify --deep --strict "$APP_BUNDLE"
}

create_dmg() {
    info "Creating DMG..."
    rm -rf "$STAGING_DIR"
    mkdir -p "$STAGING_DIR"
    ditto "$APP_BUNDLE" "$STAGING_DIR/$APP_NAME.app"
    ln -s /Applications "$STAGING_DIR/Applications"

    hdiutil create \
        -volname "$APP_NAME" \
        -srcfolder "$STAGING_DIR" \
        -ov \
        -format UDZO \
        "$DMG_PATH" >/dev/null
}

main() {
    if [ "$(uname -s)" != "Darwin" ]; then
        error "macOS packaging must be run on macOS."
        exit 1
    fi

    require_command awk
    require_command cargo
    require_command codesign
    require_command ditto
    require_command hdiutil
    require_command install
    require_command plutil
    require_command sed
    require_command sips

    bold "=== Building $APP_NAME for macOS ==="
    info "Building release binary..."
    MACOSX_DEPLOYMENT_TARGET="$MIN_MACOS" cargo build --release --manifest-path "$ROOT/Cargo.toml"

    mkdir -p "$PACKAGE_DIR"
    assemble_app
    sign_app
    create_dmg

    info "Done: $DMG_PATH ($(du -h "$DMG_PATH" | cut -f1))"
    info "Note: this DMG is ad-hoc signed, but not Developer ID signed or notarized."
}

main "$@"
