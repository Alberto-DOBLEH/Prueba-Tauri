# Informe: Prueba Ghostscript + `printers` Contra IPP/CUPS Docker

Este informe documenta el resultado observado al probar el flujo experimental de impresion PDF silenciosa con Ghostscript y la crate `printers`, usando una impresora virtual expuesta por Docker/CUPS/IPP.

El objetivo es que una sesion/agente en la VM Windows pueda continuar el diagnostico con el mismo contexto.

## Resumen Ejecutivo

El servidor IPP/CUPS recibio una solicitud `Print-Job`, pero rechazo el trabajo con:

```text
client-error-attributes-or-values-not-supported
Unsupported document-format mimeMediaType value.
```

La linea mas importante es:

```text
Print-Job Auto-type header: 0000000000000000
```

Eso indica que el servidor IPP no detecto un documento PostScript/PDF/raster valido. Un PostScript valido deberia iniciar normalmente con:

```text
%!PS
```

En hexadecimal:

```text
25 21 50 53
```

Por lo tanto, el problema actual no parece ser que IPP/CUPS no reciba nada. El servidor si recibe una solicitud de impresion, pero el contenido que llega no tiene un formato soportado o esta llegando corrupto/vacio.

## Log Recibido

```text
pdl-override-supported (keyword) attempted

    preferred-attributes-supported (boolean) false

    printer-get-attributes-supported (keyword) document-format

    printer-geo-location (unknown) unknown

    printer-is-accepting-jobs (boolean) true

    printer-info (textWithoutLanguage) Docker Virtual Printer

    printer-location (textWithoutLanguage) Docker Desktop en Fedora

    printer-name (nameWithoutLanguage) Docker Virtual Printer

    printer-organization (textWithoutLanguage) 

    printer-organizational-unit (textWithoutLanguage) 

    printer-strings-languages-supported (naturalLanguage) en

    printer-uuid (uri) urn:uuid:9d94201b-e7d4-3687-5c6c-de84c82805c4

    reference-uri-schemes-supported (1setOf uriScheme) file,ftp,http,https

    uri-authentication-supported (1setOf keyword) none,none

    uri-security-supported (1setOf keyword) none,tls

    which-jobs-supported (1setOf keyword) completed,not-completed,aborted,all,canceled,pending,pending-held,processing,processing-stopped

    printer-config-change-date-time (dateTime) 2026-08-05T17:11:52Z

    printer-config-change-time (integer) 0

    printer-current-time (dateTime) 2026-08-05T17:12:58Z

    printer-icons (1setOf uri) https://192.168.122.1:8631/icon-sm.png,https://192.168.122.1:8631/icon.png,https://192.168.122.1:8631/icon-lg.png

    printer-more-info (uri) https://192.168.122.1:8631/

    printer-state (enum) idle

    printer-state-change-date-time (dateTime) 2026-08-05T17:11:52Z

    printer-state-change-time (integer) 0

    printer-state-message (textWithoutLanguage) Idle.

    printer-state-reasons (keyword) none

    printer-strings-uri (uri) https://192.168.122.1:8631/en.strings

    printer-supply-info-uri (uri) https://192.168.122.1:8631/supplies

    printer-up-time (integer) 66

    printer-uri-supported (1setOf uri) ipp://192.168.122.1:8631/ipp/print,ipps://192.168.122.1:8631/ipp/print

    queued-job-count (integer) 0

Accepted connection from 172.20.0.1

172.20.0.1 POST /ipp/print

Request:

  version=1.0

  operation-id=Print-Job(0002)

  request-id=2


  operation-attributes-tag

    attributes-charset (charset) utf-8

    attributes-natural-language (naturalLanguage) en-us

    printer-uri (uri) http://192.168.122.1:8631/ipp/print

    job-name (nameWithoutLanguage) POS Local PDF Ghostscript

    requesting-user-name (nameWithoutLanguage) djmin

172.20.0.1 Print-Job Auto-type header: 0000000000000000

172.20.0.1 Print-Job client-error-attributes-or-values-not-supported (Unsupported document-format mimeMediaType value.)

172.20.0.1 OK
```

