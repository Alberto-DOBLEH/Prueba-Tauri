import cors from 'cors';
import express from 'express';
import morgan from 'morgan';
import { db, generateFolio, initializeSchema, money } from './db.js';

initializeSchema();

const app = express();
const port = Number(process.env.PORT || 3001);

app.use(cors());
app.use(express.json());
app.use(morgan('dev'));

function getItems(table, idColumn, id) {
  return db.prepare(`SELECT * FROM ${table} WHERE ${idColumn} = ? ORDER BY id`).all(id);
}

function calculateItems(items) {
  if (!Array.isArray(items) || items.length === 0) {
    const error = new Error('El pedido necesita al menos un producto.');
    error.status = 400;
    throw error;
  }

  return items.map((item) => {
    const product = db.prepare('SELECT * FROM products WHERE id = ?').get(item.productId);
    if (!product || product.status !== 'available') {
      const error = new Error('Producto no disponible.');
      error.status = 400;
      throw error;
    }
    const quantity = Number(item.quantity || 0);
    if (!Number.isInteger(quantity) || quantity <= 0) {
      const error = new Error('La cantidad debe ser un entero positivo.');
      error.status = 400;
      throw error;
    }
    return { product, quantity, total: money(product.price * quantity) };
  });
}

function documentTotals(items) {
  const subtotal = money(items.reduce((sum, item) => sum + item.total, 0));
  const tax = money(subtotal * 0.16);
  return { subtotal, tax, total: money(subtotal + tax) };
}

app.get('/api/health', (_req, res) => {
  res.json({ ok: true, service: 'pos-backend' });
});

app.get('/api/products', (req, res) => {
  const search = `%${String(req.query.search || '').trim()}%`;
  const onlyAvailable = req.query.available === 'true';
  const sql = `
    SELECT * FROM products
    WHERE (sku LIKE ? OR name LIKE ? OR type LIKE ?)
    ${onlyAvailable ? "AND status = 'available'" : ''}
    ORDER BY name
  `;
  res.json(db.prepare(sql).all(search, search, search));
});

app.post('/api/products', (req, res) => {
  const { sku, name, type, price, stock = 0 } = req.body;
  if (!sku || !name || !type || Number(price) < 0 || Number(stock) < 0) {
    return res.status(400).json({ message: 'Datos invalidos para crear producto.' });
  }
  const result = db.prepare('INSERT INTO products (sku, name, type, price, stock) VALUES (?, ?, ?, ?, ?)').run(sku, name, type, money(price), Number(stock));
  res.status(201).json(db.prepare('SELECT * FROM products WHERE id = ?').get(result.lastInsertRowid));
});

app.patch('/api/products/:id/stock', (req, res) => {
  const id = Number(req.params.id);
  const { action, quantity = 0, note = '' } = req.body;
  const product = db.prepare('SELECT * FROM products WHERE id = ?').get(id);
  if (!product) return res.status(404).json({ message: 'Producto no encontrado.' });

  const qty = Number(quantity);
  if (!['add', 'remove', 'void'].includes(action)) return res.status(400).json({ message: 'Accion no valida.' });
  if (action !== 'void' && (!Number.isInteger(qty) || qty <= 0)) return res.status(400).json({ message: 'Cantidad no valida.' });
  if (action === 'remove' && product.stock < qty) return res.status(400).json({ message: 'Stock insuficiente.' });

  const tx = db.transaction(() => {
    if (action === 'add') db.prepare('UPDATE products SET stock = stock + ?, status = \'available\', updated_at = CURRENT_TIMESTAMP WHERE id = ?').run(qty, id);
    if (action === 'remove') db.prepare('UPDATE products SET stock = stock - ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?').run(qty, id);
    if (action === 'void') db.prepare("UPDATE products SET stock = 0, status = 'unavailable', updated_at = CURRENT_TIMESTAMP WHERE id = ?").run(id);
    db.prepare('INSERT INTO stock_movements (product_id, movement_type, quantity, note) VALUES (?, ?, ?, ?)').run(id, action, action === 'void' ? product.stock : qty, note);
  });
  tx();
  res.json(db.prepare('SELECT * FROM products WHERE id = ?').get(id));
});

