#!/usr/bin/env bash
# Bumps the app version to match the release tag being built, so the version
# baked into the binary (what Settings displays, and what the Android
# updater's "is this newer?" check compares against) never drifts from what
# was actually tagged and released. Run from the repo root (legere/),
# passing the tag (e.g. "v0.1.0") as the only argument.
set -euo pipefail

tag="$1"

if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "::warning::'$tag' isn't a vX.Y.Z tag (likely a manual workflow_dispatch run) — leaving the committed version as-is."
  exit 0
fi

version="${tag#v}"

# The Rust/Tauri version lives in the workspace root, not src-tauri/Cargo.toml
# — src-tauri inherits it via `version.workspace = true`.
jq --arg v "$version" '.version = $v' src-tauri/tauri.conf.json > src-tauri/tauri.conf.json.tmp
mv src-tauri/tauri.conf.json.tmp src-tauri/tauri.conf.json

# Only the first "version = ..." line, i.e. [workspace.package]'s — not any
# dependency's pinned "version = ..." further down.
sed -i "0,/^version = \".*\"/s//version = \"$version\"/" Cargo.toml

npm version "$version" --no-git-tag-version --allow-same-version

echo "Synced app version to $version"
