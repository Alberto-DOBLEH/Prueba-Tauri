# Frontend

Interfaz en React con Vite y contenedor Tauri.

## Dependencias

- Node.js y npm.
- Rust, Cargo y dependencias GTK/WebKit para Tauri en Fedora.
- Backend corriendo en `http://localhost:3001`.

## Comandos

- `npm install`: instala React, Vite, Tauri CLI y jsPDF.
- `npm run dev`: levanta la version web en `http://127.0.0.1:5173`.
- `npm run build`: genera `dist` para produccion.
- `npm run tauri:dev`: abre la aplicacion de escritorio en modo desarrollo.
- `npm run tauri:build`: genera paquete RPM.

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
- El paquete configurado es `rpm` para Fedora.
