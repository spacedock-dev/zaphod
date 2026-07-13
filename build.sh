#!/bin/bash
# ABOUTME: Builds the zellij-sidebar wasm plugin with rustup's rustc pinned.
# ABOUTME: Needed because homebrew rust shadows rustup on PATH and lacks wasm32-wasip1 std.
set -euo pipefail
cd "$(dirname "$0")"
RUSTC="$(rustup which rustc)" cargo build --release --target wasm32-wasip1 "$@"
mkdir -p target
(
    cd grout
    go build -o ../target/zaphod .
)
ls -la target/wasm32-wasip1/release/zellij-sidebar.wasm
ls -la target/zaphod
