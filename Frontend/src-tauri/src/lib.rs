use std::{fs, path::PathBuf, process::Command};

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

fn save_pdf_to_downloads(file_name: &str, bytes: Vec<u8>) -> Result<PathBuf, String> {
    let path = downloads_path(file_name, "documento.pdf")?;
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
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
    let path = temp_print_path("ticket-prueba.escpos", "ticket-prueba.escpos");
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
    if !cfg!(target_os = "windows") {
        return Err("La impresion automatica de PDF solo esta implementada para Windows.".into());
    }

    let path = save_pdf_to_downloads(&file_name, bytes)?;

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
fn print_pdf_to_printer(printer_name: String, file_name: String, bytes: Vec<u8>) -> Result<u64, String> {
    #[cfg(target_os = "windows")]
    {
    let path = save_pdf_to_downloads(&file_name, bytes)?;
    let printer = find_printer(&printer_name)?;
    printer
        .print_file(
            &path.to_string_lossy(),
            PrinterJobOptions {
                name: Some("POS Local PDF"),
                raw_properties: &[("document-format", "application/pdf")],
                converter: printers::common::converters::Converter::None,
            },
        )
        .map_err(|error| format!("{error:?}"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (printer_name, file_name, bytes);
        Err("La impresion PDF con rust-printers esta habilitada solo en Windows en este build.".into())
    }
}

#[tauri::command]
fn print_test_escpos(printer_name: String) -> Result<u64, String> {
    let path = build_test_escpos_file()?;
    print_raw_file(&printer_name, &path, "POS Local ESC/POS Test")
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
            print_escpos_windows,
            print_test_escpos,
            print_sale_escpos
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
