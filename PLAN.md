# PLAN

## Milestones

- [x] **M1**: Scaffolding do monorepo + app Tauri rodando com UI mínima (chat, plan/steps, diff, log, seleção de workspace).
- [x] **M2**: Tools com guardrails completos (workspace sandbox, diff-first apply, backup branch, shell allowlist + confirmação).
- [x] **M3**: Indexer SQLite FTS5 + ripgrep + index incremental.
- [x] **M4**: Provider local llama.cpp (Metal, streaming) + setup de modelo GGUF.
- [ ] **M5**: Loop do agente (plan -> retrieve -> propose diff -> apply -> validate) com demo.
- [ ] **M6**: Provider remoto opcional (Azure-like) com UI/config.

## Checklist M1
- [x] Workspace Rust com crates `core_agent`, `tools`, `indexer`, `llm_provider`.
- [x] App Tauri em `apps/desktop` com backend Rust e frontend mínimo.
- [x] Documentação inicial (README, IMPLEMENT, SECURITY, PLAN).
- [x] Script `scripts/setup_models.sh`.
- [x] Validações locais básicas (`cargo fmt`, `cargo test`).
- [x] Conexão UI ↔ backend Tauri para seleção real de workspace e chat com resposta mock.


## Checklist M2
- [x] `file_tool` com workspace sandbox, bloqueio de caminhos sensíveis e modo diff-first com aprovação explícita.
- [x] `shell_tool` com allowlist e confirmação obrigatória fora da lista.
- [x] `git_tool` com status/diff e criação de branch de backup `codexu/<timestamp>`.
- [x] Testes unitários básicos para guardrails críticos em `tools`.


## Checklist M3
- [x] `crates/indexer` com SQLite + FTS5 e schema inicial (`files` + `docs_fts`).
- [x] Indexação incremental por hash SHA-256 de conteúdo.
- [x] Busca FTS com snippet e limite de resultados.
- [x] `search_tool` com `ripgrep` real e busca no índice SQLite.


## Checklist M4
- [x] `crates/llm_provider` com `LlmConfig` e validação de GGUF.
- [x] `LocalLlamaCppProvider` executando `llama-cli` com parâmetros (contexto, temp, top_p, max_tokens, gpu layers).
- [x] Streaming MVP via chunks por linha da saída do `llama.cpp`.
- [x] Recomendação de modelo para Mac 18GB (7B Q4_K_M / Q5_K_M).
