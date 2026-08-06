# PDFtoPrinter

Coloca aqui el ejecutable `PDFtoPrinter.exe` para probar impresion silenciosa de PDFs con la misma estructura del sistema anterior documentado.

Archivo esperado:

```text
PDFtoPrinter.exe
```

Ruta esperada:

```text
Frontend/src-tauri/resources/pdftoprinter/PDFtoPrinter.exe
```

El flujo implementado es:

```text
PDF bytes -> PDF temporal -> PDFtoPrinter.exe "archivo.pdf" "impresora" -> spooler Windows
```

No uses el instalador. Debe ser el ejecutable portable/standalone si la licencia permite empaquetarlo dentro del instalador Tauri.
