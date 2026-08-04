use std::{fs, io::Write, path::PathBuf, process::Command, time::{SystemTime, UNIX_EPOCH}};

#[cfg(target_os = "windows")]
use printers::{
    common::base::{job::PrinterJobOptions, printer::PrinterState},
    get_default_printer, get_printer_by_name, get_printers,
};
use serde::{Deserialize, Serialize};
use tauri::Manager;

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrinterLogPayload {
    test_type: String,
    method: String,
    printer_name: Option<String>,
    file_path: Option<String>,
    file_size: Option<u64>,
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

fn record_printer_log(payload: PrinterLogPayload) -> Result<PrinterTestResult, String> {
    let result = PrinterTestResult {
        created_at: unix_timestamp().to_string(),
        test_type: payload.test_type,
        method: payload.method,
        printer_name: payload.printer_name.unwrap_or_default(),
        file_path: payload.file_path.unwrap_or_default(),
        file_size: payload.file_size.unwrap_or_default(),
        header: payload.header,
        success: payload.success,
        job_id: payload.job_id,
        message: payload.message,
        error: payload.error,
    };
    append_printer_test_log(&result)?;
    Ok(result)
}

fn file_header(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes[..bytes.len().min(4)]).to_string())
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;

    for byte in input.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        let value = base64_value(byte).ok_or_else(|| format!("Caracter Base64 invalido: {}", byte as char))?;
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
    }

    Ok(output)
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

fn printer_result(
    test_type: &str,
    method: &str,
    printer_name: &str,
    file_path: &str,
    file_size: u64,
    header: Option<String>,
    success: bool,
    message: &str,
    error: Option<String>,
) -> PrinterTestResult {
    PrinterTestResult {
        created_at: unix_timestamp().to_string(),
        test_type: test_type.into(),
        method: method.into(),
        printer_name: printer_name.into(),
        file_path: file_path.into(),
        file_size,
        header,
        success,
        job_id: None,
        message: message.into(),
        error,
    }
}

fn append_and_return_log(mut result: PrinterTestResult) -> PrinterTestResult {
    if let Err(error) = append_printer_test_log(&result) {
        result.success = false;
        result.message = format!("{} Ademas fallo guardar log: {error}", result.message);
        result.error = Some(match result.error {
            Some(existing) => format!("{existing} | Log: {error}"),
            None => error,
        });
    }
    result
}

fn sumatra_pdf_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("resources").join("sumatrapdf").join("SumatraPDF.exe"));
        candidates.push(resource_dir.join("resources").join("sumatrapdf").join("SumatraPDF-3.6.1-64.exe"));
        candidates.push(resource_dir.join("sumatrapdf").join("SumatraPDF.exe"));
        candidates.push(resource_dir.join("sumatrapdf").join("SumatraPDF-3.6.1-64.exe"));
        candidates.push(resource_dir.join("SumatraPDF.exe"));
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(app_dir) = current_exe.parent() {
            candidates.push(app_dir.join("resources").join("sumatrapdf").join("SumatraPDF.exe"));
            candidates.push(app_dir.join("resources").join("sumatrapdf").join("SumatraPDF-3.6.1-64.exe"));
            candidates.push(app_dir.join("sumatrapdf").join("SumatraPDF.exe"));
            candidates.push(app_dir.join("sumatrapdf").join("SumatraPDF-3.6.1-64.exe"));
        }
    }

    candidates
}

fn find_sumatra_pdf_in_dir(dir: &PathBuf) -> Option<PathBuf> {
    fs::read_dir(dir).ok()?.filter_map(|entry| entry.ok()).map(|entry| entry.path()).find(|path| {
        path.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| {
                    let lower = name.to_ascii_lowercase();
                    lower.starts_with("sumatrapdf") && lower.ends_with(".exe")
                })
                .unwrap_or(false)
    })
}

