use anyhow::Result;

pub trait LlmProvider {
    fn generate_stream(&self, prompt: &str) -> Result<Vec<String>>;
}

#[derive(Default)]
pub struct LocalLlamaCppProvider;

impl LlmProvider for LocalLlamaCppProvider {
    fn generate_stream(&self, _prompt: &str) -> Result<Vec<String>> {
        Ok(vec![
            "[stub] integração llama.cpp será feita em M4".to_string()
        ])
    }
}

#[derive(Default)]
pub struct RemoteProvider;

impl LlmProvider for RemoteProvider {
    fn generate_stream(&self, _prompt: &str) -> Result<Vec<String>> {
        Ok(vec![
            "[stub] provider remoto opcional será feito em M6".to_string()
        ])
    }
}
