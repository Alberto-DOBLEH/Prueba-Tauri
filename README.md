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

## Empaquetar Linux RPM

Con el backend probado y desde `Frontend`:

```bash
npm run tauri:build:linux
```

El RPM se genera en una ruta similar a:

`Frontend/src-tauri/target/release/bundle/rpm/`

## Adaptacion para Windows

Esta version ya tiene scripts separados para Linux y Windows. Lo recomendado es subir estos cambios a una branch, por ejemplo:

```bash
git checkout -b windows-build
git add .
git commit -m "Agregar configuracion de build para Windows"
git push -u origin windows-build
```

En Windows no necesitas cambiar el codigo principal. Desde la VM Windows clonas esa branch, instalas dependencias y generas instaladores.

Instaladores disponibles desde `Frontend`:

```bash
npm run tauri:build:windows
```

Ese comando intenta generar ambos formatos: `.exe` con NSIS y `.msi`.

Si quieres solo EXE:

```bash
npm run tauri:build:windows:exe
```

Si quieres solo MSI:

```bash
npm run tauri:build:windows:msi
```

Las salidas quedan normalmente en:

- `Frontend/src-tauri/target/release/bundle/nsis/`
- `Frontend/src-tauri/target/release/bundle/msi/`

Importante: el instalador de Tauri empaqueta la app de escritorio, pero en este proyecto el backend Express sigue siendo un proceso local separado. Para produccion Windows hay dos opciones futuras: correr el backend con Node instalado, o integrar/arrancar el backend como sidecar de Tauri.

## Impresion y PDFs

- Ventas: en Tauri/Windows se intenta enviar el ticket como bytes ESC/POS a una impresora termica compartida. En web/Linux queda un fallback con `window.print()` en formato de 58 mm.
- Cotizaciones: se genera PDF con `jsPDF`; en Tauri se guarda en Descargas y en Windows tambien se puede mandar directo a imprimir usando la accion `Print` registrada en Windows.
- Reportes: ventas e inventario se generan como PDF, se guardan en Descargas y en Tauri/Windows se intentan imprimir automaticamente con el visor PDF predeterminado.
- Historial: ventas y cotizaciones se pueden reimprimir desde administracion.
- Impresoras: modulo aislado para probar impresion PDF y ESC/POS desde Tauri, guardar evidencias y registrar logs por intento.

La configuracion de impresora esta en la pantalla `Administracion`, seccion `Impresion Windows`.
