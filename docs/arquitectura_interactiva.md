# Arquitectura interactiva

## Componentes

```text
Navegador del usuario
        │
        ▼
Frontend Apple-like /static
        │ HTTP JSON
        ▼
Backend Rust Axum
        │
        ├── Static HTML Analyzer
        │
        ├── WebDriver Controller
        │       │
        │       ▼
        │   Chrome/Firefox controlado
        │       │
        │       ▼
        │   SAP Fiori / SAPUI5 vivo
        │
        └── Report Builder
```

## Flujo de sesión viva

1. El usuario abre el Studio.
2. Introduce una URL Fiori.
3. El backend Rust conecta con ChromeDriver.
4. Se abre Fiori en un navegador controlado.
5. El backend espera a que SAPUI5 esté inicializado.
6. Se inyecta `ui5_probe.js`.
7. El probe extrae controles, DOM, bindings, modelos y endpoints.
8. Rust convierte el resultado en `PageSnapshot`.
9. Axum devuelve el snapshot al frontend.
10. La interfaz renderiza resumen, árbol, acciones, bindings, riesgos y workflow.

## Capas de extracción

### DOM visible

- Etiquetas HTML.
- IDs.
- Clases.
- Roles ARIA.
- Textos visibles.
- Selectores CSS candidatos.

### UI5 lógico

- `sap.ui.getCore()`.
- Controles registrados.
- Tipos de control.
- Parent/children.
- Agregaciones.
- Bindings.
- Modelos.
- Context paths.

### Integración

- Performance entries.
- Service URLs de modelos.
- Rutas OData detectadas en HTML.

## Criterios de calidad

La puntuación de calidad se calcula considerando:

- UI5 detectado.
- Cantidad de controles.
- Acciones candidatas.
- Endpoints OData.
- Bindings/context paths.
- Riesgos de IDs dinámicos.
- Captura incompleta o HTML estático.

## Estrategia de automatización recomendada

1. Usar Studio para descubrir estructura.
2. Identificar acciones robustas.
3. Preferir llamadas OData cuando el endpoint sea claro.
4. Usar selectores UI solo cuando la API no cubra el caso.
5. Versionar workflows YAML.
6. Ejecutar snapshots antes y después de cada acción importante.
7. Añadir verificaciones de negocio, no solo verificaciones visuales.
