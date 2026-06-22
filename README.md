# Fiori Inspector Studio RS

**Fiori Inspector Studio RS** es una herramienta Rust para analizar aplicaciones **SAP Fiori / SAPUI5** de forma interactiva, clara y profesional. Su objetivo es convertir una pantalla Fiori en un mapa de objetos automatizables: controles UI5, DOM renderizado, acciones candidatas, selectores, bindings, modelos y endpoints OData.

La estética del frontend sigue una línea limpia tipo Apple: panel lateral, tarjetas translúcidas, métricas de madurez, navegación por pestañas y acciones copiables.

## Objetivo

Automatizar Fiori de forma más robusta que con coordenadas o scraping HTML simple.

La herramienta trabaja en dos capas:

1. **DOM visible**: botones, inputs, tablas, links, roles ARIA, textos, clases SAPUI5, selectores CSS.
2. **Árbol lógico UI5**: controles `sap.m.*`, `sap.ui.*`, bindings, modelos, agregaciones, parent/children y endpoints observados.

Esto permite un enfoque parecido a SAP GUI Scripting, pero adaptado a la arquitectura web dinámica de SAPUI5/Fiori.

## Capacidades

- Interfaz web local servida desde Rust/Axum.
- Captura de una sesión Fiori viva mediante ChromeDriver o GeckoDriver.
- Análisis offline de HTML pegado o cargado desde archivo.
- Dashboard de calidad de automatización.
- Árbol visual de controles UI5.
- Listado de acciones candidatas: click, input, combo, link, tab, tabla.
- Inventario de bindings UI5 y context paths.
- Inventario de endpoints OData detectados.
- Evaluación de riesgos: UI5 no inicializado, IDs dinámicos, ausencia de endpoints, captura incompleta.
- Generación automática de workflow YAML como punto de partida.
- CLI compatible con el proyecto anterior: `snapshot-browser`, `analyze-html`, `summary`, `tree`, `actions`, `run-workflow`.

## Requisitos

- Rust estable.
- ChromeDriver o GeckoDriver para capturas de sesión viva.
- Acceso autorizado al entorno SAP Fiori.

Instalación de dependencias habituales en Ubuntu:

```bash
sudo apt update
sudo apt install -y chromium-chromedriver
```

En algunos sistemas el binario puede llamarse `chromedriver` o estar dentro del paquete de Chrome/Chromium.

## Puesta en marcha

```bash
cp config/local.toml.example config/local.toml
cargo run -- --config config/local.toml serve
```

Abre:

```text
http://127.0.0.1:7820
```

## Captura de una sesión Fiori viva

Arranca ChromeDriver:

```bash
chromedriver --port=9515
```

Después inicia el estudio:

```bash
cargo run -- --config config/local.toml serve --bind 127.0.0.1:7820
```

En la interfaz, introduce una URL Fiori, por ejemplo:

```text
https://fiori.miempresa.com/sap/bc/ui2/flp#Shell-home
```

Pulsa **Capturar sesión viva**.

Si tu organización usa SSO, login corporativo, MFA o VPN, puedes configurar una espera de login manual en `config/local.toml`:

```toml
[fiori]
manual_login_wait_secs = 45
```

## Análisis offline de HTML

En la interfaz puedes cargar un archivo `.html` o pegar el contenido HTML. Este modo es útil para documentación o evidencias, pero tiene limitaciones: no ve los modelos vivos ni todos los controles UI5 internos.

## CLI

Captura directa:

```bash
cargo run -- --config config/local.toml snapshot-browser \
  --url "https://fiori.miempresa.com/sap/bc/ui2/flp#Shell-home" \
  --output runs/home.json
```

Análisis de HTML estático:

```bash
cargo run -- analyze-html \
  --input examples/static_fiori_fragment.html \
  --output runs/static.json
```

Resumen:

```bash
cargo run -- summary --input runs/home.json
```

Árbol:

```bash
cargo run -- tree --input runs/home.json --max-depth 8
```

Acciones:

```bash
cargo run -- actions --input runs/home.json --contains buscar
```

Workflow:

```bash
cargo run -- --config config/local.toml run-workflow \
  --workflow workflows/fiori_sample_workflow.yaml \
  --output-dir runs/material-search
```

## Configuración

Archivo `config/local.toml.example`:

```toml
[browser]
webdriver_url = "http://localhost:9515"
browser = "chrome"
headless = false
accept_insecure_certs = true
window_width = 1600
window_height = 1000
user_data_dir = "./.browser-profile"

[fiori]
wait_for_ui5 = true
ui5_timeout_secs = 90
manual_login_wait_secs = 0
ready_selector = "body"

[extraction]
max_text_len = 240
include_hidden_controls = false
include_dom_nodes = true
include_performance_urls = true
max_controls = 5000
max_dom_nodes = 3000

[output]
pretty_json = true
```

## Filosofía profesional de automatización

Prioridad recomendada:

1. **OData/API** para operaciones de negocio repetibles y críticas.
2. **UI5 control tree** para descubrir estructura, bindings y controles.
3. **Stable IDs / suffix selectors / ARIA labels** para interacciones UI inevitables.
4. **DOM bruto** solo como fallback.
5. **Coordenadas de pantalla** nunca como primera opción.

## Riesgos y límites

- Algunas apps Fiori usan iframes, launchpad shells, lazy loading y navegación hash.
- Los IDs generados pueden cambiar entre sesiones.
- Una captura antes de que UI5 esté inicializado producirá información incompleta.
- HTML estático no equivale a aplicación viva.
- Debe usarse únicamente con autorización y respetando políticas corporativas.

## Estructura

```text
fiori-inspector-studio-rs/
├── src/
│   ├── browser.rs          # WebDriver + ejecución del probe JS
│   ├── config.rs           # Configuración TOML
│   ├── models.rs           # Snapshot, controles, bindings, endpoints
│   ├── report.rs           # Reportes CLI
│   ├── static_html.rs      # Análisis HTML offline
│   ├── studio.rs           # Backend Axum + API interactiva
│   ├── workflow.rs         # Workflows YAML
│   └── js/ui5_probe.js     # Extractor JavaScript ejecutado dentro del navegador
├── static/
│   ├── index.html          # Frontend Apple-like
│   ├── styles.css          # Sistema visual
│   └── app.js              # UI interactiva
├── config/
├── docs/
├── examples/
└── workflows/
```

## Validación recomendada

```bash
cargo fmt
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

## Evolución sugerida

- Persistencia en SQLite de snapshots y runs.
- Comparación visual entre dos capturas Fiori.
- Modo recorder: grabar acciones de usuario y convertirlas en YAML.
- Integración con cliente OData Rust para transformar acciones UI en llamadas API.
- Módulo de credenciales con `keyring` o integración corporativa.
- Exportación HTML/PDF de informes de automatización.
