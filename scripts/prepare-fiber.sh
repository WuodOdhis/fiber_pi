#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIBER_VERSION="v0.8.1"
SRC_DIR="$ROOT_DIR/.fiber-src/fiber-$FIBER_VERSION"
BIN_DIR="$ROOT_DIR/.fiber-bin"

mkdir -p "$ROOT_DIR/.fiber-src" "$BIN_DIR"

if [ ! -d "$SRC_DIR/.git" ]; then
  echo "[prepare] cloning Fiber $FIBER_VERSION"
  git clone --depth 1 --branch "$FIBER_VERSION" https://github.com/nervosnetwork/fiber.git "$SRC_DIR"
else
  echo "[prepare] using existing Fiber checkout at $SRC_DIR"
fi

echo "[prepare] building fnn and fnn-cli"
cargo build --release --locked -p fiber-bin -p fnn-cli --manifest-path "$SRC_DIR/Cargo.toml"

cp "$SRC_DIR/target/release/fnn" "$BIN_DIR/fnn"
cp "$SRC_DIR/target/release/fnn-cli" "$BIN_DIR/fnn-cli"
chmod +x "$BIN_DIR/fnn" "$BIN_DIR/fnn-cli"

echo "[prepare] binaries installed:"
"$BIN_DIR/fnn" --version || true
"$BIN_DIR/fnn-cli" --version || true
