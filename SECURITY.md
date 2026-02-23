# SECURITY

## Guardrails obrigatórios (MVP)

1. **Workspace sandbox (default deny fora do workspace):**
   - Ferramentas de arquivo só podem acessar caminhos dentro do workspace selecionado.
   - Bloquear caminhos sensíveis fora do workspace (`~/.ssh`, Keychain, `.env` externo etc.).

2. **Terminal seguro:**
   - Allowlist de comandos seguros (ex.: `git status`, `git diff`, `cargo test`, `pytest`, `npm test`).
   - Qualquer comando fora da allowlist exige confirmação explícita do usuário.

3. **Diff-first:**
   - O agente sempre propõe `unified diff` antes de aplicar.
   - UI expõe botões `Apply / Reject`.

4. **Backup automático:**
   - Antes de aplicar patch: criar branch temporária `codexu/<timestamp>`.

5. **Offline-first:**
   - Funcionalidade principal deve operar localmente.
   - Modo remoto é opcional e restrito à inferência.

## Estado atual
- M1 implementa esqueleto de guardrails e estrutura.
- M2 concluirá enforcement completo em runtime para tools e fluxo de aplicação de patch.
