# Contexto Para Agentes

Este repositorio es una prueba funcional de punto de venta local para Ferreteria Malova. El objetivo principal es validar Tauri en Windows, SQLite local, backend Express y flujos de impresion reales con impresoras termicas ESC/POS y documentos tipo PDF/reporte.

Este documento debe tratarse como la referencia rapida y actualizada para agentes. Algunos README historicos pueden estar desfasados, especialmente en impresion PDF y versionado.

## Estructura

- `Backend/`: API REST en Express, persistencia con `better-sqlite3`.
- `Frontend/`: React + Vite, empaquetado con Tauri.
- `Frontend/src-tauri/`: comandos nativos Rust/Tauri, deteccion e impresion Windows.
- `BD/`: base SQLite local `pos.sqlite` y archivos WAL/SHM.
- `README.md`, `WINDOWS.md`, `Frontend/README.md`: documentacion previa util, pero verificar contra el codigo actual.
- `AGENTS.md`: este contexto.

## Estado Actual

- Version actual de app: `1.0.12`.
- Rama principal usada: `main`.
- Remoto: `origin git@github.com:Alberto-DOBLEH/Prueba-Tauri.git`.
- La app Tauri instalada todavia espera que el backend este corriendo en `http://localhost:3001`.
- El backend no esta empaquetado como sidecar ni servicio todavia.
- No se deben instalar apps externas en las computadoras finales; si se necesita algo adicional, debe ir empaquetado dentro del instalador Tauri.

## Stack

- Frontend: React 19, Vite, jsPDF, Tauri API.
- Desktop: Tauri 2.
- Backend: Express 4, CORS, Morgan.
- DB: SQLite via `better-sqlite3`.
- Rust/Tauri: `printers` para spooler Windows, `escpos` para generacion de pruebas ESC/POS.

## Comandos

Backend:

```bash
cd Backend
npm install
npm run seed
npm run dev
```

Frontend web/Tauri dev:

```bash
cd Frontend
npm install
npm run dev
npm run tauri:dev
```

Build frontend:

```bash
cd Frontend
npm run build
```

Verificacion Rust:

```bash
cd Frontend/src-tauri
cargo check
```

Build Windows desde VM Windows:

```powershell
cd Frontend
npm run tauri:build:windows
```

Solo EXE NSIS:

```powershell
cd Frontend
npm run tauri:build:windows:exe
```

Solo MSI:

```powershell
cd Frontend
npm run tauri:build:windows:msi
```

Build limpio Windows:

```powershell
cd Frontend
npm run tauri:build:windows:clean
```

## Backend

Archivo principal: `Backend/src/server.js`.

Puerto por defecto: `3001`.

Endpoints principales:

- `GET /api/health`: salud del backend.
- `GET /api/products`: productos, acepta `search` y `available=true`.
- `POST /api/products`: crea producto.
- `PATCH /api/products/:id/stock`: movimientos `add`, `remove`, `void`.
- `POST /api/sales`: crea venta, descuenta stock, crea movimientos.
- `GET /api/sales`: lista ventas.
- `GET /api/sales/:id`: venta con partidas.
- `POST /api/quotes`: crea cotizacion, no descuenta stock.
- `GET /api/quotes`: lista cotizaciones.
- `GET /api/quotes/:id`: cotizacion con partidas.
- `GET /api/dashboard`: resumen administrativo.
- `GET /api/reports/sales`: datos para reporte ventas.
- `GET /api/reports/inventory`: datos para reporte inventario.

La logica de folios, schema y helpers de dinero esta en `Backend/src/db.js`.

## Base De Datos

Base principal: `BD/pos.sqlite`.

Tablas importantes:

- `products`: catalogo e inventario actual.
- `sales`: cabeceras de venta.
- `sale_items`: partidas congeladas de venta.
- `quotes`: cabeceras de cotizacion.
- `quote_items`: partidas congeladas de cotizacion.
- `stock_movements`: bitacora de inventario.

