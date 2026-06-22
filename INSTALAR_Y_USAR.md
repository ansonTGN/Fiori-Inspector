# Fiori Inspector Studio — Guía desde cero

Esta aplicación sirve para analizar pantallas SAP Fiori/SAPUI5 y preparar automatizaciones profesionales.

## 1. Arranque más sencillo

Desde la carpeta del proyecto:

```bash
./run_studio.sh
```

O directamente:

```bash
cargo run
```

Abre después:

```text
http://127.0.0.1:7820
```

## 2. Para analizar HTML guardado

1. Entra en la pestaña de análisis HTML.
2. Pega el HTML o carga un fragmento.
3. Pulsa analizar.
4. Revisa controles, acciones, selectores y riesgos.

## 3. Para analizar una sesión Fiori real

La aplicación no usa ChromeDriver. Por defecto intentará lanzar Chrome/Chromium automáticamente mediante CDP.

Arranca la aplicación:

```bash
cargo run
```

En la interfaz web:

1. Pega la URL de Fiori.
2. Pulsa capturar sesión viva.
3. Espera a que el navegador cargue Fiori.
4. Revisa el árbol UI5, acciones, bindings, OData y workflow sugerido.

## 4. Comandos útiles

```bash
cargo run -- --help
cargo run -- serve
cargo run -- analyze-html --input examples/static_fiori_fragment.html --output runs/static_snapshot.json
cargo run -- summary --input runs/static_snapshot.json
cargo run -- actions --input runs/static_snapshot.json
```

## 5. Problemas frecuentes

### Solo veo la ayuda en terminal

Con esta versión ya no debería ocurrir. `cargo run` arranca la web por defecto.

### No conecta con Chrome DevTools Protocol

Si el arranque automático falla, abre CDP manualmente y comprueba el estado:

```bash
google-chrome --remote-debugging-port=9222 --user-data-dir=./.browser-profile-cdp
curl http://127.0.0.1:9222/json/version
```

Si tu binario no se llama `google-chrome`, cambia `chrome_binary` en `config/local.toml` a `chromium` o `chromium-browser`.

### El puerto 7820 está ocupado

Usa:

```bash
cargo run -- serve --bind 127.0.0.1:7821
```

Y abre:

```text
http://127.0.0.1:7821
```