fn find_sumatra_pdf(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let candidates = sumatra_pdf_candidates(app);
    if let Some(path) = candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
    {
        return Ok(path);
    }

    for candidate in &candidates {
        if let Some(dir) = candidate.parent().map(PathBuf::from) {
            if let Some(path) = find_sumatra_pdf_in_dir(&dir) {
                return Ok(path);
            }
        }
    }

    let checked = candidates
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(" | ");
    Err(format!("No se encontro SumatraPDF portable empaquetado. Rutas revisadas: {checked}"))
}

#[cfg(target_os = "windows")]
fn escape_powershell_single(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(target_os = "windows")]
fn windows_print_jobs(printer_name: &str) -> Result<String, String> {
    let escaped_printer = escape_powershell_single(printer_name);
    let script = format!(
        "Get-PrintJob -PrinterName '{}' | Select-Object -First 5 ID,Name,JobStatus,SubmittedTime | ConvertTo-Json -Compress",
        escaped_printer
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()
        .map_err(|error| error.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() { "Sin trabajos activos detectados".into() } else { stdout })
    } else {
        Err(if stderr.is_empty() { "Get-PrintJob fallo sin detalle".into() } else { stderr })
    }
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
    format!("{value:.2}")
}

fn fit_text(value: &str, width: usize) -> String {
    plain_text(value).chars().take(width).collect()
}

fn pad_right(value: &str, width: usize) -> String {
    let fitted = fit_text(value, width);
    format!("{fitted:<width$}")
}

fn pad_left(value: &str, width: usize) -> String {
    let fitted = fit_text(value, width);
    format!("{fitted:>width$}")
}

fn push_escpos_text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(plain_text(value).as_bytes());
    bytes.push(b'\n');
}

fn sale_date_time(created_at: &str) -> (String, String) {
    let normalized = created_at.replace('T', " ");
    let mut parts = normalized.split_whitespace();
    let date = parts.next().unwrap_or(created_at).to_string();
    let time = parts.next().unwrap_or("00:00:00").chars().take(8).collect();
    (date, time)
}

