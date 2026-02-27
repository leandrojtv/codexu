# Codexu

App desktop (Tauri) para fluxo de coding assistido com foco local-first.

## Backends de LLM suportados

A partir desta versão existem **somente 2 modos**:

1. **Docker provider** (endpoint HTTP local, ex.: Ollama/llama-code)
2. **Cloud provider** (endpoint externo OpenAI-compatible)

> Suporte a modelo local via GGUF/`llama-cli` foi removido do app.

## Rodar desktop

```bash
cargo run -p codexu_desktop
```

## Rodar modo web/dev (fallback UI)

Abra `apps/desktop/ui/index.html` em servidor estático para testar layout.

## Docker provider (CodeLlama)

Arquivos prontos em `deployment/llm-docker`.

```bash
cd deployment/llm-docker
docker compose up -d
docker compose run --rm ollama-init
```

Configuração no app (Settings):
- Provider: `docker`
- Docker endpoint: `http://127.0.0.1:11434`
- Model: `codellama:7b-instruct`

## Cloud provider

Configuração no app (Settings):
- Provider: `cloud`
- Cloud endpoint: ex. `https://api.openai.com`
- API Key (desktop: keychain/credential vault via keyring; web/dev: apenas em memória do frontend)
- Model: string do modelo cloud

## Funcionalidades de UI

- Layout 3 colunas responsivo
- Chat com markdown e botões para copiar bloco de código
- Plan/Steps + Preview
- Diff viewer com Apply/Reject
- Log/Console com filtro e cópia
- Settings completos de provider/modelo/parâmetros
- Workspace selector com fallback em modo web
