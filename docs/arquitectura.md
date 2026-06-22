# Arquitectura de Fiori Inspector Studio sin ChromeDriver

## Objetivo

Construir una herramienta Rust que permita inspeccionar aplicaciones SAP Fiori de forma análoga a SAP GUI Scripting, pero adaptada a la arquitectura web dinámica de SAPUI5 y sin depender de ChromeDriver.

## Capas

```text
┌──────────────────────────────────────────────┐
│ Workflow YAML / CLI / Studio Web             │
├──────────────────────────────────────────────┤
│ Orquestador Rust                             │
├──────────────────────────────────────────────┤
│ Controlador CDP                              │
├──────────────────────────────────────────────┤
│ Chrome / Chromium con --remote-debugging     │
├──────────────────────────────────────────────┤
│ Extractor JS inyectado en navegador          │
├──────────────────────────────────────────────┤
│ Runtime SAPUI5: Element registry / Core      │
├──────────────────────────────────────────────┤
│ DOM HTML renderizado                         │
├──────────────────────────────────────────────┤
│ Fiori app / SAP Gateway / OData              │
└──────────────────────────────────────────────┘
```

## Estrategia de extracción

1. Lanzar o conectar con Chrome/Chromium vía CDP en `http://127.0.0.1:9222`.
2. Crear una pestaña nueva mediante `Target.createTarget`.
3. Navegar a la URL Fiori mediante `Page.navigate`.
4. Esperar `document.readyState` y, opcionalmente, `sap.ui.getCore().isInitialized()`.
5. Ejecutar `ui5_probe.js` con `Runtime.evaluate`.
6. Extraer controles UI5, DOM, bindings, modelos y endpoints.
7. Convertir el JSON devuelto en `PageSnapshot` Rust.
8. Presentar el resultado en Studio, CLI o workflow YAML.

## Ventajas frente a ChromeDriver

- No requiere instalar ni mantener `chromedriver`.
- Evita incompatibilidades frecuentes entre navegador y driver.
- Reduce dependencias operativas.
- Se conecta a Chrome/Chromium usando una interfaz nativa del navegador.
- Permite análisis vivo de SAPUI5 sin tratar la página solo como HTML estático.

## Limitaciones

- Requiere que Chrome/Chromium permita depuración remota local.
- Algunas políticas corporativas pueden restringir CDP.
- La interacción DOM directa puede no ser suficiente para todos los controles SAPUI5 complejos.
- Para operaciones críticas se recomienda preferir OData/API cuando sea posible.
