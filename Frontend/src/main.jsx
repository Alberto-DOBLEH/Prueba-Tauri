import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import jsPDF from 'jspdf';
import './styles.css';

const API = 'http://localhost:3001/api';
const currency = new Intl.NumberFormat('es-MX', { style: 'currency', currency: 'MXN' });

async function request(path, options) {
  const response = await fetch(`${API}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...options
  });
  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Error de red' }));
    throw new Error(error.message);
  }
  return response.json();
}

function printTicket(sale) {
  const lines = [
    '      POS LOCAL',
    '   Ticket de venta',
    '------------------------------',
    `Folio: ${sale.folio}`,
    `Fecha: ${new Date(sale.created_at).toLocaleString()}`,
    `Cliente: ${sale.customer_name || 'Mostrador'}`,
    '------------------------------',
    ...sale.items.flatMap((item) => [
      item.name.slice(0, 28),
      `${item.quantity} x ${currency.format(item.unit_price)} = ${currency.format(item.total)}`
    ]),
    '------------------------------',
    `Subtotal: ${currency.format(sale.subtotal)}`,
    `IVA:      ${currency.format(sale.tax)}`,
    `TOTAL:    ${currency.format(sale.total)}`,
    '------------------------------',
    'Gracias por su compra'
  ];

  const previousTicket = document.querySelector('.print-ticket');
  previousTicket?.remove();

  const ticket = document.createElement('pre');
  ticket.className = 'print-ticket';
  ticket.textContent = lines.join('\n');
  document.body.appendChild(ticket);

  const cleanup = () => ticket.remove();
  window.addEventListener('afterprint', cleanup, { once: true });
  window.print();
  setTimeout(cleanup, 1000);
}

function downloadPdf(title, documentData, fileName) {
  const doc = new jsPDF();
  doc.setFontSize(16);
  doc.text(title, 14, 18);
  doc.setFontSize(10);
  doc.text(`Folio: ${documentData.folio || 'Reporte'}`, 14, 30);
  doc.text(`Generado: ${new Date().toLocaleString()}`, 14, 36);
  let y = 48;
  const rows = documentData.items || documentData.sales || documentData.products || [];
  rows.forEach((row) => {
    const text = row.name
      ? `${row.sku || ''} ${row.name} | Cant: ${row.quantity || row.stock || 0} | ${currency.format(row.total || row.price || 0)}`
      : `${row.folio} | ${row.customer_name || 'Mostrador'} | ${currency.format(row.total)}`;
    doc.text(text.slice(0, 110), 14, y);
    y += 7;
    if (y > 280) {
      doc.addPage();
      y = 20;
    }
  });
  if (documentData.total) doc.text(`Total: ${currency.format(documentData.total)}`, 14, y + 6);
  doc.save(fileName);
}

function SaleSection({ refreshAdmin }) {
  const [search, setSearch] = useState('');
  const [products, setProducts] = useState([]);
  const [cart, setCart] = useState([]);
  const [customerName, setCustomerName] = useState('Cliente Mostrador');
  const [message, setMessage] = useState('');

  useEffect(() => {
    request(`/products?available=true&search=${encodeURIComponent(search)}`).then(setProducts).catch((error) => setMessage(error.message));
  }, [search]);

  const addToCart = (product) => {
    setCart((current) => {
      const found = current.find((item) => item.productId === product.id);
      if (found) return current.map((item) => item.productId === product.id ? { ...item, quantity: item.quantity + 1 } : item);
      return [...current, { productId: product.id, name: product.name, sku: product.sku, price: product.price, quantity: 1 }];
    });
  };

  const subtotal = cart.reduce((sum, item) => sum + item.price * item.quantity, 0);
  const tax = subtotal * 0.16;
  const total = subtotal + tax;

  const submit = async (kind) => {
    try {
      const payload = { customerName, items: cart.map(({ productId, quantity }) => ({ productId, quantity })) };
      const data = await request(kind === 'sale' ? '/sales' : '/quotes', { method: 'POST', body: JSON.stringify(payload) });
      if (kind === 'sale') printTicket(data);
      if (kind === 'quote') downloadPdf('Cotizacion', data, `${data.folio}.pdf`);
      setCart([]);
      setMessage(kind === 'sale' ? 'Venta registrada e impresa.' : 'Cotizacion generada en PDF.');
      refreshAdmin();
    } catch (error) {
      setMessage(error.message);
    }
  };

  return <section className="grid two">
    <div>
      <h2>Venta</h2>
      <input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Buscar por SKU, nombre o tipo" />
      <div className="cards">
        {products.map((product) => <article className="card" key={product.id}>
          <strong>{product.name}</strong>
          <span>{product.sku} | {product.type}</span>
          <span>{currency.format(product.price)} | Stock {product.stock}</span>
          <button onClick={() => addToCart(product)} disabled={product.stock <= 0}>Agregar al pedido</button>
        </article>)}
      </div>
    </div>
    <aside className="panel">
      <h3>Pedido / Cotizacion</h3>
      <input value={customerName} onChange={(event) => setCustomerName(event.target.value)} placeholder="Cliente" />
      {cart.map((item) => <div className="line" key={item.productId}>
        <span>{item.name}</span>
        <input type="number" min="1" value={item.quantity} onChange={(event) => setCart(cart.map((cartItem) => cartItem.productId === item.productId ? { ...cartItem, quantity: Number(event.target.value) } : cartItem))} />
      </div>)}
      <p>Subtotal {currency.format(subtotal)}</p>
      <p>IVA {currency.format(tax)}</p>
      <h3>Total {currency.format(total)}</h3>
      <button disabled={!cart.length} onClick={() => submit('sale')}>Cobrar e imprimir ticket</button>
      <button disabled={!cart.length} className="secondary" onClick={() => submit('quote')}>Generar cotizacion PDF</button>
      {message && <p className="message">{message}</p>}
    </aside>
  </section>;
}

function InventorySection() {
  const [products, setProducts] = useState([]);
  const [form, setForm] = useState({ sku: '', name: '', type: '', price: '', stock: '' });
  const load = () => request('/products').then(setProducts);
  useEffect(() => { load(); }, []);

  const createProduct = async (event) => {
    event.preventDefault();
    await request('/products', { method: 'POST', body: JSON.stringify({ ...form, price: Number(form.price), stock: Number(form.stock) }) });
    setForm({ sku: '', name: '', type: '', price: '', stock: '' });
    load();
  };

  const moveStock = async (product, action) => {
    const quantity = action === 'void' ? 0 : Number(prompt('Cantidad', '1'));
    await request(`/products/${product.id}/stock`, { method: 'PATCH', body: JSON.stringify({ action, quantity, note: 'Movimiento desde inventario' }) });
    load();
  };

  return <section>
    <h2>Inventario</h2>
    <form className="form" onSubmit={createProduct}>
      {['sku', 'name', 'type', 'price', 'stock'].map((field) => <input key={field} value={form[field]} onChange={(event) => setForm({ ...form, [field]: event.target.value })} placeholder={field} required />)}
      <button>Agregar producto</button>
    </form>
    <div className="table">
      {products.map((product) => <div className="row" key={product.id}>
        <span>{product.sku}</span><span>{product.name}</span><span>{product.type}</span><span>{currency.format(product.price)}</span><span>Stock {product.stock}</span><span>{product.status}</span>
        <button onClick={() => moveStock(product, 'add')}>Aumentar</button>
        <button onClick={() => moveStock(product, 'remove')}>Quitar</button>
        <button onClick={() => moveStock(product, 'void')}>Anular</button>
      </div>)}
    </div>
  </section>;
}

function AdminSection({ refreshKey }) {
  const [dashboard, setDashboard] = useState(null);
  const [sales, setSales] = useState([]);
  const [quotes, setQuotes] = useState([]);

  const load = () => Promise.all([request('/dashboard'), request('/sales'), request('/quotes')]).then(([dash, saleList, quoteList]) => {
    setDashboard(dash); setSales(saleList); setQuotes(quoteList);
  });
  useEffect(() => { load(); }, [refreshKey]);

  const reprintSale = async (id) => printTicket(await request(`/sales/${id}`));
  const reprintQuote = async (id) => {
    const quote = await request(`/quotes/${id}`);
    downloadPdf('Cotizacion', quote, `${quote.folio}.pdf`);
  };
  const report = async (type) => downloadPdf(type === 'sales' ? 'Reporte de ventas' : 'Reporte de inventario', await request(`/reports/${type}`), `reporte-${type}.pdf`);

  return <section>
    <h2>Administracion</h2>
    {dashboard && <div className="stats">
      <article><strong>{dashboard.salesSummary.count}</strong><span>Ventas</span></article>
      <article><strong>{currency.format(dashboard.salesSummary.total)}</strong><span>Total vendido</span></article>
      <article><strong>{dashboard.inventorySummary.products}</strong><span>Productos</span></article>
      <article><strong>{dashboard.inventorySummary.stock}</strong><span>Unidades en almacen</span></article>
    </div>}
    <button onClick={() => report('sales')}>Reporte ventas PDF</button>
    <button className="secondary" onClick={() => report('inventory')}>Reporte inventario PDF</button>
    <div className="grid two">
      <div><h3>Historial de ventas</h3>{sales.map((sale) => <div className="history" key={sale.id}>{sale.folio} | {currency.format(sale.total)} <button onClick={() => reprintSale(sale.id)}>Reimprimir</button></div>)}</div>
      <div><h3>Historial de cotizaciones</h3>{quotes.map((quote) => <div className="history" key={quote.id}>{quote.folio} | {currency.format(quote.total)} <button onClick={() => reprintQuote(quote.id)}>Reimprimir PDF</button></div>)}</div>
    </div>
  </section>;
}

function App() {
  const [section, setSection] = useState('sale');
  const [refreshKey, setRefreshKey] = useState(0);
  return <main>
    <header>
      <div><h1>Punto de Venta Local</h1><p>Express + SQLite + React + Tauri</p></div>
      <nav>{[['sale', 'Venta'], ['inventory', 'Inventario'], ['admin', 'Administracion']].map(([key, label]) => <button className={section === key ? 'active' : ''} onClick={() => setSection(key)} key={key}>{label}</button>)}</nav>
    </header>
    {section === 'sale' && <SaleSection refreshAdmin={() => setRefreshKey(refreshKey + 1)} />}
    {section === 'inventory' && <InventorySection />}
    {section === 'admin' && <AdminSection refreshKey={refreshKey} />}
  </main>;
}

createRoot(document.getElementById('root')).render(<App />);
