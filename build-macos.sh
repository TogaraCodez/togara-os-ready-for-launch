#!/usr/bin/env bash
set -euo pipefail

TOOLCHAIN="nightly-2026-07-10"
TARGET="x86_64-unknown-none"

rustup toolchain install "$TOOLCHAIN"
rustup component add llvm-tools-preview --toolchain "$TOOLCHAIN"
rustup target add "$TARGET" --toolchain "$TOOLCHAIN"

cargo +"$TOOLCHAIN" build -p kernel --target "$TARGET"
