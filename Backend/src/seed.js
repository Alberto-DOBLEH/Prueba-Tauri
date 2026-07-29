import { db, generateFolio, initializeSchema, money } from './db.js';

initializeSchema();

const products = [
  ['ABR-001', 'Abrillantador multiusos 1L', 'Limpieza', 58, 45],
  ['CLL-002', 'Cloro domestico 1L', 'Limpieza', 24, 80],
  ['DET-003', 'Detergente liquido 3L', 'Limpieza', 129, 30],
  ['JBN-004', 'Jabon de manos 500ml', 'Higiene', 42, 60],
  ['PAP-005', 'Papel higienico 12 rollos', 'Higiene', 96, 42],
  ['CAF-006', 'Cafe molido 500g', 'Abarrotes', 118, 24],
  ['AZU-007', 'Azucar estandar 1kg', 'Abarrotes', 31, 55],
  ['ARR-008', 'Arroz super extra 1kg', 'Abarrotes', 34, 50],
  ['FRJ-009', 'Frijol pinto 1kg', 'Abarrotes', 39, 45],
  ['ACE-010', 'Aceite vegetal 900ml', 'Abarrotes', 48, 65],
  ['SAL-011', 'Sal refinada 1kg', 'Abarrotes', 18, 90],
  ['PAS-012', 'Pasta spaghetti 200g', 'Abarrotes', 16, 100],
  ['LAT-013', 'Atun en agua 140g', 'Enlatados', 23, 72],
  ['LCH-014', 'Leche entera 1L', 'Lacteos', 29, 40],
  ['YOG-015', 'Yogurt natural 1L', 'Lacteos', 47, 25],
  ['QUE-016', 'Queso panela 400g', 'Lacteos', 82, 20],
  ['HVO-017', 'Huevo blanco 18 piezas', 'Abarrotes', 58, 35],
  ['PAN-018', 'Pan integral 680g', 'Panaderia', 49, 22],
  ['GAL-019', 'Galletas saladas 186g', 'Botanas', 21, 70],
  ['REF-020', 'Refresco cola 2L', 'Bebidas', 38, 66],
  ['AGU-021', 'Agua natural 1.5L', 'Bebidas', 17, 90],
  ['JUG-022', 'Jugo naranja 1L', 'Bebidas', 32, 38],
  ['CER-023', 'Cereal hojuelas 500g', 'Abarrotes', 74, 27],
  ['SOP-024', 'Sopa instantanea vaso', 'Abarrotes', 15, 120],
  ['MAY-025', 'Mayonesa 390g', 'Abarrotes', 46, 33],
  ['CAT-026', 'Catsup 397g', 'Abarrotes', 29, 41],
  ['CHL-027', 'Salsa picante 150ml', 'Abarrotes', 19, 59],
  ['SRV-028', 'Servilletas 500 hojas', 'Higiene', 52, 36],
  ['BLS-029', 'Bolsa basura grande 20pz', 'Limpieza', 64, 31],
  ['ESP-030', 'Esponja fibra 3pz', 'Limpieza', 28, 47],
  ['TRP-031', 'Trapeador algodon', 'Limpieza', 76, 18],
  ['ESC-032', 'Escoba angular', 'Limpieza', 69, 20],
  ['DES-033', 'Desinfectante pino 2L', 'Limpieza', 55, 44],
  ['SHA-034', 'Shampoo familiar 750ml', 'Higiene', 89, 26],
  ['PAS-035', 'Pasta dental 150ml', 'Higiene', 37, 58],
  ['CEP-036', 'Cepillo dental medio', 'Higiene', 25, 63],
  ['DES-037', 'Desodorante aerosol', 'Higiene', 61, 29],
  ['CRE-038', 'Crema corporal 400ml', 'Higiene', 73, 21],
  ['PIL-039', 'Pilas AA 4pz', 'Electronica', 84, 34],
  ['FOC-040', 'Foco LED 9W', 'Electronica', 45, 37],
  ['EXT-041', 'Extension electrica 3m', 'Electronica', 128, 13],
  ['LIB-042', 'Libreta profesional', 'Papeleria', 36, 50],
  ['PLM-043', 'Pluma azul 12pz', 'Papeleria', 48, 40],
  ['CIN-044', 'Cinta adhesiva', 'Papeleria', 22, 56],
  ['MAR-045', 'Marcador permanente', 'Papeleria', 18, 61],
  ['DOG-046', 'Croquetas perro 2kg', 'Mascotas', 112, 19],
  ['CAT-047', 'Arena para gato 4kg', 'Mascotas', 97, 16],
  ['SNK-048', 'Papas fritas 170g', 'Botanas', 44, 52],
  ['CHO-049', 'Chocolate barra 90g', 'Dulces', 27, 75],
  ['GOM-050', 'Gomitas enchiladas 100g', 'Dulces', 22, 84]
];

