#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use llm_provider::{model_recommendation_for_18gb, LlmConfig, LlmProvider, LocalLlamaCppProvider};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{api::dialog::FileDialogBuilder, AppHandle, Manager, State};
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};

#[derive(Default)]
struct AppState {
    workspace: Mutex<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatResponse {
    assistant_message: String,
    plan_steps: Vec<String>,
    diff_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppMode {
    runtime: String,
    version: String,
    milestone: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LlamaSetupStatus {
    ok: bool,
    using_local_llm: bool,
    model_path: String,
    binary: String,
    recommendation: String,
    details: Vec<String>,
}

fn workspace_config_file(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let config_dir = app
        .path_resolver()
        .app_config_dir()
        .ok_or_else(|| "não foi possível resolver app_config_dir".to_string())?;
    fs::create_dir_all(&config_dir).map_err(|e| format!("falha ao criar config dir: {e}"))?;
    Ok(config_dir.join("workspace_path.txt"))
}

fn persist_workspace(app: &AppHandle, path: &str) -> Result<(), String> {
    let file = workspace_config_file(app)?;
    fs::write(&file, path).map_err(|e| format!("falha ao persistir workspace: {e}"))?;
    println!("[backend] workspace persisted: {path}");
    Ok(())
}

fn load_persisted_workspace(app: &AppHandle) -> Result<Option<String>, String> {
    let file = workspace_config_file(app)?;
    if !file.exists() {
        return Ok(None);
    }

    let raw =
        fs::read_to_string(&file).map_err(|e| format!("falha ao ler workspace persistido: {e}"))?;
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Ok(Some(trimmed))
}

fn local_llm_enabled() -> bool {
    match std::env::var("CODEXU_USE_LOCAL_LLM") {
        Ok(v) if v == "0" => false,
        _ => true,
    }
}

fn derive_plan_steps(message: &str) -> Vec<String> {
    let lower = message.to_lowercase();
    let mut steps = vec!["Analisar objetivo do usuário".to_string()];

    if lower.contains("bug") || lower.contains("erro") || lower.contains("falha") {
        steps.push("Investigar causa raiz no código".to_string());
        steps.push("Propor correção com diff".to_string());
    } else if lower.contains("teste") {
        steps.push("Identificar testes impactados".to_string());
        steps.push("Executar validações relevantes".to_string());
    } else {
        steps.push("Mapear arquivos relevantes no workspace".to_string());
        steps.push("Propor alterações incrementais".to_string());
    }

    steps.push("Validar resultado e resumir".to_string());
    steps
}

fn normalize_env_value(raw: &str) -> Option<String> {
    let mut value = raw.trim().to_string();
    if value.is_empty() {
        return None;
    }

    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        value = value[1..value.len().saturating_sub(1)].trim().to_string();
    }

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn expand_tilde_path(value: &str) -> PathBuf {
    if value == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    }

    if let Some(rest) = value.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }

    PathBuf::from(value)
}

fn read_env_path(key: &str) -> Option<PathBuf> {
    let raw = std::env::var(key).ok()?;
    let normalized = normalize_env_value(&raw)?;
    Some(expand_tilde_path(&normalized))
}

fn detect_first_gguf_in_default_models_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let models_dir = PathBuf::from(home).join(".codexu").join("models");
    let entries = fs::read_dir(models_dir).ok()?;

    let mut ggufs = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("gguf"))
                    .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    ggufs.sort();
    ggufs.into_iter().next()
}

fn resolve_model_path() -> (PathBuf, Vec<String>) {
    let mut notes = Vec::new();

    if let Some(env_model) = read_env_path("MODEL_GGUF_PATH") {
        if env_model.exists() {
            notes.push(format!(
                "MODEL_GGUF_PATH detectado e válido: {}",
                env_model.display()
            ));
            return (env_model, notes);
        }

        notes.push(format!(
            "MODEL_GGUF_PATH detectado, mas arquivo não existe: {}",
            env_model.display()
        ));
    } else {
        notes.push("MODEL_GGUF_PATH não definido no processo".to_string());
    }

    if let Some(auto) = detect_first_gguf_in_default_models_dir() {
        notes.push(format!(
            "fallback automático: usando GGUF encontrado em ~/.codexu/models -> {}",
            auto.display()
        ));
        return (auto, notes);
    }

    notes.push("fallback automático não encontrou nenhum .gguf em ~/.codexu/models".to_string());
    (LlmConfig::default().model_path, notes)
}

