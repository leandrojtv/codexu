#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use llm_provider::{CloudLlmProvider, EndpointLlmProvider, LlmConfig, LlmProvider};
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    provider: String,
    docker_endpoint: String,
    cloud_endpoint: String,
    cloud_api_key: Option<String>,
    model: String,
    temperature: f32,
    top_p: f32,
    max_tokens: usize,
    timeout_secs: u64,
    retries: u8,
    theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: "docker".to_string(),
            docker_endpoint: "http://127.0.0.1:11434".to_string(),
            cloud_endpoint: "https://api.openai.com".to_string(),
            cloud_api_key: None,
            model: "codellama:7b-instruct".to_string(),
            temperature: 0.2,
            top_p: 0.95,
            max_tokens: 512,
            timeout_secs: 180,
            retries: 1,
            theme: "dark".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupStatus {
    ok: bool,
    provider: String,
    model: String,
    endpoint: String,
    details: Vec<String>,
}

fn app_codexu_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let config_dir = app
        .path_resolver()
        .app_config_dir()
        .ok_or_else(|| "não foi possível resolver app_config_dir".to_string())?;
    let codexu = config_dir.join(".codexu");
    fs::create_dir_all(&codexu).map_err(|e| format!("falha ao criar .codexu: {e}"))?;
    Ok(codexu)
}

fn workspace_config_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_codexu_dir(app)?.join("workspace_path.txt"))
}

fn settings_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_codexu_dir(app)?.join("config.json"))
}

fn keyring_service() -> &'static str {
    "codexu-desktop"
}

fn load_cloud_api_key_from_keyring() -> Option<String> {
    let entry = keyring::Entry::new(keyring_service(), "cloud_api_key").ok()?;
    entry.get_password().ok()
}

fn save_cloud_api_key_to_keyring(value: Option<&str>) {
    if let Ok(entry) = keyring::Entry::new(keyring_service(), "cloud_api_key") {
        match value {
            Some(v) if !v.trim().is_empty() => {
                let _ = entry.set_password(v);
            }
            _ => {
                let _ = entry.delete_password();
            }
        }
    }
}

fn history_file_for_workspace(workspace: &str) -> PathBuf {
    PathBuf::from(workspace)
        .join(".codexu")
        .join("history.json")
}

fn persist_workspace(app: &AppHandle, path: &str) -> Result<(), String> {
    let file = workspace_config_file(app)?;
    fs::write(&file, path).map_err(|e| format!("falha ao persistir workspace: {e}"))?;
    let recent = app_codexu_dir(app)?.join("recent_workspaces.json");
    let mut list: Vec<String> = if recent.exists() {
        serde_json::from_str(&fs::read_to_string(&recent).unwrap_or_default()).unwrap_or_default()
    } else {
        vec![]
    };
    list.retain(|v| v != path);
    list.insert(0, path.to_string());
    list.truncate(10);
    fs::write(
        recent,
        serde_json::to_string_pretty(&list).unwrap_or_else(|_| "[]".to_string()),
    )
    .map_err(|e| format!("falha ao persistir recentes: {e}"))?;
    Ok(())
}

fn load_persisted_workspace(app: &AppHandle) -> Result<Option<String>, String> {
    let file = workspace_config_file(app)?;
    if !file.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&file).map_err(|e| format!("falha ao ler workspace: {e}"))?;
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(trimmed))
}

fn load_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let file = settings_file(app)?;
    let mut settings = if !file.exists() {
        AppSettings::default()
    } else {
        let raw = fs::read_to_string(file).map_err(|e| format!("falha ao ler config: {e}"))?;
        serde_json::from_str(&raw).map_err(|e| format!("config inválido: {e}"))?
    };

    settings.cloud_api_key = load_cloud_api_key_from_keyring();
    Ok(settings)
}

fn save_settings_inner(app: &AppHandle, mut settings: AppSettings) -> Result<(), String> {
    if settings.provider != "docker" && settings.provider != "cloud" {
        settings.provider = "docker".to_string();
    }
    if settings.timeout_secs < 30 {
        settings.timeout_secs = 30;
    }

    save_cloud_api_key_to_keyring(settings.cloud_api_key.as_deref());
    settings.cloud_api_key = None;

    let file = settings_file(app)?;
    let raw = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(file, raw).map_err(|e| format!("falha ao salvar config: {e}"))
}

fn derive_plan_steps(message: &str) -> Vec<String> {
    let lower = message.to_lowercase();
    let mut steps = vec!["Analisar objetivo do usuário".to_string()];
    if lower.contains("bug") || lower.contains("erro") || lower.contains("falha") {
        steps.push("Investigar causa raiz".to_string());
        steps.push("Propor correção".to_string());
    } else {
        steps.push("Mapear arquivos relevantes".to_string());
        steps.push("Gerar sugestão incremental".to_string());
    }
    steps.push("Validar e resumir".to_string());
    steps
}

fn build_llm_config(settings: &AppSettings) -> LlmConfig {
    LlmConfig {
        endpoint_url: Some(settings.docker_endpoint.clone()),
        endpoint_model: settings.model.clone(),
        cloud_url: Some(settings.cloud_endpoint.clone()),
        cloud_api_key: settings.cloud_api_key.clone(),
        temperature: settings.temperature,
        top_p: settings.top_p,
        max_tokens: settings.max_tokens,
        ..LlmConfig::default()
    }
}

