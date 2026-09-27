#!/usr/bin/env bash
#
# release-local.sh — Fully-local Kerosene release runner (no GitHub Actions).
#
# Builds and publishes the Linux release (.deb / .rpm / .AppImage) on THIS
# machine and pushes the version tag + GitHub Release via git/gh. Zero GitHub
# Actions minutes.
#
# macOS and Windows packages are NOT produced here (they need macOS/Windows
# hosts); if you later want them on GitHub, add the minimal GH Actions job.
#
# Daily usage: just run this script. It decides whether a release is warranted
# based on commits since the last release tag (see scripts/release-scope.sh):
#   - If there are no new commits, it prints a notice and exits 1 (no-op).
#   - If there are, it bumps Cargo.toml to the next semver (conventional-commit
#     scoping), builds the Linux packages, pushes the version tag, and creates
#     the GitHub Release.
#
# Safety: this pushes to origin and creates a GitHub Release — call it only when
# you want a release. It does NOT require GitHub Actions; just `git` + `gh`.
#
# Environment:
#   RELEASE_FORCE=1   force a release even with no new commits (backfill/retry)
#   GH_TAG_DATE=      (unused; reserved)
#
# Prereqs on this box:
#   - gh authenticated (`gh auth status`)
#   - rpmbuild   for .rpm   (optional: `sudo apt install rpm`)
#   - cargo      for the build
#   - network    to fetch the pinned Pi runtime and appimagetool

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# ---------------------------------------------------------------------------
# System build deps. On this container there is no working sudo (setuid is
# blocked), so the ALSA dev/runtime libs are provided from a user-space prefix
# extracted at ~/.local/kerosene-deps. If that dir exists, wire pkg-config and
# the linker/rpath to it. On a normal host with apt-installed libasound2-dev
# these are already present and this block is a harmless no-op.
# ---------------------------------------------------------------------------
if [ -d "$HOME/.local/kerosene-deps" ]; then
    export PKG_CONFIG_PATH="$HOME/.local/kerosene-deps/usr/lib/x86_64-linux-gnu/pkgconfig:${PKG_CONFIG_PATH:-}"
    export LIBRARY_PATH="$HOME/.local/kerosene-deps/usr/lib/x86_64-linux-gnu:${LIBRARY_PATH:-}"
    export LD_LIBRARY_PATH="$HOME/.local/kerosene-deps/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}"
    export PATH="$HOME/.local/kerosene-deps/usr/bin:${PATH:-}"
    # Point rpmbuild at the container-extracted rpm config (no /usr/lib/rpm on
    # this box). RPM_CONFIGDIR is honored by rpm to locate rpmrc+macros.
    if [ -f "$HOME/.local/kerosene-deps/usr/lib/rpm/rpmrc" ]; then
        export RPM_CONFIGDIR="$HOME/.local/kerosene-deps/usr/lib/rpm"
    fi
    echo "(using user-space build deps from ~/.local/kerosene-deps)"
fi

# ---------------------------------------------------------------------------
# Decide whether to release. `release-scope.sh` prints KEY=VALUE lines.
# ---------------------------------------------------------------------------
echo "=== Kerosene local release check ==="
if ! ./scripts/release-scope.sh > /tmp/kerosene-release-scope.txt 2>&1; then
    echo "Nothing to release (no new commits since last tag)."
    cat /tmp/kerosene-release-scope.txt || true
    exit 0
fi

# Load the KEY=VALUE lines into the environment.
set -a
# release-scope.sh output uses single-value lines (no spaces) so `eval` is safe.
eval "$(grep -E '^(HAS_RELEASES|LAST_TAG|RANGE|NEXT_VERSION|NEXT_TAG|VERSION_EXISTS|BUMP)=' /tmp/kerosene-release-scope.txt)"
set +a

TAG="${NEXT_TAG:-v${NEXT_VERSION}}"
echo "Releasing ${NEXT_VERSION} as ${TAG} (bump=${BUMP}, range=${RANGE:-none}, new_commits=${NEW_COUNT:-?})"

