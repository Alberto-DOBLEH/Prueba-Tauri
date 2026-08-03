use std::{fs, io::Write, path::PathBuf, process::Command, time::{SystemTime, UNIX_EPOCH}};

use escpos::{
    driver::FileDriver,
    printer::Printer as EscposPrinter,
    utils::{JustifyMode, Protocol},
};
#[cfg(target_os = "windows")]
use printers::{
    common::base::{job::PrinterJobOptions, printer::PrinterState},
    get_default_printer, get_printer_by_name, get_printers,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SystemPrinterInfo {
    name: String,
    system_name: String,
    driver_name: String,
    port_name: String,
    is_default: bool,
    is_shared: bool,
    state: String,
}

#[derive(Deserialize)]
struct SaleItemPayload {
    sku: String,
    name: String,
    quantity: i64,
    unit_price: f64,
    total: f64,
}

#[derive(Deserialize)]
struct SalePayload {
    folio: String,
    created_at: String,
    customer_name: Option<String>,
    subtotal: f64,
    tax: f64,
    total: f64,
    items: Vec<SaleItemPayload>,
}

#[derive(Serialize, Deserialize, Clone)]
struct PrinterTestResult {
    created_at: String,
    test_type: String,
    method: String,
    printer_name: String,
    file_path: String,
    file_size: u64,
    header: Option<String>,
    success: bool,
    job_id: Option<u64>,
    message: String,
    error: Option<String>,
}

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

fn downloads_path(file_name: &str, fallback: &str) -> Result<PathBuf, String> {
    let base = if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .map(|path| path.join("Downloads"))
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .map(|path| path.join("Downloads"))
    }
    .map_err(|_| "No se pudo encontrar la carpeta de descargas.".to_string())?;

    fs::create_dir_all(&base).map_err(|error| error.to_string())?;
    Ok(base.join(safe_file_name(file_name, fallback)))
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn printer_tests_dir() -> Result<PathBuf, String> {
    let mut path = downloads_path("pos-printer-tests", "pos-printer-tests")?;
    path.pop();
    path.push("pos-printer-tests");
    fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    Ok(path)
}

fn printer_test_path(prefix: &str, extension: &str) -> Result<PathBuf, String> {
    let mut path = printer_tests_dir()?;
    path.push(format!("{}-{}.{}", prefix, unix_timestamp(), extension));
    Ok(path)
}

fn append_printer_test_log(result: &PrinterTestResult) -> Result<(), String> {
    let mut path = printer_tests_dir()?;
    path.push("printer-tests.log.jsonl");
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    let line = serde_json::to_string(result).map_err(|error| error.to_string())?;
    writeln!(file, "{line}").map_err(|error| error.to_string())
}

fn file_header(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes[..bytes.len().min(4)]).to_string())
}

