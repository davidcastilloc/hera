# Base de conocimiento de Hera

Este directorio reúne la documentación canónica disponible antes de comenzar
la implementación del producto.

## Documentos canónicos

| Documento | Audiencia | Contenido |
|---|---|---|
| `Hera_Especificacion_Tecnica_PRD_v0.1.docx` | Producto, backend, P2P, MCP, DevOps y seguridad | Visión, arquitectura, providers, pipeline, datos, despliegue, roadmap, riesgos y aceptación del MVP |
| `Hera_Guia_para_Agentes_AI.md` | Agentes de IA y desarrolladores de Skills/MCP | Reglas operativas, tools, secuencias, estados, errores, seguridad, ejemplos y evals |

## Jerarquía de autoridad

Cuando exista una discrepancia:

1. Las políticas legales y de seguridad más restrictivas tienen prioridad.
2. Los contratos JSON Schema implementados tendrán prioridad sobre ejemplos
   narrativos cuando empiece el desarrollo.
3. El PRD define intención y alcance del producto.
4. La guía de agentes define el comportamiento del Brain, sin sustituir las
   validaciones de Hera Connect.
5. Las decisiones futuras deben registrarse como ADR y enlazarse desde este
   índice.

## Restricción arquitectónica vigente

Hera es una herramienta local, no un SaaS. El baseline aprobado para el MVP es
Python + Pydantic + SQLite + sistema de archivos + MCP por `stdio`. Docker,
HTTP local, modelos mediante Ollama/llama.cpp y providers de red son opcionales.
No forman parte del núcleo obligatorio PostgreSQL, Redis, Kafka, Kubernetes,
microservicios, multi-tenancy, billing, telemetría cloud ni APIs de IA de pago.

## Convenciones para nuevos documentos

- Usar nombres descriptivos y versión cuando el contenido sea contractual.
- Indicar estado: propuesta, aceptado, obsoleto o reemplazado.
- Registrar fecha, responsable y documentos relacionados.
- No guardar secretos, tokens, credenciales ni información de peers.
- Añadir aquí todo documento considerado fuente de verdad.
- Mantener ejemplos de P2P limitados a material autorizado.

## Próximos documentos recomendados

- Arquitectura C4 y límites de confianza.
- JSON Schemas de las tools MCP.
- Modelo de amenazas y política de autorización.
- ADR de base de datos, cola de jobs y embeddings.
- Especificación de providers y contract tests.
- Runbook de despliegue Docker y recuperación.
- Dataset de fixtures propios o con licencia para pruebas.
- Conversaciones doradas y evaluaciones de agentes.
