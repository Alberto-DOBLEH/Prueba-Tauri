# Backend

API REST con Express y SQLite. El servidor corre por defecto en `http://localhost:3001`.

## Dependencias

- Node.js 20 o superior recomendado.
- npm.
- SQLite embebido mediante `better-sqlite3`; no requiere servidor externo.

## Comandos

- `npm install`: instala dependencias.
- `npm run seed`: crea `../BD/pos.sqlite` y carga 50 productos, 10 ventas y 3 cotizaciones.
- `npm run dev`: levanta Express con recarga usando `node --watch`.
- `npm start`: levanta Express en modo normal.

## Endpoints

- `GET /api/health`: estado del backend.
- `GET /api/products?search=&available=true`: lista productos filtrando por SKU, nombre o tipo. `available=true` limita a disponibles.
- `POST /api/products`: crea producto. Body: `sku`, `name`, `type`, `price`, `stock`.
- `PATCH /api/products/:id/stock`: administra stock. Body: `action` con `add`, `remove` o `void`, mas `quantity` y `note`.
- `POST /api/sales`: registra venta, descuenta stock y devuelve venta con partidas para imprimir ticket.
- `GET /api/sales`: historial de ventas.
- `GET /api/sales/:id`: detalle de venta para reimpresion.
- `POST /api/quotes`: registra cotizacion sin descontar stock y devuelve partidas para PDF.
- `GET /api/quotes`: historial de cotizaciones.
- `GET /api/quotes/:id`: detalle de cotizacion para reimprimir PDF.
- `GET /api/dashboard`: resumen simple de ventas e inventario.
- `GET /api/reports/sales`: datos para reporte PDF de ventas.
- `GET /api/reports/inventory`: datos para reporte PDF de inventario.

## Notas de implementacion

El backend calcula subtotales, IVA del 16% y totales en servidor para evitar confiar en el cliente. Las ventas validan stock disponible antes de registrarse. Las cotizaciones no modifican inventario.