fn save_pdf_to_downloads(file_name: &str, bytes: Vec<u8>) -> Result<PathBuf, String> {
    let path = downloads_path(file_name, "documento.pdf")?;
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

fn save_test_pdf(file_name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let path = if file_name.trim().is_empty() {
        printer_test_path("test-pdf", "pdf")?
    } else {
        let mut path = printer_tests_dir()?;
        path.push(safe_file_name(file_name, "test-pdf.pdf"));
        path
    };
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

#[cfg(target_os = "windows")]
fn print_pdf_with_windows_shell(path: &PathBuf, printer_name: Option<&str>) -> Result<(), String> {
    let path_string = path.to_string_lossy().replace('\'', "''");
    let script = if let Some(printer_name) = printer_name.filter(|name| !name.trim().is_empty()) {
        let printer_string = printer_name.replace('\'', "''").replace('"', "`\"");
        format!(
            "Start-Process -FilePath '{}' -Verb PrintTo -ArgumentList '\"{}\"' -WindowStyle Hidden",
            path_string, printer_string
        )
    } else {
        format!(
            "Start-Process -FilePath '{}' -Verb Print -WindowStyle Hidden",
            path_string
        )
    };

    let status = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("Windows no pudo abrir la accion de impresion del PDF. Revisa que exista una app PDF con accion Imprimir/PrintTo.".into())
    }
}

#[cfg(target_os = "windows")]
fn printer_state_label(state: &PrinterState) -> String {
    match state {
        PrinterState::READY => "ready",
        PrinterState::OFFLINE => "offline",
        PrinterState::PAUSED => "paused",
        PrinterState::PRINTING => "printing",
        PrinterState::UNKNOWN => "unknown",
    }
    .to_string()
}

#[cfg(target_os = "windows")]
fn find_printer(printer_name: &str) -> Result<printers::common::base::printer::Printer, String> {
    let trimmed = printer_name.trim();
    if trimmed.is_empty() {
        return get_default_printer().ok_or_else(|| "No hay impresora predeterminada configurada.".to_string());
    }
    get_printer_by_name(trimmed).ok_or_else(|| format!("No se encontro la impresora: {trimmed}"))
}

fn plain_text(value: &str) -> String {
    value.chars().filter(|character| character.is_ascii()).collect()
}

fn money(value: f64) -> String {
    format!("${value:.2}")
}

fn build_test_escpos_file() -> Result<PathBuf, String> {
    let path = printer_test_path("test-ticket", "escpos")?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    let driver = FileDriver::open_with_options(&path, &options).map_err(|error| error.to_string())?;
    let mut printer = EscposPrinter::new(driver, Protocol::default(), None);
    printer
        .init()
        .map_err(|error| error.to_string())?
        .justify(JustifyMode::CENTER)
        .map_err(|error| error.to_string())?
        .bold(true)
        .map_err(|error| error.to_string())?
        .writeln("POS LOCAL")
        .map_err(|error| error.to_string())?
        .bold(false)
        .map_err(|error| error.to_string())?
        .writeln("Prueba ESC/POS")
        .map_err(|error| error.to_string())?
        .justify(JustifyMode::LEFT)
        .map_err(|error| error.to_string())?
        .writeln("------------------------------")
        .map_err(|error| error.to_string())?
        .writeln("Impresion desde Tauri + Rust")
        .map_err(|error| error.to_string())?
        .writeln("Driver: escpos-rs")
        .map_err(|error| error.to_string())?
        .writeln("Spooler: rust-printers")
        .map_err(|error| error.to_string())?
        .writeln("------------------------------")
        .map_err(|error| error.to_string())?
        .justify(JustifyMode::CENTER)
        .map_err(|error| error.to_string())?
        .writeln("OK")
        .map_err(|error| error.to_string())?
        .feeds(3)
        .map_err(|error| error.to_string())?
        .print_cut()
        .map_err(|error| error.to_string())?;
    Ok(path)
}

fn build_sale_escpos_file(sale: SalePayload) -> Result<PathBuf, String> {
    let path = temp_print_path("ticket-venta.escpos", "ticket-venta.escpos");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    let driver = FileDriver::open_with_options(&path, &options).map_err(|error| error.to_string())?;
    let mut printer = EscposPrinter::new(driver, Protocol::default(), None);

    printer
        .init()
        .map_err(|error| error.to_string())?
        .justify(JustifyMode::CENTER)
        .map_err(|error| error.to_string())?
        .bold(true)
        .map_err(|error| error.to_string())?
        .writeln("POS LOCAL")
        .map_err(|error| error.to_string())?
        .bold(false)
        .map_err(|error| error.to_string())?
        .writeln("Ticket de venta")
        .map_err(|error| error.to_string())?
        .justify(JustifyMode::LEFT)
        .map_err(|error| error.to_string())?
        .writeln("------------------------------")
        .map_err(|error| error.to_string())?
        .writeln(&format!("Folio: {}", plain_text(&sale.folio)))
        .map_err(|error| error.to_string())?
        .writeln(&format!("Fecha: {}", plain_text(&sale.created_at)))
        .map_err(|error| error.to_string())?
        .writeln(&format!("Cliente: {}", plain_text(sale.customer_name.as_deref().unwrap_or("Mostrador"))))
        .map_err(|error| error.to_string())?
        .writeln("------------------------------")
        .map_err(|error| error.to_string())?;

    for item in sale.items {
        printer
            .writeln(&plain_text(&item.name).chars().take(28).collect::<String>())
            .map_err(|error| error.to_string())?
            .writeln(&format!("{} {} x {} = {}", plain_text(&item.sku), item.quantity, money(item.unit_price), money(item.total)))
            .map_err(|error| error.to_string())?;
    }

    printer
        .writeln("------------------------------")
        .map_err(|error| error.to_string())?
        .writeln(&format!("Subtotal: {}", money(sale.subtotal)))
        .map_err(|error| error.to_string())?
        .writeln(&format!("IVA:      {}", money(sale.tax)))
        .map_err(|error| error.to_string())?
        .bold(true)
        .map_err(|error| error.to_string())?
        .writeln(&format!("TOTAL:    {}", money(sale.total)))
        .map_err(|error| error.to_string())?
        .bold(false)
        .map_err(|error| error.to_string())?
        .writeln("------------------------------")
        .map_err(|error| error.to_string())?
        .justify(JustifyMode::CENTER)
        .map_err(|error| error.to_string())?
        .writeln("Gracias por su compra")
        .map_err(|error| error.to_string())?
        .feeds(3)
        .map_err(|error| error.to_string())?
        .print_cut()
        .map_err(|error| error.to_string())?;

    Ok(path)
}

#[cfg(target_os = "windows")]
fn print_raw_file(printer_name: &str, path: &PathBuf, job_name: &str) -> Result<u64, String> {
    let printer = find_printer(printer_name)?;
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    printer
        .print(
            &bytes,
            PrinterJobOptions {
                name: Some(job_name),
                raw_properties: &[("document-format", "RAW")],
                converter: printers::common::converters::Converter::None,
            },
        )
        .map_err(|error| format!("{error:?}"))
}

#[cfg(not(target_os = "windows"))]
fn print_raw_file(_printer_name: &str, _path: &PathBuf, _job_name: &str) -> Result<u64, String> {
    Err("La impresion nativa con rust-printers esta habilitada solo en Windows en este build.".into())
}

#[tauri::command]
fn list_system_printers() -> Vec<SystemPrinterInfo> {
    #[cfg(target_os = "windows")]
    {
    get_printers()
        .into_iter()
        .map(|printer| SystemPrinterInfo {
            name: printer.name,
            system_name: printer.system_name,
            driver_name: printer.driver_name,
            port_name: printer.port_name,
            is_default: printer.is_default,
            is_shared: printer.is_shared,
            state: printer_state_label(&printer.state),
        })
        .collect()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

#[tauri::command]
fn get_default_system_printer() -> Option<SystemPrinterInfo> {
    #[cfg(target_os = "windows")]
    {
    get_default_printer().map(|printer| SystemPrinterInfo {
        name: printer.name,
        system_name: printer.system_name,
        driver_name: printer.driver_name,
        port_name: printer.port_name,
        is_default: printer.is_default,
        is_shared: printer.is_shared,
        state: printer_state_label(&printer.state),
    })
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[tauri::command]
fn save_pdf_downloads(file_name: String, bytes: Vec<u8>) -> Result<String, String> {
    save_pdf_to_downloads(&file_name, bytes).map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn print_pdf_windows(file_name: String, bytes: Vec<u8>) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let path = save_pdf_to_downloads(&file_name, bytes)?;
        print_pdf_with_windows_shell(&path, None)?;
        Ok(path.to_string_lossy().to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (file_name, bytes);
        Err("La impresion automatica de PDF solo esta implementada para Windows.".into())
    }
}

#[tauri::command]
fn print_pdf_to_printer(printer_name: String, file_name: String, bytes: Vec<u8>) -> Result<u64, String> {
    #[cfg(target_os = "windows")]
    {
    let path = save_pdf_to_downloads(&file_name, bytes)?;
    let printer = find_printer(&printer_name)?;
    match printer
        .print_file(
            &path.to_string_lossy(),
            PrinterJobOptions {
                name: Some("POS Local PDF"),
                raw_properties: &[("document-format", "application/pdf")],
                converter: printers::common::converters::Converter::None,
            },
        )
        .map_err(|error| format!("{error:?}"))
    {
        Ok(job_id) => Ok(job_id),
        Err(_) => {
            print_pdf_with_windows_shell(&path, Some(&printer_name))?;
            Ok(0)
        }
    }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (printer_name, file_name, bytes);
        Err("La impresion PDF con rust-printers esta habilitada solo en Windows en este build.".into())
    }
}

#[tauri::command]
fn print_test_pdf_to_printer(printer_name: String, file_name: String, bytes: Vec<u8>) -> PrinterTestResult {
    let created_at = unix_timestamp().to_string();
    let method = "jsPDF -> archivo PDF -> rust-printers PDF o Windows Shell PrintTo".to_string();
    let header = file_header(&bytes);
    let saved_path = save_test_pdf(&file_name, &bytes);

    let mut result = match saved_path {
        Ok(path) => {
            let file_size = fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or_default();
            #[cfg(target_os = "windows")]
            let print_result = find_printer(&printer_name).and_then(|printer| {
                printer
                    .print_file(
                        &path.to_string_lossy(),
                        PrinterJobOptions {
                            name: Some("POS Local PDF Test"),
                            raw_properties: &[("document-format", "application/pdf")],
                            converter: printers::common::converters::Converter::None,
                        },
                    )
                    .map_err(|error| format!("{error:?}"))
            });

            #[cfg(not(target_os = "windows"))]
            let print_result: Result<u64, String> = Err("rust-printers para PDF esta habilitado solo en Windows en este build.".into());

            match print_result {
                Ok(job_id) => PrinterTestResult {
                    created_at: created_at.clone(),
                    test_type: "pdf".into(),
                    method: method.clone(),
                    printer_name: printer_name.clone(),
                    file_path: path.to_string_lossy().to_string(),
                    file_size,
                    header: header.clone(),
                    success: true,
                    job_id: Some(job_id),
                    message: "Trabajo PDF enviado al spooler. Valida fisicamente si la impresora lo proceso.".into(),
                    error: None,
                },
                Err(error) => {
                    #[cfg(target_os = "windows")]
                    let shell_result = print_pdf_with_windows_shell(&path, Some(&printer_name));

                    #[cfg(not(target_os = "windows"))]
                    let shell_result: Result<(), String> = Err("Windows Shell PrintTo esta disponible solo en Windows.".into());

                    match shell_result {
                        Ok(()) => PrinterTestResult {
                            created_at: created_at.clone(),
                            test_type: "pdf".into(),
                            method: method.clone(),
                            printer_name: printer_name.clone(),
                            file_path: path.to_string_lossy().to_string(),
                            file_size,
                            header: header.clone(),
                            success: true,
                            job_id: None,
                            message: "El spooler no acepto PDF crudo; Windows recibio la orden PrintTo mediante la app PDF asociada. Valida fisicamente si salio el documento.".into(),
                            error: Some(format!("Fallo directo: {error}")),
                        },
                        Err(shell_error) => PrinterTestResult {
                            created_at: created_at.clone(),
                            test_type: "pdf".into(),
                            method: method.clone(),
                            printer_name: printer_name.clone(),
                            file_path: path.to_string_lossy().to_string(),
                            file_size,
                            header: header.clone(),
                            success: false,
                            job_id: None,
                            message: "No se pudo enviar el PDF directo al spooler ni por Windows Shell PrintTo.".into(),
                            error: Some(format!("Directo: {error} | Shell: {shell_error}")),
                        },
                    }
                },
            }
        }
        Err(error) => PrinterTestResult {
            created_at,
            test_type: "pdf".into(),
            method,
            printer_name,
            file_path: String::new(),
            file_size: bytes.len() as u64,
            header,
            success: false,
            job_id: None,
            message: "No se pudo guardar el PDF de prueba.".into(),
            error: Some(error),
        },
    };

    if let Err(error) = append_printer_test_log(&result) {
        result.message = format!("{} Ademas fallo guardar log: {error}", result.message);
    }
    result
}

#[tauri::command]
fn print_test_escpos(printer_name: String) -> PrinterTestResult {
    let created_at = unix_timestamp().to_string();
    let method = "escpos-rs -> archivo .escpos -> rust-printers RAW -> WinSpool".to_string();
    let mut result = match build_test_escpos_file() {
        Ok(path) => {
            let file_size = fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or_default();
            match print_raw_file(&printer_name, &path, "POS Local ESC/POS Test") {
                Ok(job_id) => PrinterTestResult {
                    created_at: created_at.clone(),
                    test_type: "escpos".into(),
                    method: method.clone(),
                    printer_name: printer_name.clone(),
                    file_path: path.to_string_lossy().to_string(),
                    file_size,
                    header: None,
                    success: true,
                    job_id: Some(job_id),
                    message: "Trabajo ESC/POS enviado como RAW. Valida fisicamente si salio el ticket.".into(),
                    error: None,
                },
                Err(error) => PrinterTestResult {
                    created_at: created_at.clone(),
                    test_type: "escpos".into(),
                    method: method.clone(),
                    printer_name: printer_name.clone(),
                    file_path: path.to_string_lossy().to_string(),
                    file_size,
                    header: None,
                    success: false,
                    job_id: None,
                    message: "No se pudo enviar el ticket ESC/POS al spooler.".into(),
                    error: Some(error),
                },
            }
        }
        Err(error) => PrinterTestResult {
            created_at,
            test_type: "escpos".into(),
            method,
            printer_name,
            file_path: String::new(),
            file_size: 0,
            header: None,
            success: false,
            job_id: None,
            message: "No se pudo generar el archivo ESC/POS de prueba.".into(),
            error: Some(error),
        },
    };

    if let Err(error) = append_printer_test_log(&result) {
        result.message = format!("{} Ademas fallo guardar log: {error}", result.message);
    }
    result
}

#[tauri::command]
fn get_printer_test_logs() -> Result<Vec<PrinterTestResult>, String> {
    let mut path = printer_tests_dir()?;
    path.push("printer-tests.log.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    Ok(content
        .lines()
        .filter_map(|line| serde_json::from_str::<PrinterTestResult>(line).ok())
        .collect())
}

#[tauri::command]
fn open_printer_test_folder() -> Result<String, String> {
    let path = printer_tests_dir()?;
    #[cfg(target_os = "windows")]
    let status = Command::new("explorer").arg(&path).status();
    #[cfg(target_os = "linux")]
    let status = Command::new("xdg-open").arg(&path).status();
    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg(&path).status();

    status.map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn print_sale_escpos(printer_name: String, sale: SalePayload) -> Result<u64, String> {
    let path = build_sale_escpos_file(sale)?;
    print_raw_file(&printer_name, &path, "POS Local Ticket")
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
        .invoke_handler(tauri::generate_handler![
            list_system_printers,
            get_default_system_printer,
            save_pdf_downloads,
            print_pdf_windows,
            print_pdf_to_printer,
            print_test_pdf_to_printer,
            print_escpos_windows,
            print_test_escpos,
            get_printer_test_logs,
            open_printer_test_folder,
            print_sale_escpos
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
