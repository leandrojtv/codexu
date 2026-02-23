use anyhow::Result;

pub fn ripgrep_search(_query: &str) -> Result<Vec<String>> {
    Ok(vec!["search mock (M1)".to_string()])
}