No borrar `BD/pos.sqlite` salvo que el usuario pida reiniciar datos. `npm run seed` puede recrear o poblar datos segun la implementacion actual.

## Frontend

Archivo principal: `Frontend/src/main.jsx`.

Pantallas:

- `Venta`: busqueda de productos, carrito, cobrar venta, generar cotizacion.
- `Inventario`: alta de productos, aumentar/quitar/anular stock.
- `Administracion`: dashboard, reportes, historiales y reimpresion.
- `Impresoras`: deteccion, pruebas, logs, carpeta de evidencias.

Configuracion de impresion guardada en `localStorage` con clave `pos-print-settings`.

## Tauri

Config: `Frontend/src-tauri/tauri.conf.json`.

Puntos clave:

- `productName`: `POS Local`.
- `version`: actualmente `1.0.12`.
- `identifier`: `com.pruebas.poslocal`.
- En dev usa `http://127.0.0.1:5173`.
- En build usa `Frontend/dist`.
- Bundle Windows genera `nsis` y `msi`.
- NSIS instala por usuario: `installMode: currentUser`.

Comandos Tauri/Rust importantes en `Frontend/src-tauri/src/lib.rs`:

- `list_system_printers`: lista impresoras Windows.
- `get_default_system_printer`: obtiene predeterminada.
- `save_pdf_downloads`: guarda PDF en Descargas.
- `print_pdf_windows`: intento legacy con Shell `Print`; no confiar para flujo final sin app PDF.
- `print_pdf_to_printer`: intento legacy directo al spooler para PDF; solo sirve si driver acepta PDF nativo.
- `print_test_pdf_to_printer`: prueba legacy PDF/spooler con fallback Shell.
- `print_test_escpos`: prueba ESC/POS nativa usando `resources/escpos/tk-raw.txt` decodificado desde Base64.
- `print_sale_escpos`: imprime venta como ESC/POS RAW por `printers`.
- `print_escpos_windows`: respaldo por impresora compartida con `copy /B`.
- `get_printer_test_logs`: lee logs.
- `add_printer_test_log`: agrega logs desde frontend.
- `open_printer_test_folder`: abre carpeta de evidencias.

## Impresion ESC/POS

El ticket de venta actual sigue el ejemplo de `tk-raw.txt`, que es un ticket ESC/POS en Base64 usado como referencia. La prueba ESC/POS imprime el recurso empaquetado `Frontend/src-tauri/resources/escpos/tk-raw.txt` decodificado como bytes RAW.

Puntos tecnicos:

- `ESC @` inicializa impresora: bytes `0x1B 0x40` o decimal `27, 64`.
- El llamado "byte 64" se refiere al `@` dentro de la secuencia `ESC @`, no a un byte aislado que imprima algo.
- Se usa RAW ESC/POS, no HTML ni PDF.
- Comandos usados en formato actual incluyen `ESC @`, `ESC M`, `ESC a`, `GS !`, `ESC d`, `ESC !`, `GS V`.
- El corte usa `GS V A 3`: bytes `0x1D 0x56 0x41 0x03`.

Flujos de ticket:

- Preferido: `print_sale_escpos` en Rust genera archivo `.escpos` temporal y lo manda al spooler con `printers` como RAW.
- Respaldo: `print_escpos_windows` en Rust recibe bytes desde JS y hace `copy /B` a una impresora compartida.
- Fallback final: `window.print()` visual si no hay configuracion nativa.

Formato de venta actual:

- Encabezado Ferreteria Malova.
- Fecha, hora, folio y vendedor fijo `08`.
- Columnas `COD CANT ART PRECIO IMPORTE`.
- Articulo fijo como `PZA` por ahora.
- Total resaltado.
- Leyenda de facturacion y devoluciones.

Si se cambia el formato ESC/POS, mantener sincronizados:

- `build_sale_escpos_file` en `Frontend/src-tauri/src/lib.rs`.
- `escposTicketBytes` y `ticketLines` en `Frontend/src/main.jsx`.

## PDFs, Cotizaciones Y Reportes

