import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import jsPDF from 'jspdf';
import './styles.css';

const API = 'http://localhost:3001/api';
const currency = new Intl.NumberFormat('es-MX', { style: 'currency', currency: 'MXN' });
const isTauri = () => Boolean(window.__TAURI_INTERNALS__);
const defaultPrintSettings = { pdfPrinterName: '', thermalPrinterName: '', thermalPrinterShare: '', autoPrintPdf: true };
const printerLogsChangedEvent = 'pos-printer-logs-changed';

function notifyPrinterLogsChanged() {
  window.dispatchEvent(new Event(printerLogsChangedEvent));
}

async function logPrinterEvent(payload) {
  if (!isTauri()) return null;
  try {
    const result = await invoke('add_printer_test_log', { payload });
    notifyPrinterLogsChanged();
    return result;
  } catch (error) {
    console.warn('No se pudo guardar el log de impresion.', error, payload);
    return null;
  }
}

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

function plainText(value) {
  return String(value || '')
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^\x20-\x7E\n]/g, '');
}

function ticketLines(sale) {
  const { date, time } = ticketDateTime(sale.created_at);
  return [
    'FERRETERIA MALOVA, S.A. DE C.V.',
    'SUC NOVIEMBRE',
    'DONDE USTED COMPRA DE CORAZON',
    'BLVD CENTENARIO 2104',
    'LOS ANGELES',
    'R.F.C. FMA850606P44',
    'TEL: (668)2392358',
    `FECHA: ${date}  T:${fitText(sale.folio, 9)}`,
    `HORA: ${time}  VEND: 08`,
    '',
    'COD  CANT  ART  PRECIO  IMPORTE',
    '---------------------------------',
    ...sale.items.flatMap((item) => [
      `${padRight(item.sku, 5)} ${padLeft(Number(item.quantity).toFixed(2), 5)} ${padRight('PZA', 5)} ${padLeft(ticketMoney(item.unit_price), 5)} ${padLeft(ticketMoney(item.total), 7)}`,
      `${fitText(item.name, 32)} `
    ]),
    '---------------------------------',
    `T O T A L : ${ticketMoney(sale.total)}`,
    '',
    `*** IDWEB PARA FACTURAR ${fitText(sale.folio, 12)} ***`,
    'FACTURA EN',
    'WWW.FERRETERIAMALOVA.COM.MX/FACTURACION',
    '***GRACIAS POR SU PREFERENCIA***',
    'TODA DEVOLUCION CAUSARA 20% DE',
    'CARGO Y NO SE ACEPTA DESPUES',
    'DE 8 DIAS',
    '---------------------------------'
  ].map(plainText);
}

function ticketMoney(value) {
  return Number(value || 0).toFixed(2);
}

function fitText(value, width) {
  return plainText(value).slice(0, width);
}

function padRight(value, width) {
  return fitText(value, width).padEnd(width, ' ');
}

function padLeft(value, width) {
  return fitText(value, width).padStart(width, ' ');
}

function ticketDateTime(createdAt) {
  const normalized = String(createdAt || '').replace('T', ' ');
  const [date = normalized || new Date().toISOString().slice(0, 10), time = '00:00:00'] = normalized.split(/\s+/);
  return { date, time: time.slice(0, 8) };
}

