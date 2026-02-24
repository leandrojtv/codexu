# Codexu (macOS Apple Silicon)

App desktop local-first (Tauri + Rust) para fluxo estilo agente de código.

## Status atual
- ✅ M1: app desktop mínimo + chat/painéis.
- ✅ M2: guardrails de tools (sandbox, diff-first, shell policy, backup branch).
- ✅ M3: indexador SQLite FTS5 + busca com ripgrep.
- ✅ M4: provider local llama.cpp (Metal) + streaming MVP.
- 🚧 Próximo: M5 (loop do agente fim-a-fim).

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
- **CMake** (obrigatório para compilar `llama.cpp`)

Instalação rápida (Homebrew):

```bash
brew install cmake
```

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
- título **Codexu (M4)**

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


## M4: LLM local com llama.cpp (Metal)

### O que foi implementado no código
- `crates/llm_provider::LlmConfig` com parâmetros do modelo.
- `LocalLlamaCppProvider` que executa `llama-cli` localmente.
- Validação de `model_path` `.gguf` e existência do arquivo.
- Streaming MVP (chunks por linha de saída).

### Passo a passo exato (macOS Apple Silicon)

1. Preparar diretório de modelos:

```bash
bash scripts/setup_models.sh
```

2. Colocar o arquivo GGUF na pasta criada (exemplo):

```bash
$HOME/.codexu/models/code-llama-7b-q4_k_m.gguf
```

3. Instalar/compilar o llama.cpp com Metal:

```bash
git clone https://github.com/ggerganov/llama.cpp.git
cd llama.cpp
cmake -B build -DGGML_METAL=ON
cmake --build build -j
```

4. Definir binário para o app (se necessário):

```bash
export LLAMA_CPP_BINARY="/caminho/llama.cpp/build/bin/llama-cli"
```

5. Rodar o app:

```bash
cd /caminho/do/codexu
cargo run -p codexu_desktop
```

### Recomendação para Mac com 18GB RAM
- Preferir `Code Llama 7B` quantizado:
  - `Q4_K_M` (default recomendado)
  - `Q5_K_M` (opcional)


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


### `cargo run -p codexu_desktop` mas ainda aparece `runtime: browser fallback`
Isso normalmente indica que o frontend não recebeu o objeto global do Tauri.

Checklist:
1. Verifique em `apps/desktop/src-tauri/tauri.conf.json` se `build.withGlobalTauri` está `true`.
2. Rode limpeza e execute novamente:

```bash
cargo clean
cargo run -p codexu_desktop
```

3. No Log / Console do app, veja a linha de debug:
   - `__TAURI__=true` esperado no runtime desktop.



### `zsh: command not found: cmake`
Você não tem o CMake instalado no macOS.

Instale com Homebrew:

```bash
brew install cmake
```

Valide:

```bash
cmake --version
```

Depois repita a build do `llama.cpp`:

```bash
cmake -B build -DGGML_METAL=ON
cmake --build build -j
```
