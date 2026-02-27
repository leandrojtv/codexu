use anyhow::Result;

pub struct Executor;

impl Executor {
    pub fn execute_step(&self, step_title: &str) -> Result<String> {
        Ok(format!("execução mock: {step_title}"))
    }
}