El estado actual evita depender de lectores PDF externos.

Flujo vigente:

- React genera PDF con `jsPDF`.
- Tauri guarda el PDF en Descargas con `save_pdf_downloads`.
- Para imprimir, React genera HTML imprimible equivalente y llama `window.print()` desde WebView2.
- Esto abre dialogo de impresion. No es silencioso.
- No depende de Edge como app PDF, Adobe ni Sumatra.

Limitacion actual:

- `window.print()` no permite impresion silenciosa ni seleccionar impresora automaticamente desde JS.
- Para imprimir PDFs directo y sin dialogo, se necesita solucion nativa o binario empaquetado.

Opciones futuras discutidas:

- Empaquetar SumatraPDF portable dentro del instalador Tauri y usar `-print-to "IMPRESORA" -silent archivo.pdf`.
- Implementar impresion nativa con WebView2/Win32 desde Rust.
- Renderizar PDF/HTML a imagen y mandar a Win32 print.

Branch experimental Sumatra:

- Rama: `feature/sumatra-pdf-printing`.
- Ruta esperada del binario: `Frontend/src-tauri/resources/sumatrapdf/SumatraPDF.exe`.
- Config Tauri empaqueta `resources/sumatrapdf/*`.
- Comando Rust: `print_pdf_sumatra`.
- Flujo: `jsPDF -> PDF en Descargas -> SumatraPDF portable -print-to/-print-to-default -silent -exit-when-done`.
- Si Sumatra no existe o falla, el frontend usa fallback HTML/WebView2 con dialogo y registra logs.

Branch experimental PDFtoPrinter:

- Rama: `feature/pdftoprinter-sidecar`.
- Ruta esperada del binario: `Frontend/src-tauri/resources/pdftoprinter/PDFtoPrinter.exe`.
- Config Tauri empaqueta `resources/pdftoprinter/*`.
- Comando Rust: `print_pdf_pdftoprinter`.
- Flujo: `PDF bytes -> PDF temporal -> PDFtoPrinter.exe "archivo.pdf" "impresora" -> spooler Windows`.
- Resolucion dinamica: busca primero ruta manual, variable `POS_PDFTOPRINTER_PATH`, Escritorio/Descargas/Documentos y rutas comunes; si no encuentra externo usa el empaquetado.
- Comando Rust adicional: `resolve_pdftoprinter`, devuelve candidato activo y rutas revisadas.
- Modo diagnostico: conserva el PDF en `Downloads/pos-printer-tests/pdf-diagnostics/` y genera trace detallado.
- El PDF temporal se elimina al terminar en modo normal; los logs registran comando, exit code, stdout/stderr, ruta del ejecutable, cola Windows y etapa de fallo.

Condicion del usuario:

- No instalar aplicaciones externas aparte de Tauri.
- Si se agrega herramienta auxiliar, debe ir incluida en el instalador de la app.

## Logs De Impresion

Carpeta de logs/evidencias:

