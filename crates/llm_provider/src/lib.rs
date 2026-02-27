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

    fn is_azure_endpoint(&self) -> bool {
        self.cloud_url().contains("openai.azure.com")
    }

    fn is_responses_url(&self, url: &str) -> bool {
        url.contains("/responses") || url.contains("/openai/responses")
    }

    fn preferred_auth_mode(&self) -> &'static str {
        match std::env::var("CODEXU_CLOUD_AUTH_MODE") {
            Ok(v) if v.eq_ignore_ascii_case("api-key") => "api-key",
            Ok(v) if v.eq_ignore_ascii_case("bearer") => "bearer",
            _ => "auto",
        }
    }

    fn auth_attempt_order(&self) -> Vec<&'static str> {
        match self.preferred_auth_mode() {
            "api-key" => vec!["api-key", "bearer"],
            "bearer" => vec!["bearer", "api-key"],
            _ if self.is_azure_endpoint() => vec!["bearer", "api-key"],
            _ => vec!["bearer"],
        }
    }

    fn build_cloud_body(
        &self,
        prompt: &str,
        is_responses: bool,
        include_temperature: bool,
        include_top_p: bool,
    ) -> Value {
        let mut base = if is_responses {
            serde_json::json!({
                "model": self.config.endpoint_model,
                "messages": [{"role": "user", "content": prompt}],
                "max_completion_tokens": self.config.max_tokens,
                "stream": false
            })
        } else {
            serde_json::json!({
                "model": self.config.endpoint_model,
                "messages": [{"role": "user", "content": prompt}],
                "max_tokens": self.config.max_tokens,
                "stream": false
            })
        };

        if let Some(obj) = base.as_object_mut() {
            if include_temperature {
                obj.insert(
                    "temperature".to_string(),
                    serde_json::json!(self.config.temperature),
                );
            }
            if include_top_p {
                obj.insert("top_p".to_string(), serde_json::json!(self.config.top_p));
            }
        }

        base
    }

    fn handle_unsupported_parameter_error(
        &self,
        error_text: &str,
        include_temperature: &mut bool,
        include_top_p: &mut bool,
    ) -> bool {
        let lower = error_text.to_lowercase();
        if !lower.contains("unsupported parameter") {
            return false;
        }

        if lower.contains("temperature") && *include_temperature {
            *include_temperature = false;
            return true;
        }

        if lower.contains("top_p") && *include_top_p {
            *include_top_p = false;
            return true;
        }

        false
    }

    fn resolved_cloud_request_url(&self) -> String {
        let raw = self.cloud_url();
        let trimmed = raw.trim().trim_end_matches('/').to_string();

        if trimmed.contains("/openai/") || trimmed.contains("/v1/") || trimmed.contains("?") {
            return trimmed;
        }

        format!("{trimmed}/v1/chat/completions")
    }

    fn extract_text_from_cloud_payload(&self, payload: &Value, request_url: &str) -> String {
        if self.is_responses_url(request_url) {
            if let Some(out) = payload.get("output_text").and_then(|v| v.as_str()) {
                return out.to_string();
            }

            if let Some(items) = payload.get("output").and_then(|v| v.as_array()) {
                let mut collected = String::new();
                for item in items {
                    if let Some(contents) = item.get("content").and_then(|v| v.as_array()) {
                        for content in contents {
                            if let Some(text) = content.get("text").and_then(|v| v.as_str()) {
                                if !collected.is_empty() {
                                    collected.push('\n');
                                }
                                collected.push_str(text);
                            }
                        }
                    }
                }
                if !collected.trim().is_empty() {
                    return collected;
                }
            }
        }

        payload
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
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
        let url = self.resolved_cloud_request_url();
        let is_responses = self.is_responses_url(&url);

        let client = Client::builder()
            .timeout(Duration::from_secs(request_timeout_secs()))
            .build()
            .context("falha ao criar client HTTP")?;

        let mut include_temperature = true;
        let mut include_top_p = true;
        let mut last_error = String::new();

        for _ in 0..4 {
            let body =
                self.build_cloud_body(prompt, is_responses, include_temperature, include_top_p);
            let mut payload: Option<Value> = None;

            for mode in self.auth_attempt_order() {
                let req = client.post(&url).json(&body);
                let req = match mode {
                    "api-key" => req.header(
                        "api-key",
                        self.config.cloud_api_key.clone().unwrap_or_default(),
                    ),
                    _ => req.bearer_auth(self.config.cloud_api_key.clone().unwrap_or_default()),
                };

                let resp = req.send().context("falha ao chamar endpoint cloud")?;
                if resp.status().is_success() {
                    payload = Some(resp.json().context("resposta JSON inválida do cloud")?);
                    break;
                }

                let code = resp.status();
                let txt = resp.text().unwrap_or_default();
                last_error = format!("auth={mode} -> HTTP {code}: {txt}");

                if code.as_u16() == 400
                    && self.handle_unsupported_parameter_error(
                        &txt,
                        &mut include_temperature,
                        &mut include_top_p,
                    )
                {
                    payload = None;
                    break;
                }

                if !matches!(code.as_u16(), 401 | 403) {
                    payload = None;
                    break;
                }
            }

            if let Some(payload) = payload {
                let text = self.extract_text_from_cloud_payload(&payload, &url);
                let sanitized = sanitize_model_output(&text);
                if sanitized.trim().is_empty() {
                    return Ok(vec!["[llm-cloud] resposta vazia".to_string()]);
                }

                return Ok(sanitized
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.to_string())
                    .collect());
            }
        }

        bail!("cloud retornou erro: {last_error}")
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