## Interpretacion Del Log

### La Impresora Virtual Esta Disponible

El servidor anuncia:

```text
printer-is-accepting-jobs true
printer-state idle
printer-state-message Idle.
queued-job-count 0
```

Esto indica que la impresora virtual esta en estado disponible y acepta trabajos.

### Windows Si Intento Enviar Un Trabajo

El servidor recibio:

```text
operation-id=Print-Job
job-name POS Local PDF Ghostscript
requesting-user-name djmin
```

Esto confirma que el flujo llego al punto de enviar algo desde Windows hacia IPP/CUPS.

### El Servidor Rechazo El Formato Del Documento

El rechazo fue:

```text
client-error-attributes-or-values-not-supported
Unsupported document-format mimeMediaType value.
```

El servidor no pudo aceptar el formato enviado.

### El Header Detectado No Es Valido

El servidor detecto:

```text
Auto-type header: 0000000000000000
```

Esto es sospechoso porque un archivo PostScript generado correctamente por Ghostscript deberia empezar con:

```text
%!PS
```

Hexadecimal esperado:

```text
25 21 50 53
```

Un header de ceros puede indicar:

- Se genero un archivo vacio/corrupto.
- Se envio el archivo equivocado al spooler.
- Ghostscript fallo pero el flujo continuo.
- `printers`/Winspool transformo o envolvio el trabajo de forma no compatible.
- La cola IPP no acepta el tipo enviado cuando se declara/manda como RAW.

## Hipotesis Principales

### 1. Ghostscript No Genero PostScript Valido

El archivo `.ps` temporal puede estar vacio, corrupto o iniciar con bytes nulos.

Validacion esperada en la VM:

```powershell
Get-Item "C:\ruta\archivo.ps"
Format-Hex "C:\ruta\archivo.ps" -Count 16
```

Debe iniciar con:

```text
25 21 50 53
```

Si inicia con ceros:

```text
00 00 00 00
```

el problema esta en la conversion o en la ruta del archivo generado.

### 2. Se Esta Enviando El Archivo Equivocado

Puede ocurrir que el codigo convierta a `.ps`, pero envie otra ruta al spooler.

Revisar en logs de la app:

- Ruta PDF temporal.
- Ruta PS generado.
- Tamano del PS.
- Header del PS.
- Ruta enviada a `printers.print_file()`.

### 3. El Flujo Sigue Aunque Ghostscript Falle

Si Ghostscript devuelve exit code diferente de 0, no debe enviarse nada al spooler.

El flujo correcto debe ser:

```text
Ghostscript falla
  -> eliminar temporales
  -> log ERROR
  -> no llamar printers
```

### 4. `document-format` No Es Compatible Con La Cola IPP

Si se esta enviando como RAW, Windows/IPP puede no declarar correctamente el formato real.

Se debe probar:

```text
document-format = application/postscript
```

en vez de solo:

```text
document-format = RAW
```

Dependiendo de la crate `printers` y Winspool, puede que esta propiedad no se propague hasta IPP como se espera.

### 5. La Cola Docker No Acepta PostScript

Aunque la impresora acepte trabajos, puede que no acepte `application/postscript` o RAW PostScript.

Hay que revisar si el servidor anuncia `document-format-supported`.

Comando sugerido desde Linux/Fedora o dentro del contenedor si aplica:

```bash
ipptool -tv ipp://192.168.122.1:8631/ipp/print get-printer-attributes.test
```

O revisar los logs completos donde aparezca:

```text
document-format-supported
```

## Puntos Que Debe Revisar El Agente En La VM

### Revisar Logs De La App

Abrir la pantalla `Impresoras` y revisar entradas `pdf-ghostscript-test`.

Buscar:

- Exit code de Ghostscript.
- stdout/stderr de Ghostscript.
- Ruta del PDF temporal.
- Ruta del PS temporal.
- Tamano del PS.
- Header del PS.
- Job id devuelto por `printers`, si existe.
- Error exacto de `printers`, si existe.

