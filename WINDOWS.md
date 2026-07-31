# Build para Windows

Esta guia es para una branch orientada a Windows del mismo repositorio.

## Crear branch desde Linux

```bash
git checkout -b windows-build
git add .
git commit -m "Agregar soporte de build para Windows"
git push -u origin windows-build
```

## Clonar branch en la VM Windows

```powershell
git clone -b windows-build git@github.com:Alberto-DOBLEH/Prueba-Tauri-Ferreteria.git
cd Prueba-Tauri-Ferreteria
```

## Dependencias Windows

Instala:

- Node.js LTS.
- Rust desde `https://rustup.rs/`.
- Visual Studio Build Tools con `Desktop development with C++`.
- WebView2 Runtime si Windows no lo trae instalado.

Verifica:

```powershell
node -v
npm -v
rustc --version
cargo --version
```

## Instalar dependencias del proyecto

```powershell
cd Backend
npm install
npm run seed
```

```powershell
cd ..\Frontend
npm install
```

## Ejecutar en desarrollo

Terminal 1:

```powershell
cd Backend
npm start
```

Terminal 2:

```powershell
cd Frontend
npm run tauri:dev
```

## Generar instaladores

Desde `Frontend`:

```powershell
npm run tauri:build:windows
```

Solo EXE:

```powershell
npm run tauri:build:windows:exe
```

Solo MSI:

```powershell
npm run tauri:build:windows:msi
```

Si vienes de una version anterior y la carpeta de salida sigue mostrando `1.0.0`, usa el build limpio:

```powershell
npm run tauri:build:windows:msi:clean
```

`git pull` no borra `Frontend\src-tauri\target` ni `Frontend\dist` porque son carpetas ignoradas. Por eso pueden quedar instaladores viejos junto a los nuevos.

## Ubicacion de instaladores

- EXE NSIS: `Frontend\src-tauri\target\release\bundle\nsis\`
- MSI: `Frontend\src-tauri\target\release\bundle\msi\`

## Limitacion actual

La app Tauri instalada no levanta automaticamente el backend. Antes de usarla, el backend debe estar corriendo en `http://localhost:3001`.

Para una version final de Windows conviene integrar el backend como sidecar o migrar la logica de backend a comandos Tauri/Rust para que todo quede dentro de un solo instalador.

## Impresion automatica en Windows

Esta branch incluye impresion nativa basica usando comandos Tauri.

La seccion `Impresoras` usa dos crates Rust:

- `printers`: detecta impresoras y envia trabajos al spooler de Windows.
- `escpos-rs`: genera comandos ESC/POS para tickets termicos.

Desde esa pantalla puedes seleccionar la impresora de PDFs, seleccionar la impresora termica y ejecutar pruebas sin crear una venta real.

Para PDFs usa primero `Guardar PDF prueba` y `Probar PDF Windows Print`. El boton `Probar PDF directo spooler` es diagnostico avanzado; `Microsoft Print to PDF` puede crear un archivo de 0 KB porque no acepta bytes PDF crudos por `WritePrinter`.

### Tickets termicos ESC/POS

La app genera bytes ESC/POS y los envia con `copy /B` a una impresora compartida.

Configura la impresora asi:

1. Instala la impresora termica en Windows.
2. Verifica que imprime desde Windows.
3. Comparte la impresora con un nombre corto, por ejemplo `POS58`.
4. Abre la app.
5. Ve a `Administracion > Impresion Windows`.
6. Escribe `POS58` o la ruta completa `\\NOMBRE-PC\POS58`.

Notas:

- Debe ser una impresora compatible con ESC/POS.
- Si el driver no acepta datos raw, puede no imprimir correctamente.
- El formato usa comandos ESC/POS basicos: inicializar, alineacion, negritas, avance de papel y corte.

### Reportes y cotizaciones PDF

Los PDFs se guardan en `Descargas` y tambien se mandan a Windows con la accion del sistema `Print` cuando esta activada la opcion de impresion automatica.

Requisitos:

- Tener una app PDF instalada y asociada a `.pdf`.
- Tener impresora predeterminada.
- Mantener activa la opcion `Imprimir PDFs automaticamente con Windows` en administracion.

Si Windows no puede imprimir el PDF automaticamente, desactiva esa opcion para que la app solo guarde el archivo PDF en `Descargas`.

## Probar versionado del MSI

La version de la app se controla en:

- `Frontend\package.json`
- `Frontend\package-lock.json`
- `Frontend\src-tauri\tauri.conf.json`
- `Frontend\src-tauri\Cargo.toml`

La version actual es `1.0.4`. Para generar un MSI de esa version:

```powershell
cd Frontend
npm run tauri:build:windows:msi:clean
```

Instala con doble clic o con:

```powershell
msiexec /i "ruta\al\POS Local_1.0.4_x64_en-US.msi"
```
