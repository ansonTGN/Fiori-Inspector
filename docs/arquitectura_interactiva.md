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
        ├── CDP Controller
        │       │
        │       ▼
        │   Chrome/Chromium con remote debugging
        │       │
        │       ▼
        │   SAP Fiori / SAPUI5 vivo
        │
        └── Report Builder
```

## Flujo de sesión viva sin ChromeDriver

1. El usuario abre el Studio.
2. Introduce una URL Fiori.
3. El backend Rust comprueba CDP en `http://127.0.0.1:9222`.
4. Si CDP no está activo y `auto_launch = true`, lanza Chrome/Chromium automáticamente.
5. El backend crea una pestaña CDP y navega a Fiori.
6. El usuario completa el login/SSO si es necesario.
7. El backend espera a SAPUI5.
8. Se inyecta `ui5_probe.js` mediante `Runtime.evaluate`.
9. El probe extrae controles, DOM, bindings, modelos y endpoints.
10. Rust convierte el resultado en `PageSnapshot`.
11. Axum devuelve el snapshot al frontend.
12. La interfaz renderiza resumen, árbol, acciones, bindings, riesgos y workflow.

## Modos de análisis

- **CDP vivo**: máxima calidad, porque ve SAPUI5 ejecutándose.
- **HTML estático**: útil para documentación, formación o evidencias offline.

## Seguridad

El servidor local debe permanecer en `127.0.0.1`. El puerto CDP también debe limitarse al equipo local. No debe exponerse CDP a una red corporativa o pública.
