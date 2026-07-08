#!/usr/bin/env bash
set -euo pipefail

echo "[check] project root: $(pwd)"

for cmd in git cargo rustc; do
  if command -v "$cmd" >/dev/null 2>&1; then
    echo "[ok] $cmd: $($cmd --version | head -n 1)"
  else
    echo "[missing] $cmd"
    exit 1
  fi
done

if command -v docker >/dev/null 2>&1; then
  echo "[ok] docker: $(docker --version)"
else
  echo "[warn] docker not found; local source build will be used"
fi

if command -v offckb >/dev/null 2>&1; then
  echo "[ok] offckb: $(offckb --version)"
else
  echo "[warn] offckb not found"
fi

if command -v ckb-cli >/dev/null 2>&1; then
  echo "[ok] ckb-cli: $(ckb-cli --version | head -n 1)"
else
  echo "[warn] ckb-cli not found; key export/funding flow still needs verification"
fi
