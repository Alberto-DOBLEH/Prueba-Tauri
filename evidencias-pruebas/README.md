# Evidencias Y Recursos De Prueba

Esta carpeta guarda material usado para diagnosticar y validar los flujos de impresion.

- `logs_primera_prueba.jpeg`: foto de logs de la primera prueba real de impresoras.
- `logs_segunda_prueba.jpeg`: foto de logs de la segunda prueba real con SumatraPDF y ESC/POS.
- `tk-raw.txt`: ticket ESC/POS en Base64 usado como referencia para el formato Malova.

Estos archivos son evidencia/referencia. No son necesarios para correr la app en produccion. El recurso usado por Tauri para la prueba ESC/POS empaquetada vive en `Frontend/src-tauri/resources/escpos/tk-raw.txt`.
