use anyhow::{bail, Result};

const ALLOWLIST: &[&str] = &[
    "git status",
    "git diff",
    "cargo test",
    "cargo fmt",
    "pytest",
    "npm test",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellPolicyDecision {
    Allowed,
    RequiresConfirmation,
}

pub fn is_allowed(command: &str) -> bool {
    let trimmed = command.trim();
    ALLOWLIST.iter().any(|allowed| trimmed.starts_with(allowed))
}

pub fn classify_command(command: &str) -> ShellPolicyDecision {
    if is_allowed(command) {
        ShellPolicyDecision::Allowed
    } else {
        ShellPolicyDecision::RequiresConfirmation
    }
}

pub fn validate_command(command: &str, confirmed: bool) -> Result<()> {
    match classify_command(command) {
        ShellPolicyDecision::Allowed => Ok(()),
        ShellPolicyDecision::RequiresConfirmation if confirmed => Ok(()),
        ShellPolicyDecision::RequiresConfirmation => {
            bail!("comando fora da allowlist exige confirmação")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlisted_command_passes_without_confirmation() {
        assert!(validate_command("git status", false).is_ok());
    }

    #[test]
    fn non_allowlisted_requires_confirmation() {
        assert!(validate_command("rm -rf .", false).is_err());
        assert!(validate_command("rm -rf .", true).is_ok());
    }
}
