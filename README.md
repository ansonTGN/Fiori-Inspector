# Fiori Inspector Studio

**Herramienta profesional en Rust para analizar aplicaciones SAP Fiori / SAPUI5 y construir automatizaciones robustas, editables, verificables y trazables.**

Autor: **Angel A. Urbina**

---

## 1. Descripción general

**Fiori Inspector Studio** es una aplicación desarrollada en Rust para analizar aplicaciones **SAP Fiori / SAPUI5**, extraer su estructura técnica y facilitar la creación de workflows profesionales de automatización.

La herramienta permite inspeccionar una pantalla Fiori, identificar controles, revisar selectores, generar workflows YAML, editarlos desde la propia interfaz, validarlos y ejecutarlos paso a paso o de forma completa.

El objetivo principal es convertir el análisis visual y técnico de una aplicación Fiori en una automatización clara, mantenible y auditable.

Esta versión incorpora un **Laboratorio visual de Workflows** que permite trabajar directamente con scripts YAML desde la pantalla principal de la aplicación.

---

## 2. Objetivo del proyecto

SAP Fiori sustituye o complementa muchos procesos que antes se realizaban en SAP GUI clásico. Sin embargo, automatizar Fiori no debe hacerse mediante coordenadas de pantalla ni clicks frágiles sobre HTML generado dinámicamente.

Fiori Inspector Studio busca ofrecer una alternativa profesional:

* Analizar la estructura real de una pantalla Fiori.
* Identificar controles SAPUI5.
* Detectar acciones automatizables.
* Proponer selectores.
* Valorar riesgos de estabilidad.
* Generar workflows YAML.
* Editar los workflows desde una pantalla visual.
* Validar cada paso antes de ejecutar.
* Ejecutar un flujo completo o hasta un paso concreto.
* Generar evidencias de ejecución.

La filosofía del proyecto es clara: **primero entender, después automatizar**.

---

## 3. Características principales

### 3.1. Estudio interactivo local

La aplicación incluye una interfaz web local con diseño limpio, claro e inspirado en una estética tipo Apple.

Dirección por defecto:

```text
http://127.0.0.1:7820
```

Desde esta interfaz se puede:

* Analizar una página Fiori viva.
* Analizar HTML estático.
* Ver controles detectados.
* Revisar acciones candidatas.
* Consultar selectores recomendados.
* Generar workflows YAML.
* Editar workflows.
* Validar workflows.
* Ejecutar workflows completos.
* Ejecutar workflows hasta un paso seleccionado.
* Revisar resultados y evidencias.

---

### 3.2. Sin ChromeDriver

Esta versión **no utiliza ChromeDriver** ni GeckoDriver.

La comunicación con el navegador se realiza mediante **Chrome DevTools Protocol — CDP**.

Arquitectura descartada:

```text
Rust → ChromeDriver → Chrome → SAP Fiori
```

Arquitectura actual:

```text
Rust → Chrome DevTools Protocol → Chrome / Chromium → SAP Fiori
```

Esto simplifica el entorno técnico y evita depender de versiones concretas de ChromeDriver.

---

### 3.3. Análisis Fiori en vivo

La herramienta puede conectarse a Chrome o Chromium mediante CDP y analizar una aplicación Fiori real.

Puede extraer:

* URL actual.
* Título de página.
* DOM HTML renderizado.
* Controles SAPUI5 cuando están disponibles.
* Botones.
* Inputs.
* Links.
* Tablas.
* Pestañas.
* Textos visibles.
* Atributos ARIA.
* Selectores candidatos.
* Identificadores UI5.
* Riesgo de automatización.
* Posibles bindings o rutas de modelo.

---

### 3.4. Análisis de HTML estático

También se puede analizar un HTML guardado o simulado.

Este modo es útil para:

* Pruebas offline.
* Simulaciones.
* Formación.
* Desarrollo del parser.
* Documentación técnica.
* Validación inicial de la interfaz.

Limitación: un HTML estático no siempre conserva toda la información del runtime SAPUI5.

---

### 3.5. Laboratorio visual de Workflows

La mejora más importante de esta versión es la nueva pantalla **Workflow**.

Desde esta pantalla se puede:

