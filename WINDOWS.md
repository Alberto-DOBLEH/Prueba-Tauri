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

## Ubicacion de instaladores

- EXE NSIS: `Frontend\src-tauri\target\release\bundle\nsis\`
- MSI: `Frontend\src-tauri\target\release\bundle\msi\`

## Limitacion actual

La app Tauri instalada no levanta automaticamente el backend. Antes de usarla, el backend debe estar corriendo en `http://localhost:3001`.

Para una version final de Windows conviene integrar el backend como sidecar o migrar la logica de backend a comandos Tauri/Rust para que todo quede dentro de un solo instalador.