const count = db.prepare('SELECT COUNT(*) AS total FROM products').get().total;
if (count === 0) {
  const insert = db.prepare('INSERT INTO products (sku, name, type, price, stock) VALUES (?, ?, ?, ?, ?)');
  const tx = db.transaction(() => products.forEach((item) => insert.run(...item)));
  tx();
}

function createDocument(kind, customerName, itemIndexes) {
  const table = kind === 'sale' ? 'sales' : 'quotes';
  const itemTable = kind === 'sale' ? 'sale_items' : 'quote_items';
  const idColumn = kind === 'sale' ? 'sale_id' : 'quote_id';
  const prefix = kind === 'sale' ? 'VTA' : 'COT';
  const selected = itemIndexes.map(([index, quantity]) => {
    const product = db.prepare('SELECT * FROM products WHERE sku = ?').get(products[index][0]);
    return { product, quantity, total: money(product.price * quantity) };
  });
  const subtotal = money(selected.reduce((sum, item) => sum + item.total, 0));
  const tax = money(subtotal * 0.16);
  const total = money(subtotal + tax);
  const folio = generateFolio(prefix);

  const tx = db.transaction(() => {
    const result = kind === 'sale'
      ? db.prepare('INSERT INTO sales (folio, customer_name, subtotal, tax, total, payment_method) VALUES (?, ?, ?, ?, ?, ?)').run(folio, customerName, subtotal, tax, total, 'efectivo')
      : db.prepare('INSERT INTO quotes (folio, customer_name, subtotal, tax, total, valid_until, notes) VALUES (?, ?, ?, ?, ?, date(\'now\', \'+15 day\'), ?)').run(folio, customerName, subtotal, tax, total, 'Cotizacion generada como dato inicial');
    selected.forEach(({ product, quantity, total: lineTotal }) => {
      db.prepare(`INSERT INTO ${itemTable} (${idColumn}, product_id, sku, name, quantity, unit_price, total) VALUES (?, ?, ?, ?, ?, ?, ?)`).run(result.lastInsertRowid, product.id, product.sku, product.name, quantity, product.price, lineTotal);
      if (kind === 'sale') {
        db.prepare('UPDATE products SET stock = stock - ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?').run(quantity, product.id);
        db.prepare('INSERT INTO stock_movements (product_id, movement_type, quantity, note) VALUES (?, ?, ?, ?)').run(product.id, 'sale', quantity, `Venta ${folio}`);
      }
    });
  });
  tx();
}

if (db.prepare('SELECT COUNT(*) AS total FROM sales').get().total === 0) {
  [
    [[0, 2], [5, 1], [20, 3]], [[9, 2], [12, 4]], [[18, 2], [19, 2], [48, 3]], [[30, 1], [31, 1]], [[33, 1], [34, 2]],
    [[41, 3], [42, 1]], [[45, 1], [46, 1]], [[23, 5], [24, 1]], [[13, 2], [16, 1]], [[28, 2], [29, 2]]
  ].forEach((items, index) => createDocument('sale', `Cliente Mostrador ${index + 1}`, items));
}

if (db.prepare('SELECT COUNT(*) AS total FROM quotes').get().total === 0) {
  [
    [[1, 10], [2, 4], [27, 6]], [[38, 8], [39, 12]], [[6, 5], [7, 5], [8, 5]]
  ].forEach((items, index) => createDocument('quote', `Cliente Cotizacion ${index + 1}`, items));
}

console.log('Base de datos inicializada en BD/pos.sqlite');
