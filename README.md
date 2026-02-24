# Codexu (macOS Apple Silicon)

App desktop local-first (Tauri + Rust) para fluxo estilo agente de código.

## Status atual
- ✅ M1: app desktop mínimo + chat/painéis.
- ✅ M2: guardrails de tools (sandbox, diff-first, shell policy, backup branch).
- ✅ M3: indexador SQLite FTS5 + busca com ripgrep.
- 🚧 Próximo: M4 (provider local llama.cpp + streaming).

---

## TL;DR (passo a passo exato para funcionar)

> Se você é novo em desktop dev, siga **nesta ordem**.

1. Clone o repo e entre na pasta:

```bash
git clone <url-do-seu-repo>
cd codexu
```

2. Instale pré-requisitos no macOS:
- Xcode Command Line Tools
- Rust stable (cargo)
- Dependências Tauri para macOS

3. Rode validações:

```bash
cargo fmt --all
cargo test --workspace
```

4. Rode o app desktop real (não navegador):

```bash
cargo run -p codexu_desktop
```

5. Confirme no topo do app:
- **runtime: tauri v...**
- título **Codexu (M3)**

Se aparecer `runtime: browser fallback`, você não está no app desktop real.

---

## Como saber se está no modo certo

### ✅ Modo correto (desktop Tauri)
- Badge no topo: `runtime: tauri v...`
- Seleção de workspace abre diálogo nativo do macOS.

### ⚠️ Modo incorreto (fallback navegador)
- Badge no topo: `runtime: browser fallback`
- Log mostra: `modo navegador detectado: backend Tauri não disponível`
- Nesse modo, o acesso a pastas é limitado pelo browser.

---

## Problema: “não consigo selecionar workspace”

Se você clica em **Selecionar workspace** e nada acontece, verifique:

1. Você está em `runtime: tauri`? Se não, rode:

```bash
cargo run -p codexu_desktop
```

2. Permissões no macOS:
- System Settings → Privacy & Security → Files and Folders
- Dê permissão ao app/terminal para acessar pastas.

3. Tente escolher uma pasta simples (ex.: `~/Documents/teste-codexu`).

4. Veja o painel **Log / Console** para erro detalhado.

---

## O que é GGUF (explicação simples)

- **GGUF** é o formato de arquivo do modelo LLM usado pelo `llama.cpp`.
- Pense nele como “o arquivo de pesos” do modelo (ex.: Code Llama 7B quantizado).
- O app **não** commita esse arquivo no git.

### Preparar pasta de modelos

```bash
bash scripts/setup_models.sh
```

Ou custom:

```bash
bash scripts/setup_models.sh /caminho/para/modelos
```

Depois coloque seu arquivo `.gguf` nessa pasta.

---

## Estrutura principal
- `apps/desktop`: app Tauri (UI + backend local)
- `crates/tools`: tools com guardrails (M2)
- `crates/indexer`: indexação SQLite FTS5 (M3)
- `crates/core_agent`: planner/executor/validator/reporter (base)
- `crates/llm_provider`: providers local/remoto (stub por enquanto)

---

## M3 (indexação e busca)
- Indexador incremental por hash SHA-256.
- FTS5 em SQLite (`files` + `docs_fts`).
- Busca rápida via `ripgrep` (`rg`) e busca por índice SQLite.

---

## Troubleshooting rápido

### `CONNECT tunnel failed, response 403` no cargo
Seu ambiente bloqueou acesso ao `crates.io`. Sem isso, `cargo test`/`cargo run` podem falhar.

### Logs `NSSpellServer ...` no macOS
São logs do corretor ortográfico do sistema, não erro funcional do app.
