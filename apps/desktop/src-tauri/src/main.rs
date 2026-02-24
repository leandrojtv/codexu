#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;
use tauri::{api::dialog::blocking::FileDialogBuilder, AppHandle, Manager, State};

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

#[tauri::command]
fn select_workspace(app: AppHandle, state: State<'_, AppState>) -> Result<Option<String>, String> {
    println!("[backend] select_workspace called");

    let selected = FileDialogBuilder::new()
        .set_title("Selecione o workspace")
        .pick_folder();

    match selected {
        Some(path_buf) => {
            let path = path_buf.display().to_string();
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
    let assistant_message = format!(
        "Entendi. Vou trabalhar no pedido: \"{}\". Primeiro monto o plano e em seguida proponho um diff incremental.",
        message
    );
    let diff_text =
        Some("--- a/README.md\n+++ b/README.md\n@@ -1 +1 @@\n- old\n+ new\n".to_string());

    println!("[backend] response generated");
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
            select_workspace,
            get_workspace,
            send_chat_message
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar app");
}