* Ver el workflow generado automáticamente.
* Copiar el YAML.
* Editar el YAML directamente.
* Descargar el workflow.
* Restaurar el workflow original.
* Validar sintaxis y estructura.
* Ver errores y avisos.
* Ver pasos detectados.
* Revisar riesgo por paso.
* Detectar selectores frágiles.
* Ejecutar el workflow completo.
* Ejecutar solo hasta un paso seleccionado.
* Revisar informe de ejecución.

El objetivo es que una persona pueda pasar de un análisis visual a una automatización controlada sin salir de la aplicación.

---

## 4. Mejoras introducidas en esta versión

### 4.1. Editor YAML integrado

La aplicación incorpora un editor visual para modificar workflows YAML.

Esto permite ajustar directamente:

* URLs.
* Selectores.
* IDs UI5.
* Datos de entrada.
* Timeouts.
* Reintentos.
* Validaciones.
* Capturas.
* Nombres de pasos.
* Variables.

Ejemplo:

```yaml
name: "Consulta Fiori de material"
version: "1.0"
environment: "dev"

variables:
  material: "4500001234"

defaults:
  timeout_secs: 30
  retry:
    attempts: 3
    delay_ms: 800

steps:
  - action: goto
    name: "Abrir aplicación"
    url: "https://fiori.example.com/sap/bc/ui2/flp"

  - action: wait_ui5
    name: "Esperar carga SAPUI5"
    timeout_secs: 90

  - action: input
    name: "Introducir material"
    selector: "[id$='--materialInput']"
    value: "${material}"

  - action: click
    name: "Ejecutar búsqueda"
    selector: "[id$='--searchButton']"

  - action: assert_visible
    name: "Validar tabla de resultados"
    selector: "[id$='--resultTable']"

  - action: screenshot
    name: "Captura final"
    save_as: "99_resultado_final.png"
```

---

### 4.2. Validación de workflows

Antes de ejecutar, el workflow puede validarse desde la interfaz.

La validación revisa:

* Sintaxis YAML.
* Existencia de `steps`.
* Acciones soportadas.
* Pasos sin selector cuando lo necesitan.
* Pasos sin URL cuando son de navegación.
* Selectores de alto riesgo.
* IDs aparentemente generados.
* Falta de capturas o validaciones.
* Posibles problemas de mantenibilidad.

La API interna utilizada es:

```text
POST /api/workflows/validate
```

---

### 4.3. Ejecución completa

Desde la pantalla Workflow se puede ejecutar todo el flujo definido.

La API interna utilizada es:

```text
POST /api/workflows/run
```

Ejemplo conceptual de payload:

```json
{
  "yaml": "name: ...\nsteps: ...",
  "until_step": null
}
```

---

### 4.4. Ejecución hasta un paso seleccionado

La pantalla permite seleccionar un paso y ejecutar el workflow solo hasta ese punto.

Esto resulta útil para:

* Verificar progresivamente.
* Depurar selectores.
* Evitar ejecutar acciones sensibles.
* Validar la navegación antes de modificar datos.
* Comprobar condiciones intermedias.
* Construir workflows de forma segura.

Ejemplo conceptual:

```json
{
  "yaml": "name: ...\nsteps: ...",
  "until_step": 4
}
```

---

### 4.5. Previsualización de pasos

El laboratorio muestra una lista estructurada de pasos detectados, incluyendo:

* Número de paso.
* Acción.
* Nombre descriptivo.
* Selector.
* `control_id`.
* Timeout.
* Riesgo estimado.
* Avisos.

Esto facilita que una persona sin experiencia avanzada pueda entender qué hará el workflow antes de ejecutarlo.

---

### 4.6. Detección de selectores frágiles

La herramienta avisa cuando detecta selectores poco recomendables, como:

```css
#__button0
#__input1
button
div
.sapMBtn
.sapMInputBaseInner
```

Y recomienda usar selectores más estables, como:

```css
[id$='--searchButton']
[id$='--materialInput']
button[aria-label='Buscar']
input[aria-label='Material']
[title='Ejecutar']
```

---

### 4.7. Evidencias de ejecución

Cada ejecución puede generar evidencias para revisión posterior.