- Windows: `C:\Users\<usuario>\Downloads\pos-printer-tests\`
- Linux: `~/Downloads/pos-printer-tests/`

Archivo principal:

- `printer-tests.log.jsonl`

Archivo detallado PDFtoPrinter:

- `pdf-print-trace.log.jsonl`

PDFs conservados por diagnostico PDFtoPrinter:

- `pdf-diagnostics/pdftoprinter-<trace_id>.pdf`

Cada linea es JSON compatible con `PrinterTestResult`:

- `created_at`: timestamp unix como string.
- `test_type`: tipo de intento, ejemplo `escpos`, `escpos-sale`, `pdf-webview`, `pdf-webview-test`.
- `method`: ruta tecnica usada.
- `printer_name`: impresora, recurso compartido o `Dialogo de Windows`.
- `file_path`: archivo generado o destino.
- `file_size`: tamano en bytes si aplica.
- `header`: `%PDF`, `ESC@` u otro indicador.
- `success`: `true` o `false`.
- `job_id`: id del spooler si existe.
- `message`: descripcion humana de la etapa.
- `error`: error si aplica.

Actualmente se registran:

- Pruebas ESC/POS.
- Ventas ESC/POS reales.
- Envio por impresora compartida.
- Guardado de PDF.
- Preparacion de HTML imprimible.
- Carga del documento en WebView2.
- Ejecucion de `window.print()`.
- Evento `afterprint`.
- Fallos por etapa.
- Diagnostico PDFtoPrinter paso a paso: resolucion de ejecutable, permisos `icacls`, escritura/relectura de PDF temporal, impresoras Windows, `Get-Printer`, `Get-PrinterPort`, `Get-PrintJob`, eventos PrintService, stdout/stderr, exit code y timeout.

Importante: en WebView2/`window.print()`, `success: true` significa que la app proceso la etapa y solicito impresion. No garantiza que el usuario haya hecho clic en Imprimir ni que la impresora fisica haya terminado.

## Versionado

Regla obligatoria: si se hace cualquier cambio funcional, cambio de codigo del sistema, integracion, recurso empaquetado, flujo de impresion, backend, frontend, Rust/Tauri o configuracion que afecte el comportamiento de la app, se debe subir la version aunque el usuario no lo pida explicitamente. Cambios solo de documentacion que no afectan la funcion del sistema no requieren cambio de version.

Cuando el usuario pida cambiar version para generar EXE/MSI, actualizar todos estos archivos:

- `Frontend/package.json`
- `Frontend/package-lock.json`
- `Frontend/src-tauri/Cargo.toml`
- `Frontend/src-tauri/Cargo.lock`
- `Frontend/src-tauri/tauri.conf.json`

Cuidado con `Cargo.lock`: no hacer reemplazos globales ingenuos de version porque puede tocar dependencias como `fnv` o `swift-rs`. Solo debe cambiar el paquete `pos_local`.

Verificar despues:

```bash
cd Frontend
npm run build
cd src-tauri
cargo check
```

## Archivos Locales No Trackeados

Los archivos de evidencia y referencia de pruebas reales viven en `evidencias-pruebas/`:

- `logs_primera_prueba.jpeg`: foto de logs de una prueba real.
- `logs_segunda_prueba.jpeg`: foto de logs de una prueba real con SumatraPDF.
- `tk-raw.txt`: ejemplo Base64 de ticket ESC/POS.

El recurso que usa la app empaquetada para prueba ESC/POS esta duplicado en `Frontend/src-tauri/resources/escpos/tk-raw.txt`.

## Flujo Recomendado Para Cambios

Antes de editar:

- Revisar `git status --short`.
- Leer archivos relevantes antes de asumir.
- No revertir cambios no hechos por el agente.

Despues de editar frontend/Rust:

- `npm run build` desde `Frontend`.
- `cargo check` desde `Frontend/src-tauri`.

Antes de commit/push:

- Revisar `git status --short`.
- Revisar `git diff --stat`.
- Revisar `git log --oneline -10`.
- Stagear solo archivos del cambio.
- No incluir evidencias locales ni builds salvo necesidad explicita.

## Riesgos Conocidos

- Backend no empaquetado con Tauri.
- Impresion PDF silenciosa no resuelta sin herramienta empaquetada o API nativa adicional.
- Los logs WebView2 no pueden confirmar impresion fisica, solo etapas de app/dialogo.
- ESC/POS depende de que la impresora/driver acepte RAW.
- El formato de ticket usa algunos datos fijos de Malova, como sucursal, vendedor y URL de facturacion.
- `cargo fmt` puede no estar instalado en el entorno Linux actual; usar `cargo check` como minimo.

## Prioridades Del Proyecto

- Validar instaladores Windows con versionado visible.
- Validar impresion real de tickets ESC/POS.
- Tener trazabilidad clara en logs de impresion.
- Evitar dependencias externas instaladas manualmente en las PCs finales.
- Mantener cambios pequenos y pragmaticos hasta definir solucion final para impresion directa de documentos.
