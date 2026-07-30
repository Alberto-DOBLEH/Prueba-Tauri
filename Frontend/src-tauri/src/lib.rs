use std::{fs, path::PathBuf, process::Command};

fn safe_file_name(file_name: &str, fallback: &str) -> String {
    let cleaned: String = file_name
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => character,
            _ => '_',
        })
        .collect();

    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn temp_print_path(file_name: &str, fallback: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("pos-local-print");
    let _ = fs::create_dir_all(&path);
    path.push(safe_file_name(file_name, fallback));
    path
}

#[tauri::command]
fn print_pdf_windows(file_name: String, bytes: Vec<u8>) -> Result<String, String> {
    if !cfg!(target_os = "windows") {
        return Err("La impresion automatica de PDF solo esta implementada para Windows.".into());
    }

    let path = temp_print_path(&file_name, "documento.pdf");
    fs::write(&path, bytes).map_err(|error| error.to_string())?;

    let path_string = path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "Start-Process -FilePath '{}' -Verb Print -WindowStyle Hidden",
        path_string
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(path.to_string_lossy().to_string())
    } else {
        Err("Windows no pudo enviar el PDF a imprimir. Revisa que exista una app PDF con accion Imprimir.".into())
    }
}

#[tauri::command]
fn print_escpos_windows(printer_share: String, bytes: Vec<u8>) -> Result<String, String> {
    if !cfg!(target_os = "windows") {
        return Err("La impresion ESC/POS directa solo esta implementada para Windows.".into());
    }

    let printer = printer_share.trim();
    if printer.is_empty() {
        return Err("Configura el nombre compartido de la impresora termica.".into());
    }

    let path = temp_print_path("ticket.escpos", "ticket.escpos");
    fs::write(&path, bytes).map_err(|error| error.to_string())?;

    let target = if printer.starts_with("\\\\") {
        printer.to_string()
    } else {
        format!("\\\\localhost\\{}", printer)
    };

    let command = format!(
        "copy /B \"{}\" \"{}\"",
        path.to_string_lossy(),
        target
    );
    let status = Command::new("cmd")
        .args(["/C", &command])
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(target)
    } else {
        Err("No se pudo enviar el ticket ESC/POS. Revisa el nombre compartido de la impresora.".into())
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![print_pdf_windows, print_escpos_windows])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