# The release notes body: strip the RELEASE_NOTES= prefix AND decode the \n we
# embedded (release-scope.sh emits the notes as a single line with literal \n).
RELEASE_NOTES="$(sed -n 's/^RELEASE_NOTES=//p' /tmp/kerosene-release-scope.txt | sed 's/\\n/\n/g')"
printf '%s\n' "${RELEASE_NOTES}" > /tmp/kerosene-release-notes.md

# ---------------------------------------------------------------------------
# 1. Bump Cargo.toml to the next version (if a tag doesn't already exist) and
#    create the version tag.
# ---------------------------------------------------------------------------
if [ "${VERSION_EXISTS:-0}" != "1" ]; then
    echo "=== Ensuring Cargo.toml is at ${NEXT_VERSION} ==="
    NEW_LINE="version = \"${NEXT_VERSION}\""
    sed -i "s/^version = \"[^\"]*\"/${NEW_LINE//\//\\/}/" Cargo.toml

    # Only commit the bump if it actually changed the file (a prior manual
    # bump to this version leaves the file unchanged and is fine to skip).
    if ! git diff --quiet -- Cargo.toml; then
        git add Cargo.toml
        git commit -m "chore(release): ${TAG}"
    else
        echo "Cargo.toml already at ${NEXT_VERSION}; committing nothing."
    fi

    # Never fail if the tag already exists.
    if ! git rev-parse --verify --quiet "refs/tags/${TAG}" >/dev/null 2>&1; then
        git tag -a "${TAG}" -m "Kerosene ${TAG}"
    else
        echo "Tag ${TAG} already exists; reusing it."
    fi
fi

# ---------------------------------------------------------------------------
# 2. Build the Linux packages. `package.sh all` produces .deb/.rpm/.AppImage.
#    .rpm is skipped gracefully if rpmbuild is missing; .deb/.AppImage are the
#    essential Linux artifacts.
# ---------------------------------------------------------------------------
echo "=== Building Linux packages ==="
./scripts/package.sh all

# ---------------------------------------------------------------------------
# 3. Build the artifact list — only files that actually exist AND match the
#    current version, so stale artifacts from a prior release aren't attached.
# ---------------------------------------------------------------------------
ARTIFACTS=()
for pat in \
    "$ROOT"/target/debian/*"${NEXT_VERSION}"*.deb \
    "$ROOT"/target/rpm/*"${NEXT_VERSION}"*.rpm \
    "$ROOT"/target/Kerosene-"${NEXT_VERSION}"*.AppImage ; do
    if [ -f "$pat" ]; then
        ARTIFACTS+=("$pat")
    fi
done

if [ "${#ARTIFACTS[@]}" -eq 0 ]; then
    echo "::error::No Linux artifacts were produced."
    exit 1
fi
echo "Artifacts:"
for a in "${ARTIFACTS[@]}"; do echo "  - $a"; done

# ---------------------------------------------------------------------------
# 4. Push the tag (so the release references a real tag on origin).
# ---------------------------------------------------------------------------
echo "=== Pushing tag ${TAG} ==="
git push origin "${TAG}"
# Also push the version-bump commit to main so the source bump is recorded.
# (Tolerates "up to date" / explicit head cases with a plain push.)
if [ "${VERSION_EXISTS:-0}" != "1" ]; then
    git push origin main || echo "(main push skipped/up-to-date — tag was pushed)"
fi

# ---------------------------------------------------------------------------
# 5. Create / update the GitHub Release with the Linux artifacts.
# ---------------------------------------------------------------------------
echo "=== Publishing GitHub Release ${TAG} ==="
if gh release view "${TAG}" >/dev/null 2>&1; then
    echo "Release ${TAG} already exists; uploading/replacing artifacts."
    gh release upload "${TAG}" "${ARTIFACTS[@]}" --clobber
else
    gh release create "${TAG}" \
        --title "${TAG}" \
        --notes-file /tmp/kerosene-release-notes.md \
        --latest \
        "${ARTIFACTS[@]}"
fi

echo "=== Done: ${TAG} published (Linux) ==="
gh release view "${TAG}" --json tagName,isDraft,name,assets | head -30
