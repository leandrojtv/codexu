use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorkspaceGuard {
    root: PathBuf,
}

impl WorkspaceGuard {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn ensure_in_workspace(&self, candidate: &Path) -> Result<()> {
        let candidate = candidate.canonicalize()?;
        let root = self.root.canonicalize()?;
        if !candidate.starts_with(&root) {
            bail!("acesso fora do workspace bloqueado")
        }
        Ok(())
    }
}