app.post('/api/sales', (req, res, next) => {
  try {
    const items = calculateItems(req.body.items);
    const totals = documentTotals(items);
    for (const item of items) {
      if (item.product.stock < item.quantity) return res.status(400).json({ message: `Stock insuficiente para ${item.product.name}.` });
    }
    const folio = generateFolio('VTA');
    const tx = db.transaction(() => {
      const sale = db.prepare('INSERT INTO sales (folio, customer_name, subtotal, tax, total, payment_method) VALUES (?, ?, ?, ?, ?, ?)').run(folio, req.body.customerName || 'Cliente Mostrador', totals.subtotal, totals.tax, totals.total, req.body.paymentMethod || 'efectivo');
      for (const item of items) {
        db.prepare('INSERT INTO sale_items (sale_id, product_id, sku, name, quantity, unit_price, total) VALUES (?, ?, ?, ?, ?, ?, ?)').run(sale.lastInsertRowid, item.product.id, item.product.sku, item.product.name, item.quantity, item.product.price, item.total);
        db.prepare('UPDATE products SET stock = stock - ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?').run(item.quantity, item.product.id);
        db.prepare('INSERT INTO stock_movements (product_id, movement_type, quantity, note) VALUES (?, ?, ?, ?)').run(item.product.id, 'sale', item.quantity, `Venta ${folio}`);
      }
      return sale.lastInsertRowid;
    });
    const id = tx();
    res.status(201).json({ ...db.prepare('SELECT * FROM sales WHERE id = ?').get(id), items: getItems('sale_items', 'sale_id', id) });
  } catch (error) {
    next(error);
  }
});

app.get('/api/sales', (_req, res) => {
  res.json(db.prepare('SELECT * FROM sales ORDER BY created_at DESC').all());
});

app.get('/api/sales/:id', (req, res) => {
  const sale = db.prepare('SELECT * FROM sales WHERE id = ?').get(req.params.id);
  if (!sale) return res.status(404).json({ message: 'Venta no encontrada.' });
  res.json({ ...sale, items: getItems('sale_items', 'sale_id', sale.id) });
});

app.post('/api/quotes', (req, res, next) => {
  try {
    const items = calculateItems(req.body.items);
    const totals = documentTotals(items);
    const folio = generateFolio('COT');
    const tx = db.transaction(() => {
      const quote = db.prepare('INSERT INTO quotes (folio, customer_name, subtotal, tax, total, valid_until, notes) VALUES (?, ?, ?, ?, ?, ?, ?)').run(folio, req.body.customerName || 'Cliente Cotizacion', totals.subtotal, totals.tax, totals.total, req.body.validUntil || null, req.body.notes || '');
      for (const item of items) {
        db.prepare('INSERT INTO quote_items (quote_id, product_id, sku, name, quantity, unit_price, total) VALUES (?, ?, ?, ?, ?, ?, ?)').run(quote.lastInsertRowid, item.product.id, item.product.sku, item.product.name, item.quantity, item.product.price, item.total);
      }
      return quote.lastInsertRowid;
    });
    const id = tx();
    res.status(201).json({ ...db.prepare('SELECT * FROM quotes WHERE id = ?').get(id), items: getItems('quote_items', 'quote_id', id) });
  } catch (error) {
    next(error);
  }
});

app.get('/api/quotes', (_req, res) => {
  res.json(db.prepare('SELECT * FROM quotes ORDER BY created_at DESC').all());
});

app.get('/api/quotes/:id', (req, res) => {
  const quote = db.prepare('SELECT * FROM quotes WHERE id = ?').get(req.params.id);
  if (!quote) return res.status(404).json({ message: 'Cotizacion no encontrada.' });
  res.json({ ...quote, items: getItems('quote_items', 'quote_id', quote.id) });
});

app.get('/api/dashboard', (_req, res) => {
  const salesSummary = db.prepare('SELECT COUNT(*) AS count, COALESCE(SUM(total), 0) AS total FROM sales').get();
  const inventorySummary = db.prepare("SELECT COUNT(*) AS products, COALESCE(SUM(stock), 0) AS stock, SUM(CASE WHEN status = 'unavailable' THEN 1 ELSE 0 END) AS unavailable FROM products").get();
  const topProducts = db.prepare('SELECT name, sku, SUM(quantity) AS quantity, SUM(total) AS total FROM sale_items GROUP BY product_id ORDER BY quantity DESC LIMIT 5').all();
  res.json({ salesSummary, inventorySummary, topProducts });
});

app.get('/api/reports/sales', (_req, res) => {
  res.json({ generatedAt: new Date().toISOString(), sales: db.prepare('SELECT * FROM sales ORDER BY created_at DESC').all() });
});

app.get('/api/reports/inventory', (_req, res) => {
  res.json({ generatedAt: new Date().toISOString(), products: db.prepare('SELECT * FROM products ORDER BY type, name').all() });
});

app.use((error, _req, res, _next) => {
  res.status(error.status || 500).json({ message: error.message || 'Error interno.' });
});

app.listen(port, () => {
  console.log(`POS backend escuchando en http://localhost:${port}`);
});