Ejemplo de estructura:

```text
runs/
└── ejecucion-fiori/
    ├── execution_report.json
    ├── 01_estado_inicial.json
    ├── 99_resultado_final.json
    ├── 99_resultado_final.png
    └── evidence/
```

Esto permite:

* Auditar la ejecución.
* Revisar qué ocurrió.
* Comparar estados.
* Documentar errores.
* Entregar evidencias funcionales o técnicas.

---

## 5. Arquitectura del proyecto

```text
fiori-inspector-studio-rs/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── INSTALAR_Y_USAR.md
├── config/
│   └── local.toml.example
├── docs/
│   ├── arquitectura.md
│   ├── arquitectura_interactiva.md
│   ├── automatizacion_productiva.md
│   └── workflow_lab.md
├── examples/
│   └── static_fiori_fragment.html
├── workflows/
│   ├── fiori_sample_workflow.yaml
│   └── production_template.yaml
├── static/
│   ├── index.html
│   ├── styles.css
│   └── app.js
└── src/
    ├── automation.rs
    ├── browser.rs
    ├── config.rs
    ├── lib.rs
    ├── main.rs
    ├── models.rs
    ├── report.rs
    ├── static_html.rs
    ├── studio.rs
    ├── workflow.rs
    └── js/
        └── ui5_probe.js
```

---

## 6. Stack técnico

El proyecto utiliza:

* **Rust** como lenguaje principal.
* **Tokio** para ejecución asíncrona.
* **Axum** para servidor web local.
* **Chrome DevTools Protocol** para interacción con Chrome/Chromium.
* **Reqwest** para comunicación HTTP.
* **Serde** para serialización.
* **Serde JSON** para snapshots e informes.
* **Serde YAML** para workflows.
* **TOML** para configuración.
* **Scraper** para análisis HTML estático.
* **HTML / CSS / JavaScript** para la interfaz.
* **YAML** como lenguaje declarativo de automatización.

---

## 7. Requisitos

### Requisitos básicos

* Rust estable.
* Cargo.
* Google Chrome o Chromium.
* Linux, Windows o macOS.

### Requisitos recomendados

* Ubuntu 24.04 o superior.
* Chromium o Google Chrome actualizado.
* Acceso autorizado al entorno SAP Fiori.
* Permisos para analizar o automatizar la aplicación.
* Entorno de desarrollo o pruebas para validación inicial.

---

## 8. Instalación

Clonar el repositorio:

```bash
git clone https://github.com/ansonTGN/Fiori-Inspector.git
cd Fiori-Inspector
```

Compilar:

```bash
cargo build
```

Ejecutar:

```bash
cargo run
```

Abrir en el navegador:

```text
http://127.0.0.1:7820
```

---

## 9. Configuración

Copiar configuración de ejemplo:

```bash
cp config/local.toml.example config/local.toml
```

Ejemplo:

```toml
[server]
bind = "127.0.0.1:7820"

[browser]
cdp_url = "http://127.0.0.1:9222"
chrome_binary = "google-chrome"
auto_launch = true
headless = false
accept_insecure_certs = true
window_width = 1600
window_height = 1000
user_data_dir = "./.browser-profile-cdp"
```

Si se usa Chromium:

```toml
chrome_binary = "chromium"
```

o:

```toml
chrome_binary = "chromium-browser"
```

---

## 10. Uso básico

### 10.1. Arrancar el estudio

```bash
cargo run
```

o:

```bash
cargo run -- serve
```

Abrir:

```text
http://127.0.0.1:7820
```

---

### 10.2. Arrancar Chrome manualmente con CDP

Si se prefiere no usar lanzamiento automático:

```bash
google-chrome \
  --remote-debugging-port=9222 \
  --user-data-dir=./.browser-profile-cdp \
  --window-size=1600,1000
```

Con Chromium:

```bash
chromium \
  --remote-debugging-port=9222 \
  --user-data-dir=./.browser-profile-cdp \
  --window-size=1600,1000
```

Verificar CDP:

```bash
curl http://127.0.0.1:9222/json/version
```

---

### 10.3. Analizar HTML estático

```bash
cargo run -- analyze-html \
  --input examples/static_fiori_fragment.html \
  --output runs/static_snapshot.json
```

