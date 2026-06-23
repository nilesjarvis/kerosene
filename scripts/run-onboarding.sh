#!/usr/bin/env bash
#
# Launch Kerosene in an isolated config sandbox with the first-run onboarding
# welcome screen forced ON, so you can see/test it on every run without touching
# your real ~/.config/kerosene.
#
# Usage:
#   scripts/run-onboarding.sh            # reuse the sandbox (fast), re-show onboarding
#   scripts/run-onboarding.sh --fresh    # wipe the sandbox first (true clean slate)
#   scripts/run-onboarding.sh --release  # any extra args are forwarded to `cargo run`
#
# Override the sandbox location with KEROSENE_TEST_CONFIG=/some/dir.
#
# The sandbox isolates the whole config tree (config.json, caches, fonts, sounds,
# journal) via XDG_CONFIG_HOME, so it never disturbs your normal setup. Clicking
# "Enter Terminal" dismisses onboarding for that run; the next launch flips the
# flag back to false so the screen shows again.

set -euo pipefail

# Run from the repo root regardless of the caller's working directory.
REPO_ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
cd "$REPO_ROOT"

SANDBOX="${KEROSENE_TEST_CONFIG:-$HOME/.kerosene-onboarding-test}"
CFG="$SANDBOX/kerosene/config.json"

if [[ "${1:-}" == "--fresh" ]]; then
  rm -rf "$SANDBOX"
  shift
fi

mkdir -p "$SANDBOX/kerosene"

# Force the onboarding to show. If the sandbox already has a config (because a
# previous run clicked "Enter Terminal" and persisted dismissed=true), flip the
# flag back to false; on a brand-new sandbox the app loads defaults (false) on
# its own. Removing the key would NOT work -- the legacy serde default is true.
if [[ -f "$CFG" ]]; then
  if command -v jq >/dev/null 2>&1; then
    tmp="$(mktemp)"
    jq '.app_onboarding_dismissed = false' "$CFG" > "$tmp" && mv "$tmp" "$CFG"
  else
    # No jq available: drop the config so the app reloads defaults (onboarding on).
    rm -f "$CFG"
  fi
fi

echo "Onboarding sandbox: $SANDBOX  (pass --fresh to wipe it)"
exec env XDG_CONFIG_HOME="$SANDBOX" cargo run "$@"
