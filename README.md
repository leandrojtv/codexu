# Codexu (macOS Apple Silicon)

Aplicativo desktop local-first inspirado em agentes de engenharia (Codex/Claude Code), usando **Tauri + Rust**.

## Status
- ✅ **M1 concluído**: scaffolding, docs e UI mínima.
- ✅ **M2 concluído**: guardrails base nas tools (workspace sandbox, diff-first, backup branch, shell policy).
- 🚧 **Próximo**: M3 (indexer SQLite FTS + ripgrep).

## O que existe hoje (M1)
- Monorepo Rust com crates de agente, tools, indexador e provider.
- App desktop Tauri com UI mínima contendo:
  - Chat
  - Plan/Steps
  - Diff Viewer (Apply/Reject)
  - Log/Console
  - Seleção inicial de workspace

## Requisitos (macOS arm64)

### 1) Ferramentas base
- macOS Apple Silicon (arm64)
- Xcode Command Line Tools
- Rust (stable) + Cargo
- Node.js (recomendado para evolução da UI; no M1 a UI é estática)

### 2) Pré-requisitos Tauri (macOS)
Instale os requisitos oficiais do Tauri para macOS. Em geral:
- Toolchain Rust
- Xcode/SDKs

Referência: documentação oficial do Tauri para setup em macOS.

---

## Como compilar e rodar

### Passo a passo rápido

1. Clone o repositório e entre na pasta:

```bash
git clone <seu-repo>
cd codexu
```

2. Formate o código:

```bash
cargo fmt --all
```

3. Rode testes do workspace:

```bash
cargo test --workspace
```

4. Rode o app desktop em modo desenvolvimento:

```bash
cargo run -p codexu_desktop
```

> Se o seu ambiente estiver sem acesso ao `crates.io`, o `cargo test` e o `cargo run` podem falhar no download de dependências.

### Build de release (app .app no macOS)

```bash
cargo build -p codexu_desktop --release
```

Para empacotamento Tauri em `.app`, use o fluxo Tauri conforme os pré-requisitos do seu ambiente estiverem completos.

---

## Como usar o app (M1)

Ao abrir o app:

1. Clique em **Selecionar workspace**.
2. Informe o caminho absoluto de uma pasta local (mock inicial no M1).
3. Use o campo de **Chat** para registrar o pedido.
4. Acompanhe os passos no painel **Plan / Steps**.
5. Veja alterações propostas no **Diff Viewer** (botões Apply/Reject estão presentes como UI inicial).
6. Verifique eventos em **Log / Console**.

> Observação: no M1, os fluxos de aplicação real de patch/guardrails completos ainda serão concluídos no M2.

---

## Modelo GGUF (preparação)

Os pesos **não** ficam no repositório.

Crie a pasta de modelos:

```bash
bash scripts/setup_models.sh
```

Ou defina um diretório customizado:

```bash
bash scripts/setup_models.sh /caminho/para/modelos
```

Depois:
1. Baixe um Code Llama 7B GGUF quantizado (`Q4_K_M` ou `Q5_K_M`) de fonte confiável.
2. Coloque o `.gguf` no diretório criado.
3. A integração efetiva com `llama.cpp` será habilitada no **M4**.

---

## Estrutura do projeto
- O ícone do app para identificação na barra de tarefas/menu dock é `apps/desktop/src-tauri/icons/icon.png` (gerado automaticamente no build).
- `crates/core_agent`: planner/executor/validator/reporter
- `crates/tools`: file/search/git/shell tools
- `crates/indexer`: indexador (FTS em M3)
- `crates/llm_provider`: trait de provider + stubs local/remoto
- `apps/desktop`: app Tauri

## Segurança e guardrails
As decisões de segurança estão em `SECURITY.md`:
- workspace sandbox
- allowlist de terminal + confirmação
- diff-first
- backup branch temporária antes de aplicar mudanças

## Roadmap
Consulte `PLAN.md` para milestones M1 → M6 e progresso.

---


## Troubleshooting

### Erro no `cargo test --workspace` com `tauri::generate_context!()`
Se aparecer erro de macro do Tauri reclamando de `icons/icon.png` inexistente, não versione binário manualmente.

Neste projeto, o arquivo é gerado automaticamente no build por `apps/desktop/src-tauri/build.rs` antes do `tauri_build::build()`.

O caminho `apps/desktop/src-tauri/icons/icon.png` está no `.gitignore` para evitar que binários entrem na branch por acidente.

Se necessário, rode novamente:

```bash
cargo clean
cargo test --workspace
```


### Mensagem `Tauri runtime indisponível` no Log / Console
Isso ocorre quando a UI é aberta fora do runtime Tauri (por exemplo, via `http.server`).

- Em modo navegador, o app usa fallback limitado (sem comandos backend reais). A seleção tenta `showDirectoryPicker` e depois fallback adicional (inclusive entrada manual) para não ficar travada em "Nenhum workspace selecionado".
- Para fluxo completo (seletor nativo + chat backend), execute pelo app Tauri:

```bash
cargo run -p codexu_desktop
```

### Cliquei em "Selecionar workspace" e nada acontece
Em alguns ambientes, o fluxo assíncrono do seletor pode não retornar corretamente.

A versão atual usa seletor nativo em modo bloqueante no backend Tauri para garantir que a pasta selecionada seja retornada para a UI.

Se ainda não abrir/retornar seleção:
- Execute o app via Tauri (`cargo run -p codexu_desktop`), não via navegador puro.
- Verifique permissões do macOS para janelas/dialogs do app.
- Confira o painel **Log / Console** para mensagens de erro.


### Logs `NSSpellServer ... timed out/succeeded` no macOS
Essas mensagens vêm do serviço de correção ortográfica do macOS/WebKit e **não** indicam falha funcional do app.

Para reduzir ruído no campo de chat, o input foi configurado com `spellcheck="false"` e `autocorrect="off"`.

