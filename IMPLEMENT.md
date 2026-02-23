# IMPLEMENT (Runbook)

## Princípios operacionais
1. Trabalhar **incrementalmente por milestone** (M1 -> M6).
2. Não ampliar escopo sem necessidade.
3. Toda alteração de arquivo no fluxo do app deve ser **diff-first** (propor unified diff antes de aplicar).
4. Antes de aplicar mudanças no workspace do usuário, criar branch temporária `codexu/<timestamp>`.
5. Sempre rodar validações locais mínimas após alterações relevantes.

## Fluxo de execução por milestone
1. Planejar e atualizar `PLAN.md`.
2. Implementar somente o escopo do milestone atual.
3. Rodar checks:
   - `cargo fmt --all`
   - `cargo test --workspace`
4. Registrar limitações conhecidas no README/SECURITY.

## Definição de pronto (M1)
- Estrutura de monorepo criada.
- Desktop app inicial abre com UI mínima contendo:
  - Chat
  - Plan/Steps
  - Diff Viewer + Apply/Reject
  - Log/Console
- Seleção de workspace em modo inicial (mock/backend command).
