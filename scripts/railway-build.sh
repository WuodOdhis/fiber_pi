#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_TOOLCHAIN="${RAILWAY_RUST_TOOLCHAIN:-1.85.0}"

install_rust=0
if ! command -v cargo >/dev/null 2>&1; then
  install_rust=1
else
  cargo_minor="$(cargo --version | awk '{print $2}' | cut -d. -f2)"
  if [ "${cargo_minor:-0}" -lt 85 ]; then
    install_rust=1
  fi
fi

if [ "$install_rust" = "1" ]; then
  echo "[railway-build] installing Rust $RUST_TOOLCHAIN"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain "$RUST_TOOLCHAIN"
fi

# shellcheck source=/dev/null
. "$HOME/.cargo/env"

echo "[railway-build] rustc: $(rustc --version)"
echo "[railway-build] cargo: $(cargo --version)"

"$ROOT_DIR/scripts/prepare-fiber.sh"
cargo build --release -p lspd

cd "$ROOT_DIR/demo-ui"
npm install