fn append_history(workspace: &str, role: &str, content: &str) {
    let file = history_file_for_workspace(workspace);
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut arr: Vec<serde_json::Value> = if file.exists() {
        serde_json::from_str(&fs::read_to_string(&file).unwrap_or_default()).unwrap_or_default()
    } else {
        vec![]
    };
    arr.push(serde_json::json!({
        "ts": chrono_like_now(),
        "role": role,
        "content": content
    }));
    if arr.len() > 300 {
        arr = arr[arr.len() - 300..].to_vec();
    }
    let _ = fs::write(
        file,
        serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "[]".to_string()),
    );
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

#[tauri::command]
fn get_app_mode(app: AppHandle) -> AppMode {
    AppMode {
        runtime: "tauri".to_string(),
        version: app.package_info().version.to_string(),
        milestone: "M5".to_string(),
    }
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    load_settings(&app)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    save_settings_inner(&app, settings)
}

#[tauri::command]
fn list_recent_workspaces(app: AppHandle) -> Result<Vec<String>, String> {
    let file = app_codexu_dir(&app)?.join("recent_workspaces.json");
    if !file.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(file).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

#[tauri::command]
fn validate_llama_setup(app: AppHandle) -> SetupStatus {
    let settings = load_settings(&app).unwrap_or_default();
    let cfg = build_llm_config(&settings);
    let mut details = vec![format!(
        "provider ativo: {} | modelo: {}",
        settings.provider, settings.model
    )];
    let mut ok = true;

    if settings.provider == "cloud" {
        let p = CloudLlmProvider::new(cfg.clone());
        match p.validate_config() {
            Ok(()) => details.push("cloud config válido".to_string()),
            Err(e) => {
                ok = false;
                details.push(format!("erro cloud: {e}"));
            }
        }
        SetupStatus {
            ok,
            provider: "cloud".to_string(),
            model: settings.model,
            endpoint: settings.cloud_endpoint,
            details,
        }
    } else {
        let p = EndpointLlmProvider::new(cfg.clone());
        match p.validate_config() {
            Ok(()) => details.push("endpoint docker acessível".to_string()),
            Err(e) => {
                ok = false;
                details.push(format!("erro endpoint: {e}"));
            }
        }
        SetupStatus {
            ok,
            provider: "docker".to_string(),
            model: settings.model,
            endpoint: settings.docker_endpoint,
            details,
        }
    }
}

#[tauri::command]
async fn select_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
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
            if !fs::metadata(&path)
                .map_err(|e| format!("falha ao validar workspace: {e}"))?
                .is_dir()
            {
                return Err("workspace selecionado não é um diretório".into());
            }
            {
                let mut guard = state.workspace.lock().map_err(|e| e.to_string())?;
                *guard = Some(path.clone());
            }
            persist_workspace(&app, &path)?;
            Ok(Some(path))
        }
        None => Ok(None),
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
fn save_text_in_workspace(
    state: State<'_, AppState>,
    relative_path: String,
    content: String,
) -> Result<String, String> {
    let workspace = {
        let guard = state.workspace.lock().map_err(|e| e.to_string())?;
        guard.clone()
    }
    .ok_or_else(|| "workspace não selecionado".to_string())?;

    let target = PathBuf::from(workspace).join(relative_path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("falha ao criar pasta: {e}"))?;
    }
    fs::write(&target, content).map_err(|e| format!("falha ao salvar arquivo: {e}"))?;
    Ok(target.display().to_string())
}

#[tauri::command]
fn apply_diff_text(state: State<'_, AppState>, diff_text: String) -> Result<String, String> {
    let workspace = {
        let guard = state.workspace.lock().map_err(|e| e.to_string())?;
        guard.clone()
    }
    .ok_or_else(|| "workspace não selecionado".to_string())?;

    let patch_file = PathBuf::from(&workspace).join(".codexu").join("last.patch");
    if let Some(parent) = patch_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&patch_file, diff_text).map_err(|e| e.to_string())?;

    let out = std::process::Command::new("git")
        .arg("apply")
        .arg(patch_file.display().to_string())
        .current_dir(&workspace)
        .output()
        .map_err(|e| format!("falha ao executar git apply: {e}"))?;

    if out.status.success() {
        Ok("diff aplicado com sucesso".to_string())
    } else {
        Err(format!(
            "falha ao aplicar diff: {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

#[tauri::command]
fn send_chat_message(
    app: AppHandle,
    message: String,
    state: State<'_, AppState>,
) -> Result<ChatResponse, String> {
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

    let settings = load_settings(&app)?;
    let cfg = build_llm_config(&settings);
    let plan_steps = derive_plan_steps(&message);

    let result = if settings.provider == "cloud" {
        CloudLlmProvider::new(cfg).generate_stream(&message)
    } else {
        EndpointLlmProvider::new(cfg).generate_stream(&message)
    };

    match result {
        Ok(chunks) => {
            let answer = chunks.join("\n");
            if let Some(ws) = workspace {
                append_history(&ws, "user", &message);
                append_history(&ws, "assistant", &answer);
            }
            Ok(ChatResponse {
                assistant_message: answer,
                plan_steps,
                diff_text: None,
            })
        }
        Err(e) => Ok(ChatResponse {
            assistant_message: format!(
                "Falha no provider {}: {e}. Verifique configurações em Settings.",
                settings.provider
            ),
            plan_steps,
            diff_text: None,
        }),
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            let state = app.state::<AppState>();
            if let Ok(Some(path)) = load_persisted_workspace(&app.app_handle()) {
                if let Ok(mut guard) = state.workspace.lock() {
                    *guard = Some(path);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_mode,
            get_settings,
            save_settings,
            validate_llama_setup,
            list_recent_workspaces,
            select_workspace,
            get_workspace,
            save_text_in_workspace,
            apply_diff_text,
            send_chat_message
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar app");
}
