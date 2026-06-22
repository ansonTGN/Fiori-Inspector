# Laboratorio interactivo de workflows

Esta versión incorpora una pantalla profesional para convertir el análisis Fiori en automatizaciones operables.

## Objetivo

El laboratorio evita que el workflow sea una caja negra. Permite editar el YAML, validarlo, revisar cada paso, ejecutar el flujo completo o verificarlo incrementalmente hasta un paso concreto.

## Capacidades

- Editor YAML integrado en la pantalla principal.
- Copia directa del workflow.
- Descarga del YAML editado.
- Restauración del workflow generado desde el snapshot.
- Validación del YAML en backend Rust.
- Previsualización de pasos.
- Detección de errores obligatorios: `url`, `selector`, `control_id`, `value`, `save_as`, etc.
- Avisos sobre selectores frágiles como `#__button0`.
- Ejecución completa del workflow.
- Ejecución incremental hasta un paso seleccionado.
- Informe visual del último lanzamiento.
- Lectura de `execution_report.json` generado por el motor productivo.

## Uso recomendado

1. Captura una pantalla Fiori real o analiza HTML de ejemplo.
2. Abre el panel **Workflow**.
3. Revisa el YAML generado.
4. Sustituye valores de ejemplo por variables reales.
5. Pulsa **Validar**.
6. Revisa los pasos y riesgos.
7. Selecciona el primer paso y pulsa **Ejecutar hasta paso seleccionado**.
8. Repite avanzando paso a paso.
9. Cuando todos los pasos sean estables, pulsa **Ejecutar completo**.
10. Revisa evidencias e informe de ejecución.

## Estrategia profesional

- Para acciones críticas, prioriza OData/API siempre que sea posible.
- Para UI, prioriza `control_id` SAPUI5.
- Si usas DOM, prefiere selectores semánticos: `aria-label`, `title`, stable IDs o sufijos UI5.
- Evita IDs generados como `#__button0` salvo en simulaciones.
- Añade `snapshot` y `screenshot` en puntos clave.
- No guardes credenciales ni datos sensibles en YAML.
