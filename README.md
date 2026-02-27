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

## UI/UX Overhaul (Codex-style)

- App shell clean com topbar discreta, sidebar leve e painel de contexto em abas.
- Composer grande fixo no rodapé (/, @, anexar, mic, stop, regenerate).
- Empty state com cards de sugestões rápidas.
- Chat com markdown, code blocks premium (copy/insert/save/review).
- Diff panel com lista de arquivos e ações Apply/Reject por arquivo/all.
- Logs com timeline, filtros por nível e busca.

### Screenshot (after)

Veja a captura no artefato: `artifacts/ui-redesign.png`.


### Cloud endpoint (OpenAI / Azure)

- OpenAI base URL: `https://api.openai.com` (o app adiciona `/v1/chat/completions`).
- Azure endpoint completo também é suportado, por exemplo: `https://SEU-RECURSO.openai.azure.com/openai/responses?api-version=2025-04-01-preview`.
- Em Azure, o app tenta `Authorization: Bearer` primeiro e faz fallback para `api-key` quando necessário.

- Se o modelo cloud rejeitar parâmetros (ex.: `temperature`), o app tenta automaticamente reenviar sem os parâmetros não suportados.

- Você pode forçar o modo de autenticação via `CODEXU_CLOUD_AUTH_MODE=bearer` ou `CODEXU_CLOUD_AUTH_MODE=api-key`.
- Para Responses API, o app muda automaticamente entre `messages` e `input` conforme o endpoint/modelo exigir.
- Em Responses API, o app também alterna automaticamente entre `max_completion_tokens` e `max_output_tokens` conforme compatibilidade do endpoint/modelo.
