# Plan Experimental: PDF Silencioso Con Ghostscript Y `printers`

Este documento describe una ruta experimental para imprimir PDFs/reportes desde Tauri sin dialogo de impresion, usando Ghostscript para convertir el PDF y la crate `printers` para enviar el trabajo al spooler.

La idea es probar este flujo en una branch separada antes de integrarlo a `main`.

## Objetivo

- Imprimir PDFs de cotizaciones/reportes sin `window.print()`.
- Evitar depender del dialogo de WebView2.
- Mantener componentes empaquetados dentro del instalador Tauri.
- Obtener `job_id` desde el spooler cuando sea posible.
- Registrar logs claros de cada etapa.
- Guardar el PDF definitivo solo cuando el spooler acepte el trabajo.
- Eliminar temporales cuando falle generacion, conversion o envio.

## Estado Actual Del Proyecto

El flujo principal actual en `main` es:

```text
React/jsPDF
  -> PDF en Descargas
  -> SumatraPDF portable empaquetado
  -> Windows spooler / impresora seleccionada
```

Si Sumatra falla, el frontend usa fallback:

```text
HTML imprimible
  -> WebView2
  -> window.print()
  -> dialogo Windows
```

Tickets ESC/POS no usan PDF. Se imprimen como RAW ESC/POS con `printers` o respaldo `copy /B`.

## Diagnostico Actual De Sumatra

La prueba local con Docker/CUPS/IPP mostro:

- SumatraPDF se ejecuta.
- Sumatra devuelve `exit code: 0`.
- Windows crea job.
- El servidor IPP recibe el trabajo.
- CUPS reporta `job-state processing`.
- El formato detectado por IPP fue `image/pwg-raster`.

Interpretacion:

```text
App -> Sumatra -> Windows -> IPP/CUPS funciona hasta crear job
```

El problema observado parece estar despues de Windows/Sumatra, probablemente en el backend virtual Docker/CUPS o en como completa el job.

## Riesgo Principal Del Nuevo Flujo

El flujo propuesto es:

```text
PDF
  -> Ghostscript
  -> PostScript
  -> printers RAW
  -> spooler
  -> impresora
```

Esto solo funcionara bien si la impresora o cola acepta PostScript. Muchas impresoras Windows convencionales no aceptan PostScript RAW; usan driver GDI, XPS, EMF, PCL u otros formatos.

Por eso este flujo debe probarse como experimento paralelo, no reemplazar Sumatra de inmediato.

## Branch Recomendada

Crear una branch desde `main`:

```bash
git checkout main
git pull origin main
git checkout -b feature/ghostscript-printers-pdf
```

## Componentes Necesarios

### Ghostscript Portable

Se necesita una distribucion completa de Ghostscript para Windows x64, no solo `gswin64c.exe`.

Estructura esperada:

```text
Frontend/src-tauri/resources/ghostscript/
  bin/
    gswin64c.exe
    gsdll64.dll
  lib/
  Resource/
```

Nota: la estructura exacta puede variar segun la distribucion descargada. Hay que conservar los archivos que Ghostscript necesite en runtime.

### Licencia

Ghostscript normalmente se distribuye bajo AGPL o licencia comercial, segun version/distribucion. Antes de dejarlo en `main`, revisar si la distribucion interna de la empresa es aceptable bajo esa licencia.

Para esta prueba experimental, documentar claramente que Ghostscript es componente de terceros.

## Configuracion Tauri

Agregar recursos en `Frontend/src-tauri/tauri.conf.json`:

```json
"resources": [
  "resources/sumatrapdf/*",
  "resources/escpos/*",
  "resources/ghostscript/**/*"
]
```

## Implementacion Rust Propuesta

Archivo principal:

```text
Frontend/src-tauri/src/lib.rs
```

Agregar comando experimental:

```rust
#[tauri::command]
fn print_pdf_ghostscript_printers(
    app: tauri::AppHandle,
    printer_name: String,
    file_name: String,
    bytes: Vec<u8>,
) -> PrinterTestResult
```

No reemplazar `print_pdf_sumatra` inicialmente.

## Flujo Interno

```text
1. Recibir bytes PDF desde frontend
2. Guardar PDF temporal en carpeta pending
3. Validar PDF
4. Resolver ruta de Ghostscript empaquetado
5. Convertir PDF temporal a PostScript temporal
6. Buscar impresora con printers
7. Enviar PostScript como RAW al spooler
8. Si printers devuelve Ok(job_id):
   - mover/copiar PDF a Descargas o carpeta definitiva
   - registrar OK con job_id
9. Si falla conversion o spooler:
   - eliminar PDF temporal
   - eliminar PS temporal
   - registrar ERROR
```

## Carpetas Temporales

Usar una carpeta controlada por la app:

```text
%TEMP%/pos-local-print/pdf-pending/
%TEMP%/pos-local-print/pdf-converted/
```

Ejemplo de archivos:

```text
report-<timestamp>.pdf
report-<timestamp>.ps
```

## Validacion PDF

Antes de convertir:

- Existe archivo.
- Tamano mayor a cero.
- Header empieza con `%PDF-`.

Si falla:

- No imprimir.
- Eliminar temporal.
- Log `ERROR`.

## Comando Ghostscript Inicial

Primer modo a probar: PostScript.