function escposTicketBytes(sale) {
  const encoder = new TextEncoder();
  const chunks = [];
  const push = (...bytes) => chunks.push(Uint8Array.from(bytes));
  const text = (value = '') => chunks.push(encoder.encode(`${plainText(value)}\n`));
  const { date, time } = ticketDateTime(sale.created_at);

  push(0x1b, 0x40, 0x1b, 0x4d, 0x00, 0x1b, 0x61, 0x01, 0x1d, 0x21, 0x00);
  text('FERRETERIA MALOVA, S.A. DE C.V.');
  text('SUC NOVIEMBRE');
  text('DONDE USTED COMPRA DE CORAZON');
  text('BLVD CENTENARIO 2104');
  text('LOS ANGELES');
  text('R.F.C. FMA850606P44');
  text('TEL: (668)2392358');
  text(`FECHA: ${date}  T:${fitText(sale.folio, 9)}`);
  text(`HORA: ${time}  VEND: 08`);
  push(0x1b, 0x64, 0x02, 0x1d, 0x21, 0x00);
  text('COD  CANT  ART  PRECIO  IMPORTE');
  text('---------------------------------');
  sale.items.forEach((item) => {
    text(`${padRight(item.sku, 5)} ${padLeft(Number(item.quantity).toFixed(2), 5)} ${padRight('PZA', 5)} ${padLeft(ticketMoney(item.unit_price), 5)} ${padLeft(ticketMoney(item.total), 7)}`);
    text(`${fitText(item.name, 32)} `);
  });
  text('---------------------------------');
  push(0x1b, 0x61, 0x02, 0x1b, 0x21, 0x08);
  text(`T O T A L : ${ticketMoney(sale.total)}`);
  push(0x1b, 0x21, 0x00, 0x1d, 0x21, 0x00);
  text('');
  push(0x1b, 0x21, 0x01, 0x1d, 0x21, 0x00, 0x1b, 0x61, 0x01);
  text(`*** IDWEB PARA FACTURAR ${fitText(sale.folio, 12)} ***`);
  push(0x1b, 0x21, 0x08);
  text('FACTURA EN');
  text('WWW.FERRETERIAMALOVA.COM.MX/FACTURACION');
  push(0x1b, 0x21, 0x00, 0x1b, 0x21, 0x01);
  text('***GRACIAS POR SU PREFERENCIA***');
  text('TODA DEVOLUCION CAUSARA 20% DE');
  text('CARGO Y NO SE ACEPTA DESPUES');
  text('DE 8 DIAS');
  push(0x1b, 0x21, 0x00);
  text('---------------------------------');
  push(0x1d, 0x56, 0x41, 0x03);

  const totalLength = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const bytes = new Uint8Array(totalLength);
  let offset = 0;
  chunks.forEach((chunk) => {
    bytes.set(chunk, offset);
    offset += chunk.length;
  });
  return bytes;
}

