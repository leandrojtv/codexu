use std::fs;
use std::path::Path;

fn ensure_placeholder_icon() {
    // 1x1 PNG válido para evitar panic em tauri::generate_context! durante build/test.
    const PNG_BYTES: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 252, 255, 31, 0, 3, 3,
        2, 0, 237, 150, 103, 169, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    let icon_path = Path::new("icons/icon.png");
    if icon_path.exists() {
        return;
    }

    if let Some(parent) = icon_path.parent() {
        fs::create_dir_all(parent).expect("falha ao criar pasta de ícones");
    }

    fs::write(icon_path, PNG_BYTES).expect("falha ao escrever ícone placeholder");
}

fn main() {
    ensure_placeholder_icon();
    tauri_build::build()
}