```powershell
gswin64c.exe `
  -dBATCH `
  -dNOPAUSE `
  -dSAFER `
  -sDEVICE=ps2write `
  -sOutputFile="salida.ps" `
  "entrada.pdf"
```

En Rust, ejecutar con `Command` usando argumentos separados, no string concatenado.

Registrar:

- Ruta `gswin64c.exe`.
- Ruta PDF entrada.
- Ruta PS salida.
- Exit code.
- stdout.
- stderr.

## Envio Con `printers`

Despues de generar `.ps`:

```rust
printer.print_file(
    ps_path.to_string_lossy().as_ref(),
    PrinterJobOptions {
        name: Some("POS Local PDF Ghostscript"),
        raw_properties: &[("document-format", "RAW")],
        converter: printers::common::converters::Converter::None,
    },
)
```

Si devuelve `Ok(job_id)`, significa que el spooler acepto el trabajo. No garantiza impresion fisica.

## UI Experimental

En `Frontend/src/main.jsx`, en pantalla `Impresoras`, agregar boton:

```text
Probar PDF Ghostscript
```

Mantener botones existentes:

- `Probar impresion documento`: SumatraPDF.
- `Probar PDF Ghostscript`: nuevo experimento.
- `Probar ESC/POS`: tk-raw ESC/POS.

No cambiar todavia cotizaciones/reportes reales hasta validar.

## Logs Esperados

Usar el mismo `printer-tests.log.jsonl`.

Nuevo `test_type` sugerido:

```text
pdf-ghostscript-test
```

Campos importantes:

- `method`: `jsPDF -> PDF temp -> Ghostscript ps2write -> printers RAW -> WinSpool`
- `printer_name`: impresora seleccionada.
- `file_path`: PDF definitivo si se archivo, PDF temporal si fallo antes.
- `file_size`: tamano del PDF.
- `header`: `%PDF`.
- `success`: true/false.
- `job_id`: id devuelto por `printers`.
- `message`: etapa final.
- `error`: stderr/error si aplica.

Mensajes sugeridos:

```text
PDF temporal generado y validado.
Ghostscript convirtio PDF a PostScript.
Spooler acepto trabajo PostScript RAW. Job: X.
PDF definitivo archivado.
Ghostscript fallo, temporales eliminados.
Spooler rechazo trabajo, temporales eliminados.
```

## Politica De Guardado PDF

No guardar definitivamente antes de imprimir.

Flujo:

```text
PDF temporal
  -> conversion
  -> spooler Ok(job_id)
  -> copiar/mover a Descargas
```

Si falla conversion o spooler:

```text
eliminar temporal PDF
eliminar temporal PS
```

Si spooler acepta pero falla mover el PDF:

```text
success false o warning
mensaje: trabajo aceptado pero no se pudo archivar PDF
mantener evidencia si es posible
```

## Criterios De Exito

No usar el mensaje “impreso correctamente”.

Usar:

```text
Reporte enviado a la impresora.
```

O:

```text
Spooler acepto el trabajo. Job: X.
```

Porque `Ok(job_id)` no confirma que la hoja salio fisicamente.

## Plan De Pruebas En Windows VM

1. Descargar/preparar Ghostscript completo.
2. Colocarlo en `Frontend/src-tauri/resources/ghostscript/`.
3. Correr:

```powershell
cd Frontend
npm install
npm run build
cd src-tauri
cargo check
```

4. Generar instalador:

```powershell
cd ..
npm run tauri:build:windows:exe
```

5. Instalar app.
6. Abrir backend.
7. En app, ir a `Impresoras`.
8. Detectar impresoras.
9. Seleccionar impresora PDF.
10. Probar `Probar PDF Ghostscript`.
11. Revisar logs de app.
12. Revisar cola Windows.
13. Si se usa Docker/CUPS/IPP, revisar logs del contenedor.

## Pruebas Contra Docker/CUPS/IPP

Comandos utiles despues de imprimir:

```bash
docker logs <contenedor>
docker exec -it <contenedor> sh
lpstat -t
lpstat -W all -o
cat /var/log/cups/error_log
```

Revisar si CUPS detecta formato:

```text
application/postscript
```

Si queda en `processing`, puede ser problema del backend virtual, no de Tauri.

## Riesgos

- Ghostscript aumenta tamano del instalador.
- Licencia mas delicada que Sumatra.
- PostScript RAW puede no funcionar en impresoras sin soporte PS.
- Algunas colas Windows aceptan el job pero no imprimen.
- La API de `printers` puede diferir de ejemplos online.
- Confirmacion fisica requiere monitoreo adicional IPP/SNMP/driver.

## Alternativas Si PostScript Falla

1. Seguir con SumatraPDF como flujo principal.
2. Probar Ghostscript a PCL si la impresora soporta PCL.
3. Renderizar PDF a imagen y usar Win32/GDI.
4. Usar APIs nativas WebView2/Win32 para imprimir sin dialogo.

## Recomendacion

Implementar Ghostscript como experimento paralelo.

No reemplazar Sumatra hasta comprobar en impresora real que:

- Ghostscript convierte correctamente.
- `printers` devuelve `job_id`.
- La cola completa el trabajo.
- La salida fisica o virtual es correcta.

Si funciona mejor que Sumatra, entonces se puede decidir si pasa a flujo principal de reportes/cotizaciones.
