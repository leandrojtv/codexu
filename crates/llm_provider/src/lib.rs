use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

pub trait LlmProvider {
    fn generate_stream(&self, prompt: &str) -> Result<Vec<String>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    pub context_len: usize,
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub endpoint_url: Option<String>,
    pub endpoint_model: String,
    pub cloud_url: Option<String>,
    pub cloud_api_key: Option<String>,
}

fn request_timeout_secs() -> u64 {
    std::env::var("LLAMA_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v >= 30)
        .unwrap_or(180)
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            context_len: 4096,
            max_tokens: 512,
            temperature: 0.2,
            top_p: 0.95,
            endpoint_url: Some("http://127.0.0.1:11434".to_string()),
            endpoint_model: "codellama:7b-instruct".to_string(),
            cloud_url: Some("https://api.openai.com".to_string()),
            cloud_api_key: None,
        }
    }
}

fn sanitize_model_output(raw: &str) -> String {
    let markers = ["<|im_start|>", "<|im_end|>", "<|end|>", "_end|>"];
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            markers
                .iter()
                .fold(line.to_string(), |acc, marker| acc.replace(marker, ""))
        })
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone)]
pub struct EndpointLlmProvider {
    pub config: LlmConfig,
}

impl EndpointLlmProvider {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    fn endpoint_url(&self) -> String {
        self.config
            .endpoint_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:11434".to_string())
    }

    pub fn validate_config(&self) -> Result<()> {
        let url = format!("{}/api/tags", self.endpoint_url().trim_end_matches('/'));
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("falha ao criar client HTTP")?;
        let resp = client
            .get(url)
            .send()
            .context("falha ao conectar no endpoint docker")?;
        if !resp.status().is_success() {
            bail!("endpoint docker retornou status HTTP {}", resp.status());
        }
        Ok(())
    }
}

impl LlmProvider for EndpointLlmProvider {
    fn generate_stream(&self, prompt: &str) -> Result<Vec<String>> {
        let base = self.endpoint_url();
        let url = format!("{}/api/generate", base.trim_end_matches('/'));
        let client = Client::builder()
            .timeout(Duration::from_secs(request_timeout_secs()))
            .build()
            .context("falha ao criar client HTTP")?;

        let body = serde_json::json!({
            "model": self.config.endpoint_model,
            "prompt": prompt,
            "stream": false,
            "options": {
              "temperature": self.config.temperature,
              "top_p": self.config.top_p,
              "num_ctx": self.config.context_len,
              "num_predict": self.config.max_tokens
            }
        });

        let resp = client
            .post(url)
            .json(&body)
            .send()
            .context("falha ao chamar endpoint docker")?;

        if !resp.status().is_success() {
            let code = resp.status();
            let txt = resp.text().unwrap_or_default();
            bail!("endpoint docker retornou erro HTTP {code}: {txt}");
        }

        let payload: Value = resp
            .json()
            .context("resposta JSON inválida do endpoint docker")?;
        let text = payload
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let sanitized = sanitize_model_output(&text);
        if sanitized.trim().is_empty() {
            return Ok(vec!["[llm-docker] resposta vazia".to_string()]);
        }

        Ok(sanitized
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect())
    }
}

#[derive(Debug, Clone)]
pub struct CloudLlmProvider {
    pub config: LlmConfig,
}

impl CloudLlmProvider {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    fn cloud_url(&self) -> String {
        self.config
            .cloud_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com".to_string())
    }

    pub fn validate_config(&self) -> Result<()> {
        if self
            .config
            .cloud_api_key
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            bail!("cloud api key ausente")
        }
        Ok(())
    }
}

impl LlmProvider for CloudLlmProvider {
    fn generate_stream(&self, prompt: &str) -> Result<Vec<String>> {
        self.validate_config()?;
        let url = format!(
            "{}/v1/chat/completions",
            self.cloud_url().trim_end_matches('/')
        );

        let body = serde_json::json!({
            "model": self.config.endpoint_model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": self.config.temperature,
            "top_p": self.config.top_p,
            "max_tokens": self.config.max_tokens,
            "stream": false
        });

        let client = Client::builder()
            .timeout(Duration::from_secs(request_timeout_secs()))
            .build()
            .context("falha ao criar client HTTP")?;

        let resp = client
            .post(url)
            .bearer_auth(self.config.cloud_api_key.clone().unwrap_or_default())
            .json(&body)
            .send()
            .context("falha ao chamar endpoint cloud")?;

        if !resp.status().is_success() {
            let code = resp.status();
            let txt = resp.text().unwrap_or_default();
            bail!("cloud retornou erro HTTP {code}: {txt}");
        }

        let payload: Value = resp.json().context("resposta JSON inválida do cloud")?;
        let text = payload
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let sanitized = sanitize_model_output(&text);
        if sanitized.trim().is_empty() {
            return Ok(vec!["[llm-cloud] resposta vazia".to_string()]);
        }

        Ok(sanitized
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect())
    }
}

pub fn model_recommendation_for_18gb() -> &'static str {
    "Docker: codellama:7b-instruct | Cloud: modelo compatível OpenAI"
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