fn build_test_escpos_file() -> Result<PathBuf, String> {
    let path = printer_test_path("test-ticket-tk-raw", "escpos")?;
    let bytes = decode_base64(include_str!("../resources/escpos/tk-raw.txt"))?;
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

fn build_sale_escpos_file(sale: SalePayload) -> Result<PathBuf, String> {
    let path = temp_print_path("ticket-venta.escpos", "ticket-venta.escpos");
    let mut bytes = Vec::new();
    let (date, time) = sale_date_time(&sale.created_at);
    let _ = (&sale.customer_name, sale.subtotal, sale.tax);

    bytes.extend_from_slice(&[0x1b, 0x40, 0x1b, 0x4d, 0x00, 0x1b, 0x61, 0x01, 0x1d, 0x21, 0x00]);
    push_escpos_text(&mut bytes, "FERRETERIA MALOVA, S.A. DE C.V.");
    push_escpos_text(&mut bytes, "SUC NOVIEMBRE");
    push_escpos_text(&mut bytes, "DONDE USTED COMPRA DE CORAZON");
    push_escpos_text(&mut bytes, "BLVD CENTENARIO 2104");
    push_escpos_text(&mut bytes, "LOS ANGELES");
    push_escpos_text(&mut bytes, "R.F.C. FMA850606P44");
    push_escpos_text(&mut bytes, "TEL: (668)2392358");
    push_escpos_text(&mut bytes, &format!("FECHA: {}  T:{}", date, fit_text(&sale.folio, 9)));
    push_escpos_text(&mut bytes, &format!("HORA: {}  VEND: 08", time));
    bytes.extend_from_slice(&[0x1b, 0x64, 0x02, 0x1d, 0x21, 0x00]);
    push_escpos_text(&mut bytes, "COD  CANT  ART  PRECIO  IMPORTE");
    push_escpos_text(&mut bytes, "---------------------------------");

    for item in sale.items {
        let line = format!(
            "{} {} {} {} {}",
            pad_right(&item.sku, 5),
            pad_left(&format!("{:.2}", item.quantity as f64), 5),
            pad_right("PZA", 5),
            pad_left(&money(item.unit_price), 5),
            pad_left(&money(item.total), 7)
        );
        push_escpos_text(&mut bytes, &line);
        push_escpos_text(&mut bytes, &format!("{} ", fit_text(&item.name, 32)));
    }

    push_escpos_text(&mut bytes, "---------------------------------");
    bytes.extend_from_slice(&[0x1b, 0x61, 0x02, 0x1b, 0x21, 0x08]);
    push_escpos_text(&mut bytes, &format!("T O T A L : {}", money(sale.total)));
    bytes.extend_from_slice(&[0x1b, 0x21, 0x00, 0x1d, 0x21, 0x00]);
    push_escpos_text(&mut bytes, "");
    bytes.extend_from_slice(&[0x1b, 0x21, 0x01, 0x1d, 0x21, 0x00, 0x1b, 0x61, 0x01]);
    push_escpos_text(&mut bytes, &format!("*** IDWEB PARA FACTURAR {} ***", fit_text(&sale.folio, 12)));
    bytes.extend_from_slice(&[0x1b, 0x21, 0x08]);
    push_escpos_text(&mut bytes, "FACTURA EN");
    push_escpos_text(&mut bytes, "WWW.FERRETERIAMALOVA.COM.MX/FACTURACION");
    bytes.extend_from_slice(&[0x1b, 0x21, 0x00, 0x1b, 0x21, 0x01]);
    push_escpos_text(&mut bytes, "***GRACIAS POR SU PREFERENCIA***");
    push_escpos_text(&mut bytes, "TODA DEVOLUCION CAUSARA 20% DE");
    push_escpos_text(&mut bytes, "CARGO Y NO SE ACEPTA DESPUES");
    push_escpos_text(&mut bytes, "DE 8 DIAS");
    bytes.extend_from_slice(&[0x1b, 0x21, 0x00]);
    push_escpos_text(&mut bytes, "---------------------------------");
    bytes.extend_from_slice(&[0x1d, 0x56, 0x41, 0x03]);

    fs::write(&path, bytes).map_err(|error| error.to_string())?;

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
fn print_pdf_sumatra(
    app: tauri::AppHandle,
    printer_name: String,
    file_name: String,
    bytes: Vec<u8>,
    test_type: Option<String>,
) -> PrinterTestResult {
    let method = "jsPDF -> archivo PDF -> SumatraPDF portable -print-to -silent";
    let test_type = test_type.unwrap_or_else(|| "pdf-sumatra".into());
    let header = file_header(&bytes);
    let file_size = bytes.len() as u64;

    let path = match save_pdf_to_downloads(&file_name, bytes) {
        Ok(path) => path,
        Err(error) => {
            return append_and_return_log(printer_result(
                &test_type,
                method,
                printer_name.trim(),
                "",
                file_size,
                header,
                false,
                "No se pudo guardar el PDF antes de imprimir con SumatraPDF.",
                Some(error),
            ));
        }
    };

    let file_path = path.to_string_lossy().to_string();
    if !cfg!(target_os = "windows") {
        return append_and_return_log(printer_result(
            &test_type,
            method,
            printer_name.trim(),
            &file_path,
            file_size,
            header,
            false,
            "La impresion silenciosa con SumatraPDF solo esta disponible en Windows.",
            None,
        ));
    }

    let sumatra_path = match find_sumatra_pdf(&app) {
        Ok(path) => path,
        Err(error) => {
            return append_and_return_log(printer_result(
                &test_type,
                method,
                printer_name.trim(),
                &file_path,
                file_size,
                header,
                false,
                "No se pudo encontrar SumatraPDF portable empaquetado.",
                Some(error),
            ));
        }
    };

    let trimmed_printer = printer_name.trim();
    #[cfg(target_os = "windows")]
    let mut selected_printer = if trimmed_printer.is_empty() { "Predeterminada".to_string() } else { trimmed_printer.to_string() };
    #[cfg(not(target_os = "windows"))]
    let selected_printer = if trimmed_printer.is_empty() { "Predeterminada".to_string() } else { trimmed_printer.to_string() };

    #[cfg(target_os = "windows")]
    {
        if trimmed_printer.is_empty() {
            match get_default_printer() {
                Some(printer) => selected_printer = printer.system_name,
                None => {
                    return append_and_return_log(printer_result(
                        &test_type,
                        method,
                        "Predeterminada",
                        &file_path,
                        file_size,
                        header,
                        false,
                        "No hay impresora predeterminada para usar con SumatraPDF.",
                        None,
                    ));
                }
            }
        } else if let Err(error) = find_printer(trimmed_printer) {
            return append_and_return_log(printer_result(
                &test_type,
                method,
                trimmed_printer,
                &file_path,
                file_size,
                header,
                false,
                "La impresora seleccionada para SumatraPDF no existe en Windows.",
                Some(error),
            ));
        }
    }

    let mut args = vec!["-silent".to_string(), "-exit-when-done".to_string(), "-print-settings".to_string(), "fit".to_string()];
    if trimmed_printer.is_empty() {
        args.push("-print-to-default".to_string());
    } else {
        args.push("-print-to".to_string());
        args.push(trimmed_printer.to_string());
    }
    args.push(file_path.clone());

    let command_line = format!("{} {}", sumatra_path.to_string_lossy(), args.join(" "));
    let mut command = Command::new(&sumatra_path);
    command.args(&args);

    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            return append_and_return_log(printer_result(
                &test_type,
                method,
                &selected_printer,
                &file_path,
                file_size,
                header,
                false,
                "No se pudo ejecutar SumatraPDF portable.",
                Some(format!("{} | Comando: {}", error, command_line)),
            ));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let exit_code = output.status.code().map_or("sin_codigo".into(), |code| code.to_string());

    #[cfg(target_os = "windows")]
    let queue_status = windows_print_jobs(&selected_printer);

    #[cfg(not(target_os = "windows"))]
    let queue_status: Result<String, String> = Err("Consulta de cola disponible solo en Windows".into());

    let queue_message = match &queue_status {
        Ok(status) => format!("Cola Windows: {status}"),
        Err(error) => format!("No se pudo consultar cola Windows: {error}"),
    };

    if output.status.success() {
        append_and_return_log(printer_result(
            &test_type,
            method,
            &selected_printer,
            &file_path,
            file_size,
            header,
            true,
            &format!(
                "SumatraPDF termino OK. Exit code: {exit_code}. {queue_message}. Comando: {command_line}"
            ),
            if stdout.is_empty() && stderr.is_empty() {
                None
            } else {
                Some(format!("stdout: {stdout} | stderr: {stderr}"))
            },
        ))
    } else {
        append_and_return_log(printer_result(
            &test_type,
            method,
            &selected_printer,
            &file_path,
            file_size,
            header,
            false,
            &format!("SumatraPDF termino con error. Exit code: {exit_code}. {queue_message}."),
            Some(format!(
                "Comando: {command_line} | stdout: {stdout} | stderr: {stderr}"
            )),
        ))
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
    let method = "tk-raw.txt Base64 -> archivo .escpos -> rust-printers RAW -> WinSpool".to_string();
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
                    message: "Ticket tk-raw ESC/POS enviado como RAW. Valida fisicamente si salio el formato Malova.".into(),
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
                    message: "No se pudo enviar el ticket tk-raw ESC/POS al spooler.".into(),
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
            message: "No se pudo decodificar/generar el archivo ESC/POS desde tk-raw.txt.".into(),
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
fn add_printer_test_log(payload: PrinterLogPayload) -> Result<PrinterTestResult, String> {
    record_printer_log(payload)
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
            print_pdf_sumatra,
            print_test_pdf_to_printer,
            print_escpos_windows,
            print_test_escpos,
            get_printer_test_logs,
            add_printer_test_log,
            open_printer_test_folder,
            print_sale_escpos
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
