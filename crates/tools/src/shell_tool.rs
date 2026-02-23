use anyhow::{bail, Result};

const ALLOWLIST: &[&str] = &[
    "git status",
    "git diff",
    "cargo test",
    "cargo fmt",
    "pytest",
    "npm test",
];

pub fn is_allowed(command: &str) -> bool {
    ALLOWLIST.iter().any(|allowed| command.starts_with(allowed))
}

pub fn validate_command(command: &str) -> Result<()> {
    if is_allowed(command) {
        return Ok(());
    }

    bail!("comando fora da allowlist exige confirmação")
}
