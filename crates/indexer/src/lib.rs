use anyhow::Result;

#[derive(Default)]
pub struct Indexer;

impl Indexer {
    pub fn index_workspace(&self, _workspace: &str) -> Result<()> {
        Ok(())
    }
}
