use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub trait LlmProvider {
    fn generate_stream(&self, prompt: &str) -> Result<Vec<String>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    pub model_path: PathBuf,
    pub binary_path: Option<PathBuf>,
    pub context_len: usize,
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub gpu_layers: usize,
}

fn default_model_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".codexu")
            .join("models")
            .join("code-llama-7b-q4_k_m.gguf");
    }

    PathBuf::from(".codexu/models/code-llama-7b-q4_k_m.gguf")
}

fn expand_home(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix("$HOME/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    path.to_path_buf()
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model_path: default_model_path(),
            binary_path: None,
            context_len: 4096,
            max_tokens: 512,
            temperature: 0.2,
            top_p: 0.95,
            gpu_layers: 99,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalLlamaCppProvider {
    pub config: LlmConfig,
}

impl LocalLlamaCppProvider {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    pub fn validate_config(&self) -> Result<()> {
        let model = expand_home(&self.config.model_path);
        if model.extension().and_then(|e| e.to_str()) != Some("gguf") {
            bail!("model_path precisa apontar para arquivo .gguf")
        }
        if !model.exists() {
            bail!("modelo GGUF não encontrado em {}", model.display())
        }
        Ok(())
    }

    fn llama_binary(&self) -> String {
        if let Some(path) = &self.config.binary_path {
            return path.display().to_string();
        }
        std::env::var("LLAMA_CPP_BINARY").unwrap_or_else(|_| "llama-cli".to_string())
    }
}

fn is_noise_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.len() > 8 && trimmed.chars().all(|c| c == '>')
}

fn sanitize_model_output(raw: &str) -> String {
    let markers = [
        "<|im_start|>",
        "<|im_end|>",
        "<|end|>",
        "<|eot_id|>",
        "<|start_header_id|>",
        "<|end_header_id|>",
        "<|assistant|>",
        "<|user|>",
        "<|system|>",
        "_end|>",
    ];

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
        .filter(|line| !is_noise_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

impl Default for LocalLlamaCppProvider {
    fn default() -> Self {
        Self {
            config: LlmConfig::default(),
        }
    }
}

impl LlmProvider for LocalLlamaCppProvider {
    fn generate_stream(&self, prompt: &str) -> Result<Vec<String>> {
        self.validate_config()?;

        let bin = self.llama_binary();
        let mut child = Command::new(&bin)
            .args([
                "-m",
                expand_home(&self.config.model_path)
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("model_path inválido"))?,
                "-p",
                prompt,
                "-n",
                &self.config.max_tokens.to_string(),
                "-c",
                &self.config.context_len.to_string(),
                "--temp",
                &self.config.temperature.to_string(),
                "--top-p",
                &self.config.top_p.to_string(),
                "-ngl",
                &self.config.gpu_layers.to_string(),
                "--no-display-prompt",
                "--simple-io",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("falha ao executar {bin}. Instale/compile o llama.cpp"))?;

        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if child.try_wait()?.is_some() {
                break;
            }

            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                bail!(
                    "llama.cpp excedeu o tempo limite (60s). Isso pode indicar template/prompt incompatível com o modelo GGUF."
                );
            }

            thread::sleep(Duration::from_millis(50));
        }

        let output = child
            .wait_with_output()
            .with_context(|| "falha ao coletar saída do llama.cpp".to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("llama.cpp retornou erro: {stderr}")
        }

        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let sanitized = sanitize_model_output(&text);
        if sanitized.trim().is_empty() {
            return Ok(vec![
                "[llm] resposta vazia ou somente tokens de controle".to_string()
            ]);
        }

        let chunks = sanitized
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect::<Vec<_>>();

        if chunks.is_empty() {
            Ok(vec![sanitized])
        } else {
            Ok(chunks)
        }
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

pub fn model_recommendation_for_18gb() -> &'static str {
    "Code Llama 7B GGUF (Q4_K_M recomendado; Q5_K_M opcional)"
}

pub fn is_gguf(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("gguf")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_gguf() {
        assert!(is_gguf(Path::new("model.gguf")));
        assert!(!is_gguf(Path::new("model.bin")));
    }

    #[test]
    fn validates_model_path_extension() {
        let provider = LocalLlamaCppProvider::new(LlmConfig {
            model_path: PathBuf::from("/tmp/model.bin"),
            ..LlmConfig::default()
        });

        assert!(provider.validate_config().is_err());
    }

    #[test]
    fn sanitize_control_tokens() {
        let raw = "<|im_start|>\n<|im_end|>\nOlá\n_end|>\n";
        assert_eq!(sanitize_model_output(raw), "Olá");
    }
}