function printTicketFallback(sale) {
  const lines = [
    ...ticketLines(sale)
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

async function printTicket(sale, printSettings = defaultPrintSettings) {
  if (isTauri()) {
    if (printSettings.thermalPrinterName) {
      try {
        const jobId = await invoke('print_sale_escpos', { printerName: printSettings.thermalPrinterName, sale });
        await logPrinterEvent({
          testType: 'escpos-sale',
          method: 'Formato Malova ESC/POS RAW -> rust-printers -> WinSpool',
          printerName: printSettings.thermalPrinterName,
          filePath: 'Temp/pos-local-print/ticket-venta.escpos',
          fileSize: 0,
          header: 'ESC@',
          success: true,
          jobId,
          message: `Ticket de venta ${sale.folio} enviado al spooler como RAW.`,
          error: null
        });
        return;
      } catch (error) {
        await logPrinterEvent({
          testType: 'escpos-sale',
          method: 'Formato Malova ESC/POS RAW -> rust-printers -> WinSpool',
          printerName: printSettings.thermalPrinterName,
          filePath: 'Temp/pos-local-print/ticket-venta.escpos',
          fileSize: 0,
          header: 'ESC@',
          success: false,
          jobId: null,
          message: `Fallo al enviar ticket de venta ${sale.folio} al spooler.`,
          error: String(error)
        });
        throw error;
      }
    }
    if (printSettings.thermalPrinterShare) {
      const bytes = Array.from(escposTicketBytes(sale));
      try {
        const target = await invoke('print_escpos_windows', {
          printerShare: printSettings.thermalPrinterShare,
          bytes
        });
        await logPrinterEvent({
          testType: 'escpos-sale',
          method: 'Formato Malova ESC/POS RAW -> copy /B impresora compartida',
          printerName: printSettings.thermalPrinterShare,
          filePath: target,
          fileSize: bytes.length,
          header: 'ESC@',
          success: true,
          jobId: null,
          message: `Ticket de venta ${sale.folio} enviado a impresora compartida.`,
          error: null
        });
        return;
      } catch (error) {
        await logPrinterEvent({
          testType: 'escpos-sale',
          method: 'Formato Malova ESC/POS RAW -> copy /B impresora compartida',
          printerName: printSettings.thermalPrinterShare,
          filePath: '',
          fileSize: bytes.length,
          header: 'ESC@',
          success: false,
          jobId: null,
          message: `Fallo al enviar ticket de venta ${sale.folio} a impresora compartida.`,
          error: String(error)
        });
        throw error;
      }
    }
    await logPrinterEvent({
      testType: 'escpos-sale',
      method: 'Sin impresora ESC/POS configurada -> window.print fallback',
      printerName: 'Dialogo de Windows',
      filePath: '',
      fileSize: 0,
      header: null,
      success: true,
      jobId: null,
      message: `No hay impresora termica configurada. Ticket ${sale.folio} se enviara por window.print fallback.`,
      error: null
    });
  }
  printTicketFallback(sale);
}

function buildPdf(title, documentData) {
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
  return doc;
}

function escapeHtml(value) {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function documentRows(documentData) {
  return documentData.items || documentData.sales || documentData.products || [];
}

function printableDocumentHtml(title, documentData) {
  const rows = documentRows(documentData);
  const generatedAt = new Date().toLocaleString();
  const total = documentData.total ? `<p class="total">Total: ${escapeHtml(currency.format(documentData.total))}</p>` : '';
  const rowHtml = rows.map((row) => {
    const description = row.name
      ? `${row.sku || ''} ${row.name}`.trim()
      : `${row.folio || ''} ${row.customer_name || 'Mostrador'}`.trim();
    const quantity = row.quantity || row.stock || '';
    const amount = currency.format(row.total || row.price || 0);
    return `<tr><td>${escapeHtml(description)}</td><td>${escapeHtml(quantity)}</td><td>${escapeHtml(amount)}</td></tr>`;
  }).join('');

  return `<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <title>${escapeHtml(title)}</title>
  <style>
    @page { size: letter; margin: 14mm; }
    body { color: #111827; font-family: Arial, sans-serif; font-size: 12px; margin: 0; }
    h1 { font-size: 20px; margin: 0 0 12px; }
    .meta { color: #4b5563; margin-bottom: 18px; }
    table { border-collapse: collapse; width: 100%; }
    th, td { border-bottom: 1px solid #d1d5db; padding: 7px 5px; text-align: left; vertical-align: top; }
    th:nth-child(2), td:nth-child(2) { text-align: center; width: 70px; }
    th:nth-child(3), td:nth-child(3) { text-align: right; width: 110px; }
    .total { font-size: 15px; font-weight: 700; margin-top: 18px; text-align: right; }
  </style>
</head>
<body>
  <h1>${escapeHtml(title)}</h1>
  <div class="meta">
    <div>Folio: ${escapeHtml(documentData.folio || 'Reporte')}</div>
    <div>Generado: ${escapeHtml(generatedAt)}</div>
  </div>
  <table>
    <thead><tr><th>Descripcion</th><th>Cantidad</th><th>Importe</th></tr></thead>
    <tbody>${rowHtml}</tbody>
  </table>
  ${total}
</body>
</html>`;
}

function printPrintableDocument(title, documentData, logContext = {}) {
  const previousFrame = document.querySelector('.pdf-print-frame');
  previousFrame?.remove();

  const baseLog = {
    testType: logContext.testType || 'pdf-webview',
    method: logContext.method || 'PDF guardado -> HTML imprimible -> WebView2 window.print',
    printerName: logContext.printerName || 'Dialogo de Windows',
    filePath: logContext.filePath || '',
    fileSize: logContext.fileSize || 0,
    header: logContext.header || null,
    jobId: null
  };

  logPrinterEvent({
    ...baseLog,
    success: true,
    message: `HTML imprimible preparado para ${title}.`,
    error: null
  });

  const frame = document.createElement('iframe');
  frame.className = 'pdf-print-frame';
  frame.style.position = 'fixed';
  frame.style.right = '0';
  frame.style.bottom = '0';
  frame.style.width = '0';
  frame.style.height = '0';
  frame.style.border = '0';
  frame.srcdoc = printableDocumentHtml(title, documentData);
  document.body.appendChild(frame);

  frame.onload = () => {
    try {
      logPrinterEvent({
        ...baseLog,
        success: true,
        message: 'WebView2 cargo el documento imprimible. Se solicitara window.print().',
        error: null
      });
      frame.contentWindow?.focus();
      frame.contentWindow?.addEventListener('afterprint', () => {
        logPrinterEvent({
          ...baseLog,
          success: true,
          message: 'WebView2 reporto afterprint. El dialogo de impresion termino o se cerro.',
          error: null
        });
        frame.remove();
      }, { once: true });
      frame.contentWindow?.print();
      logPrinterEvent({
        ...baseLog,
        success: true,
        message: 'window.print() fue ejecutado. Windows debe mostrar el dialogo de impresion.',
        error: null
      });
      setTimeout(() => frame.remove(), 60000);
    } catch (error) {
      logPrinterEvent({
        ...baseLog,
        success: false,
        message: 'Fallo al ejecutar window.print() desde WebView2.',
        error: String(error)
      });
      frame.remove();
    }
  };
}

async function printPdfWithSumatraOrFallback(title, documentData, fileName, bytes, filePath, printSettings, testType = 'pdf-sumatra') {
  const printerName = printSettings.pdfPrinterName || '';
  const header = String.fromCharCode(...bytes.slice(0, 4));
  try {
    const result = await invoke('print_pdf_sumatra', {
      printerName,
      fileName,
      bytes: Array.from(bytes),
      testType
    });
    notifyPrinterLogsChanged();
    if (result.success) return result;

    await logPrinterEvent({
      testType: `${testType}-fallback`,
      method: 'SumatraPDF fallo -> HTML imprimible -> WebView2 window.print',
      printerName: printerName || 'Dialogo de Windows',
      filePath,
      fileSize: bytes.length,
      header,
      success: true,
      jobId: null,
      message: 'SumatraPDF no pudo completar la impresion silenciosa. Se usara fallback con dialogo WebView2.',
      error: result.error || result.message
    });
  } catch (error) {
    await logPrinterEvent({
      testType: `${testType}-fallback`,
      method: 'SumatraPDF invoke fallo -> HTML imprimible -> WebView2 window.print',
      printerName: printerName || 'Dialogo de Windows',
      filePath,
      fileSize: bytes.length,
      header,
      success: true,
      jobId: null,
      message: 'No se pudo invocar SumatraPDF desde Tauri. Se usara fallback con dialogo WebView2.',
      error: String(error)
    });
  }

  printPrintableDocument(title, documentData, {
    testType: `${testType}-fallback`,
    method: 'Fallback HTML imprimible -> WebView2 window.print',
    printerName: 'Dialogo de Windows',
    filePath,
    fileSize: bytes.length,
    header
  });
  return null;
}

async function printOrSavePdf(title, documentData, fileName, printSettings = defaultPrintSettings) {
  const doc = buildPdf(title, documentData);
  const bytes = new Uint8Array(doc.output('arraybuffer'));
  if (isTauri()) {
    const baseLog = {
      testType: 'pdf-webview',
      method: 'jsPDF -> save_pdf_downloads -> HTML imprimible -> WebView2 window.print',
      printerName: printSettings.pdfPrinterName || 'Dialogo de Windows',
      filePath: '',
      fileSize: bytes.length,
      header: String.fromCharCode(...bytes.slice(0, 4)),
      jobId: null
    };

    let filePath = '';
    try {
      filePath = await invoke('save_pdf_downloads', { fileName, bytes: Array.from(bytes) });
      await logPrinterEvent({
        ...baseLog,
        filePath,
        success: true,
        message: `PDF generado y guardado en Descargas: ${fileName}.`,
        error: null
      });
    } catch (error) {
      await logPrinterEvent({
        ...baseLog,
        success: false,
        message: `Fallo al guardar el PDF en Descargas: ${fileName}.`,
        error: String(error)
      });
      throw error;
    }

    if (printSettings.autoPrintPdf) {
      await printPdfWithSumatraOrFallback(title, documentData, fileName, bytes, filePath, printSettings);
    } else {
      await logPrinterEvent({
        ...baseLog,
        filePath,
        success: true,
        message: 'Impresion automatica desactivada. Solo se guardo el PDF.',
        error: null
      });
    }
    return;
  }
  doc.save(fileName);
}

function testPdfBytes() {
  const doc = buildPdf('Prueba de impresion PDF', {
    folio: 'PRUEBA-PDF',
    items: [{ sku: 'TEST', name: 'Documento de prueba para reportes y cotizaciones', quantity: 1, price: 0, total: 0 }]
  });
  return new Uint8Array(doc.output('arraybuffer'));
}

function SaleSection({ refreshAdmin, printSettings }) {
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
      if (kind === 'sale') await printTicket(data, printSettings);
      if (kind === 'quote') await printOrSavePdf('Cotizacion', data, `${data.folio}.pdf`, printSettings);
      setCart([]);
      setMessage(kind === 'sale' ? 'Venta registrada e impresa.' : 'Cotizacion guardada en Descargas y enviada a impresion si aplica.');
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

function AdminSection({ refreshKey, printSettings, setPrintSettings }) {
  const [dashboard, setDashboard] = useState(null);
  const [sales, setSales] = useState([]);
  const [quotes, setQuotes] = useState([]);

  const load = () => Promise.all([request('/dashboard'), request('/sales'), request('/quotes')]).then(([dash, saleList, quoteList]) => {
    setDashboard(dash); setSales(saleList); setQuotes(quoteList);
  });
  useEffect(() => { load(); }, [refreshKey]);

  const reprintSale = async (id) => printTicket(await request(`/sales/${id}`), printSettings);
  const reprintQuote = async (id) => {
    const quote = await request(`/quotes/${id}`);
    await printOrSavePdf('Cotizacion', quote, `${quote.folio}.pdf`, printSettings);
  };
  const report = async (type) => printOrSavePdf(type === 'sales' ? 'Reporte de ventas' : 'Reporte de inventario', await request(`/reports/${type}`), `reporte-${type}.pdf`, printSettings);

  return <section>
    <h2>Administracion</h2>
    <div className="print-settings">
      <h3>Impresion Windows</h3>
      <p>La seleccion principal de impresoras esta en la seccion <code>Impresoras</code>. Este campo queda como respaldo para impresoras compartidas antiguas.</p>
      <input value={printSettings.thermalPrinterShare} onChange={(event) => setPrintSettings({ ...printSettings, thermalPrinterShare: event.target.value })} placeholder="Respaldo: impresora compartida tipo POS58" />
      <label><input type="checkbox" checked={printSettings.autoPrintPdf} onChange={(event) => setPrintSettings({ ...printSettings, autoPrintPdf: event.target.checked })} /> Imprimir cotizaciones y reportes automaticamente con SumatraPDF cuando se use Tauri</label>
    </div>
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

function PrintersSection({ printSettings, setPrintSettings }) {
  const [printers, setPrinters] = useState([]);
  const [message, setMessage] = useState('');
  const [logs, setLogs] = useState([]);

  const loadPrinters = async () => {
    try {
      if (!isTauri()) {
        setMessage('La deteccion nativa solo funciona dentro de Tauri.');
        return;
      }
      const detected = await invoke('list_system_printers');
      const defaultPrinter = await invoke('get_default_system_printer');
      setPrinters(detected);
      setPrintSettings({
        ...printSettings,
        pdfPrinterName: printSettings.pdfPrinterName || defaultPrinter?.system_name || '',
        thermalPrinterName: printSettings.thermalPrinterName || defaultPrinter?.system_name || ''
      });
      setMessage(`Impresoras detectadas: ${detected.length}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  useEffect(() => { loadPrinters(); }, []);

  const loadLogs = async () => {
    try {
      if (!isTauri()) return;
      const entries = await invoke('get_printer_test_logs');
      setLogs(entries.reverse().slice(0, 20));
    } catch (error) {
      setMessage(String(error));
    }
  };

  useEffect(() => {
    const refreshLogs = () => loadLogs();
    window.addEventListener(printerLogsChangedEvent, refreshLogs);
    loadLogs();
    return () => window.removeEventListener(printerLogsChangedEvent, refreshLogs);
  }, []);

  const showResult = (result) => {
    setMessage(`${result.success ? 'OK' : 'ERROR'} | ${result.message}${result.error ? ` | ${result.error}` : ''}`);
    loadLogs();
  };

  const testPdfPrinter = async () => {
    try {
      const bytes = testPdfBytes();
      const filePath = await invoke('save_pdf_downloads', { fileName: 'prueba-pdf.pdf', bytes: Array.from(bytes) });
      await logPrinterEvent({
        testType: 'pdf-sumatra-test',
        method: 'jsPDF -> save_pdf_downloads -> SumatraPDF portable -print-to -silent',
        printerName: printSettings.pdfPrinterName || 'Predeterminada',
        filePath,
        fileSize: bytes.length,
        header: String.fromCharCode(...bytes.slice(0, 4)),
        success: true,
        jobId: null,
        message: 'PDF de prueba guardado. Se intentara impresion silenciosa con SumatraPDF.',
        error: null
      });
      await printPdfWithSumatraOrFallback('Prueba de impresion PDF', {
        folio: 'PRUEBA-PDF',
        items: [{ sku: 'TEST', name: 'Documento de prueba para reportes y cotizaciones', quantity: 1, price: 0, total: 0 }]
      }, 'prueba-pdf.pdf', bytes, filePath, printSettings, 'pdf-sumatra-test');
      setMessage('PDF guardado en Descargas. Se intento impresion silenciosa con SumatraPDF; si fallo, se abrio WebView2 como respaldo.');
    } catch (error) {
      await logPrinterEvent({
        testType: 'pdf-sumatra-test',
        method: 'jsPDF -> save_pdf_downloads -> SumatraPDF portable -print-to -silent',
        printerName: printSettings.pdfPrinterName || 'Predeterminada',
        filePath: '',
        fileSize: 0,
        header: null,
        success: false,
        jobId: null,
        message: 'Fallo la prueba de impresion silenciosa de documento con SumatraPDF.',
        error: String(error)
      });
      setMessage(String(error));
    }
  };

  const testPdfToPrinter = async () => {
    try {
      if (!printSettings.pdfPrinterName) throw new Error('Selecciona una impresora para PDFtoPrinter.');
      const bytes = testPdfBytes();
      const result = await invoke('print_pdf_pdftoprinter', {
        printerName: printSettings.pdfPrinterName,
        fileName: 'prueba-pdftoprinter.pdf',
        bytes: Array.from(bytes),
        testType: 'pdf-pdftoprinter-test'
      });
      showResult(result);
    } catch (error) {
      await logPrinterEvent({
        testType: 'pdf-pdftoprinter-test',
        method: 'PDF bytes -> PDF temporal -> PDFtoPrinter.exe ruta impresora',
        printerName: printSettings.pdfPrinterName || 'Sin impresora',
        filePath: '',
        fileSize: 0,
        header: null,
        success: false,
        jobId: null,
        message: 'Fallo la prueba de impresion con PDFtoPrinter.',
        error: String(error)
      });
      setMessage(String(error));
    }
  };

  const testEscpos = async () => {
    try {
      if (!printSettings.thermalPrinterName) throw new Error('Selecciona una impresora termica ESC/POS.');
      const result = await invoke('print_test_escpos', { printerName: printSettings.thermalPrinterName });
      showResult(result);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const openTestFolder = async () => {
    try {
      const path = await invoke('open_printer_test_folder');
      setMessage(`Carpeta de pruebas: ${path}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  return <section>
    <h2>Impresoras</h2>
    <div className="print-settings">
      <p>Modulo aislado para probar impresion desde Tauri. PDF: <code>PDF guardado -&gt; SumatraPDF portable</code> o <code>PDF temporal -&gt; PDFtoPrinter.exe</code>. ESC/POS: <code>tk-raw.txt -&gt; .escpos -&gt; RAW spooler</code>.</p>
      <button onClick={loadPrinters}>Detectar impresoras</button>
      <label>Impresora para PDFs silenciosos</label>
      <select value={printSettings.pdfPrinterName} onChange={(event) => setPrintSettings({ ...printSettings, pdfPrinterName: event.target.value })}>
        <option value="">Selecciona impresora PDF</option>
        {printers.map((printer) => <option key={`pdf-${printer.system_name}`} value={printer.system_name}>{printer.name} {printer.is_default ? '(predeterminada)' : ''}</option>)}
      </select>
      <label>Impresora termica ESC/POS</label>
      <select value={printSettings.thermalPrinterName} onChange={(event) => setPrintSettings({ ...printSettings, thermalPrinterName: event.target.value })}>
        <option value="">Selecciona impresora ESC/POS</option>
        {printers.map((printer) => <option key={`thermal-${printer.system_name}`} value={printer.system_name}>{printer.name} {printer.is_default ? '(predeterminada)' : ''}</option>)}
      </select>
      <div className="actions">
        <button onClick={testPdfPrinter}>Probar impresion documento</button>
        <button className="secondary" onClick={testPdfToPrinter}>Probar PDFtoPrinter</button>
        <button className="secondary" onClick={testEscpos}>Probar ESC/POS</button>
        <button className="secondary" onClick={openTestFolder}>Abrir carpeta pruebas</button>
        <button className="secondary" onClick={loadLogs}>Ver logs</button>
      </div>
      {message && <p className="message">{message}</p>}
    </div>
    <div className="print-settings">
      <h3>Logs de pruebas</h3>
      <p>Se guardan en <code>Descargas/pos-printer-tests/printer-tests.log.jsonl</code> junto con los PDFs y archivos <code>.escpos</code> generados.</p>
      <div className="table">
        {logs.map((log, index) => <div className="printer-row" key={`${log.created_at}-${index}`}>
          <strong>{log.success ? 'OK' : 'ERROR'} | {log.test_type} | {log.printer_name || 'Sin impresora'}</strong>
          <span>Fecha: {log.created_at} | Metodo: {log.method}</span>
          <span>Archivo: {log.file_path || 'No generado'} | Tamano: {log.file_size} bytes {log.header ? `| Header: ${log.header}` : ''}</span>
          <span>Job: {log.job_id || 'N/A'} | {log.message}</span>
          {log.error && <span>Error: {log.error}</span>}
        </div>)}
      </div>
    </div>
    <div className="table">
      {printers.map((printer) => <div className="printer-row" key={printer.system_name}>
        <strong>{printer.name}</strong>
        <span>Sistema: {printer.system_name}</span>
        <span>Driver: {printer.driver_name || 'Sin driver'}</span>
        <span>Puerto: {printer.port_name || 'Sin puerto'}</span>
        <span>Estado: {printer.state}</span>
        <span>{printer.is_default ? 'Predeterminada' : 'No predeterminada'} | {printer.is_shared ? 'Compartida' : 'No compartida'}</span>
      </div>)}
    </div>
  </section>;
}

function App() {
  const [section, setSection] = useState('sale');
  const [refreshKey, setRefreshKey] = useState(0);
  const [printSettings, setPrintSettingsState] = useState(() => {
    const saved = localStorage.getItem('pos-print-settings');
    if (!saved) return defaultPrintSettings;
    const parsed = JSON.parse(saved);
    return { ...defaultPrintSettings, ...parsed, thermalPrinterName: parsed.thermalPrinterName || parsed.thermalPrinterShare || '' };
  });
  const setPrintSettings = (settings) => {
    setPrintSettingsState(settings);
    localStorage.setItem('pos-print-settings', JSON.stringify(settings));
  };
  return <main>
    <header>
      <div><h1>Punto de Venta Local</h1><p>Express + SQLite + React + Tauri</p></div>
      <nav>{[['sale', 'Venta'], ['inventory', 'Inventario'], ['admin', 'Administracion'], ['printers', 'Impresoras']].map(([key, label]) => <button className={section === key ? 'active' : ''} onClick={() => setSection(key)} key={key}>{label}</button>)}</nav>
    </header>
    {section === 'sale' && <SaleSection refreshAdmin={() => setRefreshKey(refreshKey + 1)} printSettings={printSettings} />}
    {section === 'inventory' && <InventorySection />}
    {section === 'admin' && <AdminSection refreshKey={refreshKey} printSettings={printSettings} setPrintSettings={setPrintSettings} />}
    {section === 'printers' && <PrintersSection printSettings={printSettings} setPrintSettings={setPrintSettings} />}
  </main>;
}

createRoot(document.getElementById('root')).render(<App />);
