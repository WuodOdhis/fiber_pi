#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_TOOLCHAIN="${RAILWAY_RUST_TOOLCHAIN:-1.95.0}"

install_rust=0
if ! command -v cargo >/dev/null 2>&1; then
  install_rust=1
else
  cargo_minor="$(cargo --version | awk '{print $2}' | cut -d. -f2)"
  if [ "${cargo_minor:-0}" -lt 92 ]; then
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

if [ -z "${LIBCLANG_PATH:-}" ]; then
  libclang_path="$(find /nix/store -path '*/lib/libclang.so*' -print -quit 2>/dev/null || true)"
  if [ -n "$libclang_path" ]; then
    export LIBCLANG_PATH="$(dirname "$libclang_path")"
  fi
fi

if [ -n "${LIBCLANG_PATH:-}" ]; then
  echo "[railway-build] LIBCLANG_PATH=$LIBCLANG_PATH"
else
  echo "[railway-build] warning: LIBCLANG_PATH not found before Fiber build"
fi

openssl_pc="$(find /nix/store -path '*/lib/pkgconfig/openssl.pc' -print -quit 2>/dev/null || true)"
if [ -n "$openssl_pc" ]; then
  openssl_pkgconfig_dir="$(dirname "$openssl_pc")"
  export PKG_CONFIG_PATH="$openssl_pkgconfig_dir${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
  echo "[railway-build] PKG_CONFIG_PATH=$PKG_CONFIG_PATH"
else
  echo "[railway-build] warning: openssl.pc not found in /nix/store before lspd build"
fi

stdbool_header="$(find /nix/store -path '*/include/stdbool.h' -print -quit 2>/dev/null || true)"
if [ -n "$stdbool_header" ]; then
  clang_builtin_includes="$(dirname "$stdbool_header")"
  export BINDGEN_EXTRA_CLANG_ARGS="-I$clang_builtin_includes ${BINDGEN_EXTRA_CLANG_ARGS:-}"
  echo "[railway-build] BINDGEN_EXTRA_CLANG_ARGS=$BINDGEN_EXTRA_CLANG_ARGS"
else
  echo "[railway-build] warning: stdbool.h not found in /nix/store before Fiber build"
fi

"$ROOT_DIR/scripts/prepare-fiber.sh"
cargo build --release -p lspd

cd "$ROOT_DIR/demo-ui"
npm install
