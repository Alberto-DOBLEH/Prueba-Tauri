# Frontend

Interfaz en React con Vite y contenedor Tauri.

## Dependencias

- Node.js y npm.
- Rust y Cargo.
- Dependencias GTK/WebKit para Tauri en Fedora si vas a generar RPM.
- Microsoft WebView2 y Visual Studio Build Tools si vas a generar instaladores Windows.
- Backend corriendo en `http://localhost:3001`.

## Comandos

- `npm install`: instala React, Vite, Tauri CLI y jsPDF.
- `npm run dev`: levanta la version web en `http://127.0.0.1:5173`.
- `npm run build`: genera `dist` para produccion.
- `npm run tauri:dev`: abre la aplicacion de escritorio en modo desarrollo.
- `npm run tauri:build`: genera los paquetes que soporte el sistema actual segun Tauri.
- `npm run tauri:build:linux`: genera paquete RPM en Fedora/Linux.
- `npm run tauri:build:windows`: genera instalador `.exe` NSIS y `.msi` desde Windows.
- `npm run tauri:build:windows:exe`: genera solo `.exe` NSIS desde Windows.
- `npm run tauri:build:windows:msi`: genera solo `.msi` desde Windows.

## Pantallas

### Venta

Incluye buscador por SKU, nombre o tipo. Los productos aparecen como tarjetas con nombre, SKU, precio, stock y boton para agregarlos al pedido.

Desde el pedido se puede:

- Cobrar venta: registra la venta, descuenta inventario y abre impresion de ticket termico.
- Generar cotizacion: registra cotizacion sin descontar stock y descarga PDF.

### Inventario

Permite crear productos y administrar existencias:

- Aumentar stock.
- Quitar stock.
- Anular stock, marcando el producto como no disponible y dejando stock en cero.

### Administracion

Muestra dashboard sencillo con ventas, total vendido, cantidad de productos y unidades en almacen.

Tambien incluye:

- Reporte de ventas en PDF.
- Reporte de inventario en PDF.
- Historial de ventas con boton de reimprimir ticket.
- Historial de cotizaciones con boton de reimprimir PDF.

## Tauri

La configuracion esta en `src-tauri/tauri.conf.json`.

- En desarrollo usa `devUrl: http://127.0.0.1:5173`.
- En produccion usa `frontendDist: ../dist`.
- La politica CSP permite llamadas HTTP al backend local en puerto `3001`.
- `bundle.targets` esta en `all`, pero los scripts limitan el build por plataforma.
- Linux/Fedora usa `npm run tauri:build:linux`.
- Windows usa `npm run tauri:build:windows`, `npm run tauri:build:windows:exe` o `npm run tauri:build:windows:msi`.

## Preparar Windows para compilar

Instala en la VM Windows:

- Node.js LTS.
- Rust con `rustup`.
- Microsoft Visual Studio Build Tools con carga de trabajo `Desktop development with C++`.
- Microsoft Edge WebView2 Runtime, normalmente ya viene instalado en Windows 10/11.

Luego abre PowerShell en la carpeta del proyecto:

```powershell
cd Frontend
npm install
npm run tauri:build:windows
```

Para correr en desarrollo desde Windows:

```powershell
cd Backend
npm install
npm run seed
npm start
```

En otra terminal:

```powershell
cd Frontend
npm install
npm run tauri:dev
```

## Notas para instaladores Windows

El `.exe` generado por NSIS suele ser mas practico para instalacion sencilla. El `.msi` es util si se quiere distribuir con politicas empresariales o administracion centralizada.

El backend Express no se instala automaticamente como servicio en esta version. La app Tauri instalada espera encontrar el backend en `http://localhost:3001`.