---

### 10.4. Capturar snapshot de Fiori en vivo

```bash
cargo run -- snapshot-cdp \
  --url "https://tu-servidor-fiori/sap/bc/ui2/flp" \
  --output runs/fiori_snapshot.json
```

---

### 10.5. Ver resumen

```bash
cargo run -- summary \
  --input runs/fiori_snapshot.json
```

---

### 10.6. Ver árbol de controles

```bash
cargo run -- tree \
  --input runs/fiori_snapshot.json \
  --max-depth 6
```

---

### 10.7. Listar acciones

```bash
cargo run -- actions \
  --input runs/fiori_snapshot.json
```

Filtrar acciones:

```bash
cargo run -- actions \
  --input runs/fiori_snapshot.json \
  --contains material
```

---

### 10.8. Ejecutar workflow desde CLI

```bash
cargo run -- run-workflow \
  --workflow workflows/production_template.yaml \
  --output-dir runs/test-productivo
```

---

## 11. Uso del Laboratorio de Workflows

Flujo recomendado desde la interfaz:

1. Abrir la aplicación.
2. Analizar una página Fiori o un HTML de ejemplo.
3. Ir a la pantalla **Workflow**.
4. Revisar el YAML generado.
5. Editar pasos, datos, selectores o validaciones.
6. Pulsar **Validar**.
7. Revisar errores y avisos.
8. Seleccionar un paso si se quiere ejecutar parcialmente.
9. Pulsar **Ejecutar hasta paso seleccionado**.
10. Revisar resultado.
11. Repetir hasta que el flujo sea estable.
12. Ejecutar workflow completo.
13. Revisar evidencias generadas.

---

## 12. Acciones soportadas en workflows

Las acciones soportadas pueden incluir:

```text
goto
wait_ui5
wait
wait_for
click
input
select
press
snapshot
screenshot
assert_visible
assert_text
```

Ejemplo:

```yaml
steps:
  - action: goto
    url: "https://fiori.example.com/sap/bc/ui2/flp"

  - action: wait_ui5
    timeout_secs: 90

  - action: input
    name: "Introducir material"
    selector: "[id$='--materialInput']"
    value: "${material}"

  - action: click
    name: "Buscar"
    selector: "[id$='--searchButton']"

  - action: assert_visible
    name: "Validar resultados"
    selector: "[id$='--resultTable']"

  - action: screenshot
    save_as: "resultado.png"
```

---

## 13. Estrategia profesional de automatización

Prioridad recomendada:

1. **API / OData**, si existe y está autorizado.
2. **Control SAPUI5**, si se puede identificar un `control_id`.
3. **Selector semántico estable**, como `aria-label`, `title` o sufijo UI5.
4. **Selector DOM específico**, solo si no hay alternativa.
5. **Nunca coordenadas de pantalla como estrategia principal.**

---

## 14. Buenas prácticas con datos

No se recomienda incrustar datos productivos directamente en pasos.

No recomendado:

```yaml
- action: input
  selector: "[id$='--materialInput']"
  value: "4500001234"
```

Recomendado:

```yaml
variables:
  material: "4500001234"

steps:
  - action: input
    selector: "[id$='--materialInput']"
    value: "${material}"
```

Esto permite:

* Reutilizar workflows.
* Cambiar datos sin tocar la lógica.
* Preparar ejecución por lotes.
* Separar datos y proceso.
* Versionar workflows sin exponer información sensible.

---

## 15. Evidencias y control de calidad

Una automatización profesional debe generar evidencias.

Ejemplo:

```yaml
- action: snapshot
  name: "Captura estado inicial"
  save_as: "01_estado_inicial.json"

- action: screenshot
  name: "Captura visual final"
  save_as: "99_resultado_final.png"
```

También se recomienda añadir validaciones:

```yaml
- action: assert_visible
  name: "Validar tabla de resultados"
  selector: "[id$='--resultTable']"
  timeout_secs: 30
```

---

## 16. Seguridad

La herramienta debe utilizarse solo en entornos autorizados.

Reglas importantes:

