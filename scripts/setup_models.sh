#!/usr/bin/env bash
set -euo pipefail

MODELS_DIR="${1:-$HOME/.codexu/models}"
mkdir -p "$MODELS_DIR"

cat <<MSG
[Codexu] Diretório de modelos preparado:
  $MODELS_DIR

Próximos passos:
1) Baixe um modelo Code Llama 7B GGUF quantizado (Q4_K_M ou Q5_K_M) de uma fonte confiável.
2) Coloque o arquivo .gguf nesse diretório.
3) Aponte esse caminho no app (integração completa será habilitada em M4).
MSG
