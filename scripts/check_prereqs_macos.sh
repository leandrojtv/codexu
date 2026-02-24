#!/usr/bin/env bash
set -euo pipefail

missing=0

check_cmd () {
  local cmd="$1"
  local hint="$2"
  if command -v "$cmd" >/dev/null 2>&1; then
    echo "[ok] $cmd -> $(command -v "$cmd")"
  else
    echo "[missing] $cmd ($hint)"
    missing=1
  fi
}

check_cmd xcode-select "instale Xcode Command Line Tools"
check_cmd cargo "instale Rust (rustup)"
check_cmd cmake "brew install cmake"
check_cmd git "instale Command Line Tools"

echo
if [[ "$missing" -eq 1 ]]; then
  echo "Pré-requisitos faltando. Corrija os itens [missing] e rode novamente."
  exit 1
fi

echo "Pré-requisitos principais OK para build local do Codexu + llama.cpp."
