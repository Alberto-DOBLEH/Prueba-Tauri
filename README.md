# Punto de Venta Local con Tauri

Proyecto local de punto de venta dividido en tres carpetas principales:

- `Frontend`: React, Vite y Tauri para ejecutar la interfaz como aplicacion de escritorio.
- `Backend`: Express con endpoints REST para ventas, cotizaciones, inventario, reportes y dashboard.
- `BD`: SQLite como almacenamiento local persistente.

## Arranque rapido

1. Instalar dependencias del backend:
   `cd Backend && npm install`
2. Crear la base con datos iniciales:
   `npm run seed`
3. Levantar backend:
   `npm run dev`
4. En otra terminal, instalar frontend:
   `cd Frontend && npm install`
5. Ejecutar web:
   `npm run dev`
6. Ejecutar como escritorio con Tauri:
   `npm run tauri:dev`

## Dependencias del sistema en Fedora para Tauri

Instala Node.js, Rust y dependencias WebKit/GTK:

```bash
sudo dnf install nodejs npm rust cargo webkit2gtk4.1-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel patchelf
```

Si Rust no esta actualizado, instala con rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Como funciona Tauri aqui

Tauri vive en `Frontend/src-tauri`. En desarrollo, Tauri abre una ventana nativa que consume `http://127.0.0.1:5173`, servido por Vite. La UI React consume el backend Express por HTTP en `http://localhost:3001/api`.

En build, Tauri ejecuta `npm run build`, toma los archivos estaticos de `Frontend/dist` y los incrusta en la aplicacion de escritorio. El backend se mantiene como proceso local separado en este ejemplo, por lo que antes de abrir la app empaquetada debes tener el backend iniciado o configurarlo como servicio local.

## Empaquetar RPM

Con el backend probado y desde `Frontend`:

```bash
npm run tauri:build
```

El RPM se genera en una ruta similar a:

`Frontend/src-tauri/target/release/bundle/rpm/`

## Impresion y PDFs

- Ventas: se genera una ventana de impresion con formato de ticket termico de 58 mm usando texto monoespaciado.
- Cotizaciones: se genera PDF con `jsPDF`.
- Reportes: ventas e inventario se exportan como PDF desde administracion.
- Historial: ventas y cotizaciones se pueden reimprimir desde administracion.
