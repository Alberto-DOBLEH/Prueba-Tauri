# Resumen De Pruebas Ghostscript Contra Docker IPP/CUPS

Este documento resume dos pruebas realizadas desde la VM Windows contra el servidor Docker que simula impresoras IPP/CUPS.

El objetivo era validar el flujo experimental:

```text
PDF -> Ghostscript -> PostScript/RAW -> printers -> Winspool -> IPP/CUPS Docker
```

## Pruebas Comparadas

### Prueba 1: Docker Virtual Printer PS

La primera prueba se hizo seleccionando la impresora:

```text
Docker Virtual Printer PS
```

Resultado observado en el servidor IPP/CUPS:

```text
operation-id=Print-Job
job-name POS Local PDF Ghostscript RAW
Print-Job Auto-type header: 0000000000000000
Print-Job client-error-attributes-or-values-not-supported
Unsupported document-format mimeMediaType value.
```

### Prueba 2: Predeterminada PWG

La segunda prueba se hizo usando la impresora predeterminada configurada como PWG.

Resultado observado en el servidor IPP/CUPS:

```text
operation-id=Print-Job
job-name POS Local PDF Ghostscript RAW
Print-Job Auto-type header: 0000000000000000
Print-Job client-error-attributes-or-values-not-supported
Unsupported document-format mimeMediaType value.
```

## Resultado General

Ambas pruebas terminaron con el mismo rechazo por parte del servidor IPP/CUPS:

```text
client-error-attributes-or-values-not-supported
Unsupported document-format mimeMediaType value.
```

En ambas, el servidor detecto el mismo header:

```text
0000000000000000
```

Esto significa que el servidor IPP/CUPS recibio un trabajo, pero el contenido recibido no fue reconocido como un documento valido.

## Que Si Funciono

En ambas pruebas:

- La impresora virtual estaba activa.
- La impresora aceptaba trabajos.
- Windows abrio conexion hacia el servidor IPP/CUPS.
- El servidor recibio `POST /ipp/print`.
- El servidor recibio una operacion `Print-Job`.
- El trabajo llego con nombre `POS Local PDF Ghostscript RAW`.

Esto confirma que el flujo llego hasta:

```text
App/Tauri -> printers -> Winspool -> IPP/CUPS Docker
```

## Que Fallo

El contenido enviado al servidor no tenia un formato valido o soportado.

El header detectado fue:

```text
00 00 00 00 00 00 00 00
```

Un PostScript valido deberia empezar normalmente con:

```text
%!PS
```

En hexadecimal:

```text
25 21 50 53
```

Un PDF valido empezaria con:

```text
%PDF
```

En hexadecimal:

```text
25 50 44 46
```

Por eso el servidor rechazo el trabajo.

## Interpretacion

El hecho de que la prueba PS y la prueba PWG terminen igual indica que el problema probablemente no esta en cual impresora Docker se selecciono, sino en el contenido que se manda al spooler.

Hipotesis principales:

- Ghostscript genero un archivo `.ps` invalido, vacio o con bytes nulos.
- El archivo `.ps` correcto existe, pero el codigo esta enviando otra ruta al spooler.
- El flujo sigue enviando al spooler aunque Ghostscript haya fallado.
- `printers` o Winspool no esta mandando el PostScript como se espera.
- El `document-format` usado como RAW no esta llegando a IPP como `application/postscript`.
- La cola IPP rechaza el formato porque no puede autodetectarlo.

## Punto Critico A Validar

Antes de seguir probando impresoras, hay que validar el archivo `.ps` generado por Ghostscript en la VM.

En PowerShell:

```powershell
Get-Item "C:\ruta\archivo.ps"
Format-Hex "C:\ruta\archivo.ps" -Count 16
```

Resultado esperado:

```text
25 21 50 53
```

Si el resultado es:

```text
00 00 00 00
```

entonces Ghostscript, la ruta de salida o la seleccion del archivo enviado estan mal.

## Datos Que Deben Aparecer En El Log De La App

Para diagnosticar correctamente el flujo Ghostscript, el log de `pdf-ghostscript-test` debe incluir:

- Ruta del PDF temporal.
- Tamano del PDF temporal.
- Header del PDF temporal.
- Ruta del archivo `.ps` generado.
- Tamano del `.ps`.
- Header del `.ps`.
- Exit code de Ghostscript.
- stdout de Ghostscript.
- stderr de Ghostscript.
- Ruta exacta enviada a `printers.print_file()`.
- `document-format` usado.
- `job_id` si `printers` lo devuelve.
- Error exacto si `printers` falla.

## Cambio Recomendado En El Experimento

El flujo experimental debe detenerse antes de imprimir si el `.ps` no es valido.

Validaciones minimas:

```text
1. El archivo .ps existe.
2. El tamano es mayor que 0.
3. El header inicia con %!PS.
```

Si falla cualquiera:

```text
No llamar printers.
Registrar ERROR.
Conservar el archivo en modo diagnostico o eliminarlo segun configuracion.
```

## Prueba Siguiente Recomendada

1. Generar PDF de prueba.
2. Convertirlo manualmente con Ghostscript en PowerShell.
3. Validar header del `.ps` con `Format-Hex`.
4. Si el `.ps` es valido, probar enviarlo con el flujo experimental.
5. Comparar el header que ve CUPS.

Comando manual sugerido:

```powershell
& "C:\ruta\ghostscript\bin\gswin64c.exe" `
  -dBATCH `
  -dNOPAUSE `
  -dSAFER `
  -sDEVICE=ps2write `
  -sOutputFile="C:\Temp\prueba-pos.ps" `
  "C:\Users\djmin\Downloads\prueba-pdf.pdf"
```

Validacion:

```powershell
Get-Item "C:\Temp\prueba-pos.ps"
Format-Hex "C:\Temp\prueba-pos.ps" -Count 16
```

## Conclusion

Tanto `Docker Virtual Printer PS` como la predeterminada `PWG` recibieron el intento de impresion, pero el servidor IPP/CUPS rechazo ambos trabajos por el mismo motivo: el contenido recibido inicia con bytes nulos y no con un header de documento valido.

La siguiente prioridad no es cambiar de impresora, sino confirmar que Ghostscript esta produciendo un `.ps` valido y que ese mismo archivo es el que se esta enviando mediante `printers`.
