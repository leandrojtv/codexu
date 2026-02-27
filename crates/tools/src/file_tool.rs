use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorkspaceGuard {
    root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PatchProposal {
    pub relative_path: PathBuf,
    pub diff: String,
}

impl WorkspaceGuard {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_in_workspace(&self, candidate: &Path) -> Result<()> {
        let root = self.root.canonicalize().with_context(|| {
            format!("falha ao canonicalizar workspace: {}", self.root.display())
        })?;

        let candidate_abs = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            root.join(candidate)
        };

        let normalized = normalize_path(&candidate_abs);
        if !normalized.starts_with(&root) {
            bail!("acesso fora do workspace bloqueado")
        }

        if is_sensitive_outside_workspace(&normalized, &root) {
            bail!("caminho sensível bloqueado fora do workspace")
        }

        Ok(())
    }

    pub fn read_file(&self, relative_path: &Path) -> Result<String> {
        self.ensure_in_workspace(relative_path)?;
        let full = self.root.join(relative_path);
        fs::read_to_string(&full)
            .with_context(|| format!("falha ao ler arquivo: {}", full.display()))
    }

    pub fn write_new_file(
        &self,
        relative_path: &Path,
        content: &str,
        approved: bool,
    ) -> Result<()> {
        if !approved {
            bail!("diff-first: write_new_file exige aprovação explícita")
        }

        self.ensure_in_workspace(relative_path)?;
        let full = self.root.join(relative_path);
        if full.exists() {
            bail!("write_new_file só permite criar arquivos novos")
        }

        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("falha ao criar diretório: {}", parent.display()))?;
        }

        fs::write(&full, content)
            .with_context(|| format!("falha ao gravar arquivo: {}", full.display()))
    }

    pub fn apply_patch(&self, proposal: &PatchProposal, approved: bool) -> Result<()> {
        if !approved {
            bail!("diff-first: apply_patch exige aprovação explícita")
        }

        self.ensure_in_workspace(&proposal.relative_path)?;
        if !proposal.diff.contains("--- ") || !proposal.diff.contains("+++ ") {
            bail!("patch inválido: unified diff incompleto")
        }

        let target = self.root.join(&proposal.relative_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("falha ao criar diretório: {}", parent.display()))?;
        }

        // MVP M2: persistimos o patch proposto para revisão/aplicação posterior.
        let patch_path = target.with_extension("patch.pending");
        fs::write(&patch_path, &proposal.diff)
            .with_context(|| format!("falha ao salvar patch pendente: {}", patch_path.display()))
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn is_sensitive_outside_workspace(candidate: &Path, workspace_root: &Path) -> bool {
    if candidate.starts_with(workspace_root) {
        return false;
    }

    let s = candidate.display().to_string();
    s.contains("/.ssh") || s.contains("/Library/Keychains") || s.ends_with(".env")
}

pub fn parse_unified_diff(relative_path: impl Into<PathBuf>, diff: &str) -> Result<PatchProposal> {
    if diff.trim().is_empty() {
        return Err(anyhow!("diff vazio"));
    }

    Ok(PatchProposal {
        relative_path: relative_path.into(),
        diff: diff.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_workspace() -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("codexu-tools-{ts}"));
        fs::create_dir_all(&root).expect("mkdir");
        root
    }

    #[test]
    fn blocks_outside_workspace() {
        let root = test_workspace();
        let guard = WorkspaceGuard::new(&root);
        let outside = PathBuf::from("/tmp/outside.txt");
        assert!(guard.ensure_in_workspace(&outside).is_err());
    }

    #[test]
    fn requires_approval_for_patch() {
        let root = test_workspace();
        let guard = WorkspaceGuard::new(&root);
        let proposal = PatchProposal {
            relative_path: PathBuf::from("src/lib.rs"),
            diff: "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@\n".to_string(),
        };

        assert!(guard.apply_patch(&proposal, false).is_err());
        assert!(guard.apply_patch(&proposal, true).is_ok());
    }
}
