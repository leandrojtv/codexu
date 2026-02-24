use anyhow::{Context, Result};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn git_status() -> Result<String> {
    run_git(["status", "--short"])
}

pub fn git_diff() -> Result<String> {
    run_git(["diff", "--"])
}

pub fn create_backup_branch(prefix: &str) -> Result<String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("relógio do sistema inválido")?
        .as_secs();
    let branch = format!("{prefix}/{ts}");

    run_git(["checkout", "-b", &branch])
        .with_context(|| format!("falha ao criar branch de backup {branch}"))?;

    Ok(branch)
}

fn run_git<const N: usize>(args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("falha ao executar git")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git retornou erro: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
