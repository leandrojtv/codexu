use anyhow::Result;

pub struct Validator;

impl Validator {
    pub fn run(&self) -> Result<Vec<String>> {
        Ok(vec![
            "validator mock: sem checks neste milestone".to_string()
        ])
    }
}
