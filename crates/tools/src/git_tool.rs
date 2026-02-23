use anyhow::Result;

pub fn git_status() -> Result<String> {
    Ok("git status mock (M1)".to_string())
}
