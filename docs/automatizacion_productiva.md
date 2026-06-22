# Automatización productiva de SAP Fiori

Este documento describe el enfoque recomendado para usar Fiori Inspector Studio en entornos reales.

## Principios

1. No automatizar por coordenadas.
2. No depender de IDs generados como `#__button0` salvo para prototipos.
3. Priorizar `control_id` UI5 estable, sufijos `[id$='--...']`, `aria-label` y validaciones.
4. Añadir `assert` después de acciones críticas.
5. Guardar evidencias estructuradas (`snapshot`) y visuales (`screenshot`).
6. Parametrizar datos mediante `variables` y `{{vars.nombre}}`.
7. No guardar contraseñas en YAML.
8. Ejecutar primero en DEV/QAS.

## Flujo recomendado

1. Captura la pantalla con CDP.
2. Revisa Acciones y Riesgos.
3. Descarga el workflow YAML generado.
4. Sustituye valores de ejemplo.
5. Cambia selectores débiles por `control_id` o selector recomendado.
6. Añade validaciones de negocio.
7. Ejecuta el workflow con `run-workflow`.
8. Revisa `execution_report.json` y evidencias.

## Targeting

Cada paso de interacción puede identificar el objetivo por:

```yaml
control_id: "application-ZAPP---Main--searchButton"
```

O por selector:

```yaml
selector: "[id$='--searchButton']"
```

O por texto como último recurso:

```yaml
text: "Buscar"
```

## Ejecución

```bash
cargo run -- run-workflow \
  --workflow workflows/production_template.yaml \
  --output-dir runs/ejecucion_001
```

## Evidencias

El motor genera:

- `execution_report.json`
- snapshots JSON
- capturas PNG
- evidencias automáticas ante fallos

Esto facilita auditoría, diagnóstico y mejora continua.