* No subir contraseñas al repositorio.
* No subir cookies.
* No subir tokens.
* No subir capturas con datos sensibles.
* No subir `config/local.toml` si contiene URLs internas.
* No subir carpetas `runs/` con evidencias productivas.
* Mantener el servidor local en `127.0.0.1`.
* No exponer el estudio a internet.
* No saltarse controles de acceso.
* Respetar las políticas corporativas de ciberseguridad.
* Validar primero en desarrollo o calidad.

`.gitignore` recomendado:

```gitignore
/target/
.env
.env.*
config/local.toml
runs/
*.log
*.tmp
.browser-profile-cdp/
.vscode/
.idea/
.DS_Store
Thumbs.db
```

---

## 17. Limitaciones

La herramienta no sustituye a:

* SAP Gateway.
* Servicios OData oficiales.
* SAP Business Application Studio.
* SAP Fiori Tools.
* Herramientas corporativas de testing.
* Gobierno funcional SAP.

Limitaciones conocidas:

* El HTML estático no conserva todo el runtime SAPUI5.
* Algunas aplicaciones usan IDs generados.
* Algunos controles personalizados requieren heurísticas específicas.
* SSO corporativo puede requerir login manual.
* CDP necesita Chrome o Chromium local.
* La automatización UI debe validarse cuidadosamente antes de operar con datos reales.
* No todas las reglas de negocio pueden inferirse desde la interfaz.

---

## 18. Casos de uso

Casos de uso profesionales:

* Analizar una aplicación Fiori antes de automatizarla.
* Documentar pantallas Fiori.
* Identificar controles UI5.
* Generar workflows de prueba.
* Crear evidencias de ejecución.
* Validar selectores.
* Preparar automatizaciones repetibles.
* Formar equipos técnicos.
* Comparar comportamiento entre entornos.
* Construir flujos previos a una integración OData.

---

## 19. Roadmap

Posibles mejoras futuras:

* Grabador visual de workflows.
* Ejecución paso a paso interactiva con pausa entre pasos.
* Gestión de variables desde tabla visual.
* Lectura de datos desde CSV, JSON o Excel.
* Ejecución por lotes.
* Reporte HTML de ejecución.
* Exportación PDF de evidencias.
* Comparación entre snapshots.
* Correlación con metadatos OData.
* Exportación a Playwright.
* Exportación a Selenium.
* Generación de código Rust.
* Dashboard histórico de ejecuciones.
* Perfiles por entorno: DEV, QA, PRE, PROD.
* Control de aprobaciones para pasos sensibles.

---

## 20. Comandos de desarrollo

Compilar:

```bash
cargo build
```

Ejecutar:

```bash
cargo run
```

Ejecutar tests:

```bash
cargo test
```

Formatear:

```bash
cargo fmt
```

Lint:

```bash
cargo clippy --all-targets -- -D warnings
```

Generar documentación:

```bash
cargo doc --open
```

---

## 21. Publicación en GitHub

Repositorio sugerido:

```text
https://github.com/ansonTGN/Fiori-Inspector
```

Comandos habituales:

```bash
git add .
git commit -m "Add workflow editor, validation and step execution"
git push
```

Si el repositorio todavía no existe:

```bash
gh repo create ansonTGN/Fiori-Inspector --private --source=. --remote=origin --push
```

---

## 22. Autor

**Angel A. Urbina**

Ingeniero Industrial, profesional de Data Science y Ciberseguridad, con experiencia en SAP, Rust, Python, automatización, sistemas industriales, análisis de procesos empresariales y desarrollo de herramientas técnicas para entornos corporativos.

GitHub:

```text
https://github.com/ansonTGN
```

---

## 23. Licencia

Licencia sugerida:

```text
MIT License
Copyright (c) Angel A. Urbina
```

---

## 24. Aviso legal

Este proyecto es una herramienta independiente para análisis, documentación y soporte a la automatización de aplicaciones SAP Fiori / SAPUI5.

No es un producto oficial de SAP.

SAP, SAP Fiori, SAPUI5 y otros nombres relacionados son marcas comerciales o marcas registradas de SAP SE o sus afiliadas.

Utiliza esta herramienta de forma responsable y únicamente en sistemas donde tengas autorización expresa para analizar, probar o automatizar.

