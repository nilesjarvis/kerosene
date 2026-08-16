#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION_FILE="$ROOT/packaging/pi/version.txt"
CHECKSUM_FILE="$ROOT/packaging/pi/SHA256SUMS"
VERSION="$(tr -d '[:space:]' < "$VERSION_FILE")"

info() {
    echo "[+] $*" >&2
}

error() {
    echo "[!] $*" >&2
}

detect_platform() {
    local os
    local arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux) os="linux" ;;
        Darwin) os="darwin" ;;
        *)
            error "Unsupported Pi host operating system: $os"
            return 1
            ;;
    esac

    case "$arch" in
        x86_64|amd64) arch="x64" ;;
        arm64|aarch64) arch="arm64" ;;
        *)
            error "Unsupported Pi host architecture: $arch"
            return 1
            ;;
    esac

    echo "${os}-${arch}"
}

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        error "A SHA-256 tool (sha256sum or shasum) is required"
        return 1
    fi
}

download_file() {
    local url="$1"
    local output="$2"
    if command -v curl >/dev/null 2>&1; then
        curl --fail --location --show-error --silent "$url" --output "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget --quiet "$url" --output-document "$output"
    else
        error "curl or wget is required to download the pinned Pi binary"
        return 1
    fi
}

validate_binary() {
    local binary="$1"
    local reported
    if [ ! -x "$binary" ]; then
        return 1
    fi
    reported="$($binary --version 2>/dev/null | head -1 | tr -d '\r')"
    case "$reported" in
        *"$VERSION"*) return 0 ;;
        *) return 1 ;;
    esac
}

validate_bundle() {
    local bundle="$1"
    validate_binary "$bundle/pi" \
        && [ -f "$bundle/package.json" ] \
        && [ -f "$bundle/theme/dark.json" ] \
        && [ -f "$bundle/theme/light.json" ]
}

smoke_test_rpc() {
    local bundle="$1"
    local smoke_dir
    local response
    smoke_dir="$(mktemp -d "$ROOT/target/pi/.smoke.XXXXXX")"
    if ! response="$({
        cd "$smoke_dir"
        env \
            OPENROUTER_API_KEY=rpc-smoke-test \
            KEROSENE_AGENT_HYPERDASH_API_KEY= \
            KEROSENE_AGENT_SNAPSHOT="$smoke_dir/snapshot.json" \
            PI_CODING_AGENT_DIR="$smoke_dir/config" \
            PI_SKIP_VERSION_CHECK=1 \
            PI_TELEMETRY=0 \
            "$bundle/pi" \
            --mode rpc \
            --no-session \
            --provider openrouter \
            --model openai/gpt-4.1 \
            --tools kerosene_data \
            --extension "$ROOT/assets/agent/kerosene.ts" <<'EOF'
{"type":"get_state"}
EOF
    } 2>&1)"; then
        rm -rf "$smoke_dir"
        error "Pi failed the offline RPC startup smoke test"
        return 1
    fi
    rm -rf "$smoke_dir"
    if [[ "$response" != *'"command":"get_state","success":true'* ]]; then
        error "Pi did not return a successful get_state RPC response"
        return 1
    fi
}

PLATFORM="${1:-$(detect_platform)}"
case "$PLATFORM" in
    linux-x64|linux-arm64|darwin-x64|darwin-arm64) ;;
    *)
        error "Unsupported Pi packaging platform: $PLATFORM"
        exit 1
        ;;
esac

ARCHIVE_NAME="pi-${PLATFORM}.tar.gz"
EXPECTED_SHA="$(awk -v archive="$ARCHIVE_NAME" '$2 == archive { print $1 }' "$CHECKSUM_FILE")"
if [ -z "$EXPECTED_SHA" ]; then
    error "No pinned checksum exists for $ARCHIVE_NAME"
    exit 1
fi

CACHE_DIR="$ROOT/target/pi/$VERSION/$PLATFORM"
BUNDLE_PATH="$CACHE_DIR/bundle"
if validate_bundle "$BUNDLE_PATH" && smoke_test_rpc "$BUNDLE_PATH"; then
    echo "$BUNDLE_PATH"
    exit 0
fi

DOWNLOAD_DIR="$ROOT/target/pi/downloads/$VERSION"
ARCHIVE_PATH="$DOWNLOAD_DIR/$ARCHIVE_NAME"
mkdir -p "$DOWNLOAD_DIR" "$CACHE_DIR"

if [ -f "$ARCHIVE_PATH" ]; then
    ACTUAL_SHA="$(sha256_file "$ARCHIVE_PATH")"
    if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
        error "Discarding cached $ARCHIVE_NAME with an invalid checksum"
        rm -f "$ARCHIVE_PATH"
    fi
fi

if [ ! -f "$ARCHIVE_PATH" ]; then
    info "Downloading Pi $VERSION for $PLATFORM"
    TEMP_ARCHIVE="$ARCHIVE_PATH.download"
    rm -f "$TEMP_ARCHIVE"
    download_file \
        "https://github.com/earendil-works/pi/releases/download/v${VERSION}/${ARCHIVE_NAME}" \
        "$TEMP_ARCHIVE"
    ACTUAL_SHA="$(sha256_file "$TEMP_ARCHIVE")"
    if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
        rm -f "$TEMP_ARCHIVE"
        error "Checksum verification failed for $ARCHIVE_NAME"
        exit 1
    fi
    mv "$TEMP_ARCHIVE" "$ARCHIVE_PATH"
fi

TEMP_DIR="$(mktemp -d "$ROOT/target/pi/.extract.XXXXXX")"
cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

tar -xzf "$ARCHIVE_PATH" -C "$TEMP_DIR"
SOURCE_BINARY="$(find "$TEMP_DIR" -type f -name pi -print | sed -n '1p')"
if [ -z "$SOURCE_BINARY" ]; then
    error "$ARCHIVE_NAME does not contain a Pi executable"
    exit 1
fi

SOURCE_DIR="$(dirname "$SOURCE_BINARY")"
STAGED_BUNDLE="$TEMP_DIR/runtime-bundle"
mkdir -p "$STAGED_BUNDLE/theme"
install -m 755 "$SOURCE_BINARY" "$STAGED_BUNDLE/pi"
install -m 644 "$SOURCE_DIR/package.json" "$STAGED_BUNDLE/package.json"
install -m 644 "$SOURCE_DIR/theme/dark.json" "$STAGED_BUNDLE/theme/dark.json"
install -m 644 "$SOURCE_DIR/theme/light.json" "$STAGED_BUNDLE/theme/light.json"
install -m 644 "$SOURCE_DIR/theme/theme-schema.json" "$STAGED_BUNDLE/theme/theme-schema.json"

rm -rf "$BUNDLE_PATH"
mv "$STAGED_BUNDLE" "$BUNDLE_PATH"
if ! validate_bundle "$BUNDLE_PATH"; then
    rm -rf "$BUNDLE_PATH"
    error "The extracted Pi bundle did not report the pinned version $VERSION"
    exit 1
fi
if ! smoke_test_rpc "$BUNDLE_PATH"; then
    rm -rf "$BUNDLE_PATH"
    exit 1
fi

echo "$BUNDLE_PATH"
