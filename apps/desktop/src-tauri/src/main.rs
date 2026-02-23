#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::State;

#[derive(Default)]
struct AppState {
    workspace: Mutex<Option<String>>,
}

#[tauri::command]
fn set_workspace(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_dir() {
        return Err("workspace precisa ser um diretório".into());
    }
    let mut guard = state.workspace.lock().map_err(|e| e.to_string())?;
    *guard = Some(path);
    Ok(())
}

#[tauri::command]
fn get_workspace(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let guard = state.workspace.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![set_workspace, get_workspace])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar app");
}
