# Codexu (macOS Apple Silicon)

Aplicativo desktop local-first inspirado em agentes de engenharia (Codex/Claude Code), usando Tauri + Rust.

## Status
- ✅ M1 concluído: scaffolding, docs e UI mínima.
- 🚧 Próximo: M2 (tools + guardrails completos).

## Requisitos
- macOS arm64 (Apple Silicon)
- Rust estável + Cargo
- Tauri prerequisites para macOS

## Estrutura
- `crates/core_agent`: planner/executor/validator/reporter
- `crates/tools`: file/search/git/shell tools
- `crates/indexer`: indexador (FTS em M3)
- `crates/llm_provider`: trait de provider + stubs local/remoto
- `apps/desktop`: app Tauri

## Rodando local
```bash
cargo test --workspace
cargo run -p codexu_desktop
```

## Modelo GGUF
Os pesos **não** ficam no repositório. Use:

```bash
bash scripts/setup_models.sh
```

Depois configure o caminho de um arquivo `.gguf` na UI (será integrado nos próximos milestones).

## Guardrails
Veja `SECURITY.md` para decisões e requisitos de sandbox/diff-first/allowlist.
