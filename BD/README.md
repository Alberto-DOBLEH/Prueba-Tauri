# Base de Datos

SQLite guarda la informacion en `BD/pos.sqlite`. Se crea ejecutando `npm run seed` desde `Backend`.

## Tablas

### products

Catalogo e inventario actual.

- `id`: identificador interno.
- `sku`: clave unica para busqueda y ventas.
- `name`: nombre visible del producto.
- `type`: categoria o tipo para busqueda.
- `price`: precio unitario.
- `stock`: unidades disponibles.
- `status`: `available` o `unavailable` para anular productos sin borrarlos.
- `created_at`, `updated_at`: auditoria basica.

### sales

Cabecera de venta.

- `id`: identificador interno.
- `folio`: folio unico imprimible.
- `customer_name`: cliente o mostrador.
- `subtotal`, `tax`, `total`: importes calculados por backend.
- `payment_method`: forma de pago.
- `created_at`: fecha de venta.

### sale_items

Detalle congelado de cada venta.

- `sale_id`: referencia a venta.
- `product_id`: referencia al producto original.
- `sku`, `name`: copia historica para que el ticket no cambie si se edita el producto.
- `quantity`, `unit_price`, `total`: cantidades e importes de linea.

### quotes

Cabecera de cotizacion.

- `folio`: folio unico para PDF.
- `customer_name`: cliente cotizado.
- `subtotal`, `tax`, `total`: importes calculados.
- `valid_until`: vigencia opcional.
- `notes`: notas comerciales.
- `created_at`: fecha de cotizacion.

### quote_items

Detalle congelado de cotizacion. Tiene la misma idea que `sale_items`, pero no descuenta stock.

### stock_movements

Bitacora de inventario.

- `product_id`: producto afectado.
- `movement_type`: `add`, `remove`, `void` o `sale`.
- `quantity`: cantidad modificada.
- `note`: motivo o referencia.
- `created_at`: fecha del movimiento.

## Justificacion

La separacion entre cabeceras y partidas permite guardar documentos con varias lineas. Copiar `sku`, `name` y precio en partidas mantiene historial estable aunque el catalogo cambie. `stock_movements` permite auditar entradas, salidas, anulaciones y salidas por venta sin depender solo del stock actual.