### Revisar Archivo PostScript Temporal

Si el archivo `.ps` se conserva para diagnostico, correr:

```powershell
Get-Item "C:\ruta\archivo.ps"
Format-Hex "C:\ruta\archivo.ps" -Count 16
```

Resultado bueno esperado:

```text
25 21 50 53
```

Resultado malo observado indirectamente por CUPS:

```text
00 00 00 00
```

### Probar Ghostscript Manualmente

Ejecutar algo equivalente:

```powershell
& "C:\ruta\ghostscript\bin\gswin64c.exe" `
  -dBATCH `
  -dNOPAUSE `
  -dSAFER `
  -sDEVICE=ps2write `
  -sOutputFile="C:\Temp\prueba-pos.ps" `
  "C:\Users\djmin\Downloads\prueba-pdf.pdf"
```

Luego validar:

```powershell
Get-Item "C:\Temp\prueba-pos.ps"
Format-Hex "C:\Temp\prueba-pos.ps" -Count 16
```

### Probar Envio Manual Del PS

Si el PS es valido, probar desde Windows imprimir manualmente ese archivo hacia la impresora IPP/CUPS si hay un comando disponible.

Tambien revisar cola Windows:

```powershell
Get-Printer
Get-PrintJob -PrinterName "NOMBRE EXACTO"
```

## Cambios Recomendados En Codigo

### Validar PS Antes De Imprimir

Antes de llamar `printers.print_file()`, validar:

- Existe.
- Tamano > 0.
- Header empieza con `%!PS`.

Si falla:

```text
No llamar printers.
Eliminar temporales.
Registrar ERROR.
```

### Registrar Mas Detalle Del PS

Agregar al log:

- `ps_path`.
- `ps_size`.
- `ps_header`.
- `ghostscript_exit_code`.
- `ghostscript_stdout`.
- `ghostscript_stderr`.

### Probar `application/postscript`

Cambiar experimentalmente:

```rust
raw_properties: &[("document-format", "RAW")]
```

por:

```rust
raw_properties: &[("document-format", "application/postscript")]
```

Si no cambia nada, puede que Winspool no lo propague a IPP.

### Conservar Temporales En Modo Diagnostico

Para pruebas, conviene no borrar inmediatamente el `.ps` cuando falla el envio, para poder inspeccionarlo.

Se puede agregar una bandera interna:

```text
keep_debug_files = true
```

O conservarlos siempre en:

```text
Downloads/pos-printer-tests/
```

## Criterio Para Siguiente Prueba

La siguiente prueba debe confirmar uno de estos resultados:

### Caso A: PS Invalido

```text
Ghostscript genero PS con header 00000000 o archivo vacio.
```

Accion:

- Corregir comando Ghostscript.
- Revisar rutas/argumentos.
- Revisar si faltan archivos de Ghostscript empaquetados.

### Caso B: PS Valido Pero IPP Lo Rechaza

```text
PS inicia con %!PS, pero IPP rechaza document-format.
```

Accion:

- Probar `application/postscript`.
- Revisar `document-format-supported` del servidor IPP.
- Probar PCL o volver a Sumatra.

### Caso C: PS Valido Y Spooler Acepta

```text
printers devuelve job_id y CUPS crea job.
```

Accion:

- Revisar si job completa.
- Revisar salida fisica/virtual.
- Comparar contra flujo Sumatra.

## Conclusion Actual

El log indica que el servidor IPP/CUPS recibio una solicitud de impresion llamada `POS Local PDF Ghostscript`, pero la rechazo porque el formato del documento no fue reconocido.

La pista principal es:

```text
Auto-type header: 0000000000000000
```

Antes de seguir probando impresoras, hay que confirmar si el archivo PostScript generado por Ghostscript realmente es valido y si ese mismo archivo es el que se esta enviando al spooler.

El siguiente cambio recomendado es fortalecer el comando experimental para registrar y validar el `.ps` antes de imprimir.