fn candidate_binaries_from_env_path(path: &PathBuf) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .windows(2)
        .any(|w| w[0] == "llama.cpp" && w[1] == "llama.cpp")
    {
        let deduped = path
            .to_string_lossy()
            .replace("/llama.cpp/llama.cpp/", "/llama.cpp/");
        candidates.push(PathBuf::from(deduped));
    }

    if let Some(parent) = path.parent() {
        candidates.push(parent.join("llama-cli"));
    }

    candidates
}

fn resolve_binary_path() -> (Option<PathBuf>, Vec<String>) {
    let mut notes = Vec::new();

    if let Some(env_bin) = read_env_path("LLAMA_CPP_BINARY") {
        if env_bin.exists() {
            notes.push(format!(
                "LLAMA_CPP_BINARY detectado e válido: {}",
                env_bin.display()
            ));
            return (Some(env_bin), notes);
        }

        notes.push(format!(
            "LLAMA_CPP_BINARY detectado, mas arquivo não existe: {}",
            env_bin.display()
        ));

        for candidate in candidate_binaries_from_env_path(&env_bin) {
            if candidate.exists() {
                notes.push(format!(
                    "fallback automático: binário corrigido detectado -> {}",
                    candidate.display()
                ));
                return (Some(candidate), notes);
            }
        }

        notes.push("fallback automático: usando llama-cli via PATH".to_string());
        return (None, notes);
    }

    notes.push("LLAMA_CPP_BINARY não definido no processo; usando llama-cli via PATH".to_string());
    (None, notes)
}

fn build_llm_config_from_env() -> LlmConfig {
    let mut cfg = LlmConfig::default();
    cfg.model_path = resolve_model_path().0;
    cfg.binary_path = resolve_binary_path().0;
    cfg
}

#[tauri::command]
fn get_app_mode(app: AppHandle) -> AppMode {
    AppMode {
        runtime: "tauri".to_string(),
        version: app.package_info().version.to_string(),
        milestone: "M4".to_string(),
    }
}

#[tauri::command]
fn validate_llama_setup() -> LlamaSetupStatus {
    let using_local_llm = local_llm_enabled();
    let cfg = build_llm_config_from_env();
    let provider = LocalLlamaCppProvider::new(cfg.clone());

    let binary = cfg
        .binary_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "llama-cli (PATH)".to_string());

    let mut details = Vec::new();
    let mut ok = true;

    for note in resolve_model_path().1 {
        details.push(note);
    }

    for note in resolve_binary_path().1 {
        details.push(note);
    }

    match provider.validate_config() {
        Ok(()) => details.push(format!(
            "modelo GGUF válido e encontrado: {}",
            cfg.model_path.display()
        )),
        Err(e) => {
            ok = false;
            details.push(format!("erro no modelo: {e}"));
        }
    }

    let bin_to_check = cfg
        .binary_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "llama-cli".to_string());

    let bin_check = std::process::Command::new(&bin_to_check)
        .arg("--version")
        .output();

    match bin_check {
        Ok(out) if out.status.success() => details.push("binário llama.cpp acessível".to_string()),
        Ok(_) => {
            ok = false;
            details
                .push("binário llama.cpp encontrado, mas retornou erro em --version".to_string());
        }
        Err(e) => {
            ok = false;
            details.push(format!("não foi possível executar binário llama.cpp: {e}"));
        }
    }

    if !using_local_llm {
        details.push("CODEXU_USE_LOCAL_LLM=0 (chat em modo mock)".to_string());
    }

    LlamaSetupStatus {
        ok,
        using_local_llm,
        model_path: cfg.model_path.display().to_string(),
        binary,
        recommendation: model_recommendation_for_18gb().to_string(),
        details,
    }
}

