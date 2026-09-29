#!/usr/bin/env bash
# Unit tests + headless acceptance self-test. Usage: scripts/test.sh [screenshot-dir]
set -e
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="$HOME/.cache/elementallegends-target"
cargo test --release
cargo build --release
SDL_AUDIODRIVER=dummy "$CARGO_TARGET_DIR/release/elementallegends" --selftest "$@"
