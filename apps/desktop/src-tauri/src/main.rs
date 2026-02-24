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

fn detect_first_gguf_in_default_models_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let models_dir = PathBuf::from(home).join(".codexu").join("models");
    let entries = fs::read_dir(models_dir).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
            return Some(path);
        }
    }

    None
}

fn build_llm_config_from_env() -> LlmConfig {
    let mut cfg = LlmConfig::default();

    if let Ok(p) = std::env::var("MODEL_GGUF_PATH") {
        cfg.model_path = PathBuf::from(p);
    } else if let Some(auto) = detect_first_gguf_in_default_models_dir() {
        cfg.model_path = auto;
    }

    if let Ok(p) = std::env::var("LLAMA_CPP_BINARY") {
        cfg.binary_path = Some(PathBuf::from(p));
    }

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
                return Ok(ChatResponse {
                    assistant_message: format!(
                        "Falha no llama.cpp: {e}.\nUse o comando de validação de setup e confira MODEL_GGUF_PATH / LLAMA_CPP_BINARY."
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