#[tauri::command]
async fn select_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    println!("[backend] select_workspace called");

    let (tx, rx) = oneshot::channel::<Option<String>>();
    let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));

    FileDialogBuilder::new()
        .set_title("Selecione o workspace")
        .pick_folder({
            let tx = tx.clone();
            move |folder| {
                if let Ok(mut guard) = tx.lock() {
                    if let Some(sender) = guard.take() {
                        let _ = sender.send(folder.map(|p| p.display().to_string()));
                    }
                }
            }
        });

    let selected = timeout(Duration::from_secs(120), rx)
        .await
        .map_err(|_| "timeout ao aguardar seleção de workspace".to_string())?
        .map_err(|_| "falha ao receber seleção de workspace".to_string())?;

    match selected {
        Some(path) => {
            let metadata =
                fs::metadata(&path).map_err(|e| format!("falha ao validar workspace: {e}"))?;
            if !metadata.is_dir() {
                return Err("workspace selecionado não é um diretório".into());
            }

            let mut guard = state.workspace.lock().map_err(|e| e.to_string())?;
            *guard = Some(path.clone());
            persist_workspace(&app, &path)?;
            println!("[backend] workspace selected: {path}");
            Ok(Some(path))
        }
        None => {
            println!("[backend] workspace selection canceled");
            Ok(None)
        }
    }
}

#[tauri::command]
fn get_workspace(app: AppHandle, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let in_memory = {
        let guard = state.workspace.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };

    if in_memory.is_some() {
        return Ok(in_memory);
    }

    let persisted = load_persisted_workspace(&app)?;
    if let Some(path) = persisted.clone() {
        let mut guard = state.workspace.lock().map_err(|e| e.to_string())?;
        *guard = Some(path);
    }

    Ok(persisted)
}

#[tauri::command]
fn send_chat_message(message: String, state: State<'_, AppState>) -> Result<ChatResponse, String> {
    println!("[backend] send_chat_message called: {message}");
    let workspace = {
        let guard = state.workspace.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };

    if workspace.is_none() {
        return Ok(ChatResponse {
            assistant_message: "Selecione um workspace antes de enviar mensagens.".to_string(),
            plan_steps: vec![
                "Selecionar workspace".to_string(),
                "Reenviar mensagem".to_string(),
                "Executar plano".to_string(),
            ],
            diff_text: None,
        });
    }

    let plan_steps = derive_plan_steps(&message);

    let use_local = local_llm_enabled();
    if use_local {
        let provider = LocalLlamaCppProvider::new(build_llm_config_from_env());
        match provider.generate_stream(&message) {
            Ok(chunks) => {
                let assistant_message = chunks.join("\n");
                println!("[backend] response generated (local llama.cpp)");
                return Ok(ChatResponse {
                    assistant_message,
                    plan_steps,
                    diff_text: None,
                });
            }
            Err(e) => {
                println!("[backend] local llama.cpp falhou: {e}");
                let cfg = build_llm_config_from_env();
                let binary_hint = cfg
                    .binary_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "llama-cli (PATH)".to_string());
                let err_text = e.to_string();
                let guidance = if err_text.contains("tempo limite") {
                    "A geração excedeu o tempo limite. Tente aumentar LLAMA_TIMEOUT_SECS (ex.: 300), reduzir max_tokens/contexto ou usar quantização/modelo mais leve."
                } else {
                    "Instale/compile o llama.cpp e garanta que o binário está acessível."
                };
                return Ok(ChatResponse {
                    assistant_message: format!(
                        "Falha no llama.cpp: {e}.
{guidance}
Use o comando de validação de setup e confira MODEL_GGUF_PATH / LLAMA_CPP_BINARY.
Configuração atual -> MODEL_GGUF_PATH: {} | LLAMA_CPP_BINARY: {}",
                        cfg.model_path.display(),
                        binary_hint
                    ),
                    plan_steps,
                    diff_text: None,
                });
            }
        }
    }

    let assistant_message = format!(
        "[mock] Entendi. Vou trabalhar no pedido: \"{}\".\nLocal LLM está desabilitado (CODEXU_USE_LOCAL_LLM=0).",
        message
    );
    let diff_text =
        Some("--- a/README.md\n+++ b/README.md\n@@ -1 +1 @@\n- old\n+ new\n".to_string());

    println!("[backend] response generated (mock)");
    Ok(ChatResponse {
        assistant_message,
        plan_steps,
        diff_text,
    })
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            let state = app.state::<AppState>();
            if let Ok(Some(path)) = load_persisted_workspace(&app.app_handle()) {
                if let Ok(mut guard) = state.workspace.lock() {
                    *guard = Some(path.clone());
                }
                println!("[backend] workspace restored at startup: {path}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_mode,
            validate_llama_setup,
            select_workspace,
            get_workspace,
            send_chat_message
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar app");
}
