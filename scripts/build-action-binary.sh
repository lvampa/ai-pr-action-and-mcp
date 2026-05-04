#!/usr/bin/env bash
# Build the composite-action Linux binary and install it under bin/.
# Intended for CI (Ubuntu + musl-tools) and maintainers mirroring that environment.
#
# Usage (from repo root):
#   rustup target add x86_64-unknown-linux-musl
#   sudo apt-get install -y musl-tools   # Debian/Ubuntu
#   ./scripts/build-action-binary.sh
#
# Override target triple:
#   TARGET=x86_64-unknown-linux-gnu ./scripts/build-action-binary.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET="${TARGET:-x86_64-unknown-linux-musl}"
CRATE="ai-pr-action"
DEST="bin/${CRATE}"

echo "Building ${CRATE} (release, ${TARGET})…"
cargo build -p "${CRATE}" --release --target "${TARGET}"

mkdir -p bin
cp "target/${TARGET}/release/${CRATE}" "${DEST}"
strip "${DEST}"
chmod +x "${DEST}"

echo "Installed ${DEST} ($(wc -c < "${DEST}") bytes)"
