# Arquitectura de fiori-dom-agent-rs

## Objetivo

Construir una herramienta Rust que permita inspeccionar e interactuar con aplicaciones SAP Fiori de forma análoga a SAP GUI Scripting, pero respetando la arquitectura web de SAPUI5.

## Capas

```text
┌──────────────────────────────────────────────┐
│ Workflow YAML / CLI                          │
├──────────────────────────────────────────────┤
│ Orquestador Rust                             │
├──────────────────────────────────────────────┤
│ WebDriver: Chrome / Firefox                  │
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

1. Esperar a que `sap.ui.getCore().isInitialized()` sea verdadero.
2. Leer controles desde `sap.ui.core.Element.registry.all()` cuando esté disponible.
3. Usar fallback hacia estructuras del core si la versión UI5 es antigua.
4. Para cada control:
   - ID lógico.
   - Tipo UI5.
   - Estado visible/habilitado/editable.
   - Texto/valor/título/tooltip.
   - DOM asociado.
   - Aggregations y relación padre/hijo.
   - Bindings de propiedades.
   - Modelos y posibles `service_url`.
   - Selectores candidatos.
   - Tipo de interacción probable.
5. Leer recursos de red con `performance.getEntriesByType('resource')` para localizar URLs OData.

## Filosofía de automatización

SAP Fiori no debe automatizarse como una web cualquiera. La capa DOM es una representación final generada por SAPUI5. Para flujos robustos hay que combinar:

- árbol lógico UI5,
- atributos ARIA,
- IDs estables,
- bindings,
- servicios OData,
- evidencia de ejecución.

## Seguridad

- No se guardan credenciales.
- No se interceptan contraseñas.
- No se saltan controles de autorización.
- Se recomienda ejecutar primero en entornos DEV/QAS.
- Toda acción productiva debería tener registro de auditoría.
