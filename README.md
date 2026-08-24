# Hera

Base de conocimiento inicial del proyecto Hera.

Hera es una capa inteligente universal para descubrir, compartir, adquirir,
analizar y organizar música mediante ecosistemas P2P y otras fuentes
autorizadas, usando IA local o incluida en hosts como Codex y Antigravity.

## Empezar aquí

1. Producto, arquitectura e implementación:
   [`docs/Hera_Especificacion_Tecnica_PRD_v0.1.docx`](docs/Hera_Especificacion_Tecnica_PRD_v0.1.docx)
2. Comportamiento de agentes, tools MCP y guardrails:
   [`docs/Hera_Guia_para_Agentes_AI.md`](docs/Hera_Guia_para_Agentes_AI.md)
3. Índice y reglas de mantenimiento:
   [`docs/README.md`](docs/README.md)

## Principios del proyecto

- Local-first y sin dependencia obligatoria de una API de IA de pago.
- Provider-agnostic: integrar componentes maduros detrás de contratos estables.
- Uso exclusivo de contenido que el usuario esté autorizado a obtener y procesar.
- Autorización y aprobación antes de efectos externos.
- Descargas aisladas en cuarentena antes de validar y organizar.
- Ranking explicable, procedencia auditable y control del DJ.
- Skills y contratos MCP portables entre hosts de agentes.
- Herramienta single-user local: Python, SQLite, archivos y MCP por `stdio`.
- Sin Kubernetes, Redis, Kafka, microservicios, billing ni APIs de IA de pago.
- Docker, HTTP local, Ollama y providers de red son complementos opcionales.

## Estructura actual

```text
hera/
├── README.md
└── docs/
    ├── README.md
    ├── Hera_Especificacion_Tecnica_PRD_v0.1.docx
    └── Hera_Guia_para_Agentes_AI.md
```

## Estructura objetivo del proyecto

```text
hera/
├── README.md
├── docs/
│   ├── architecture/
│   ├── product/
│   ├── operations/
│   ├── security/
│   └── decisions/
├── skills/hera/
├── src/hera/
│   ├── contracts/
│   ├── domain/
│   ├── policy/
│   ├── jobs/
│   └── mcp/
├── providers/
├── analyzers/
├── deploy/                    # Docker/NAS opcional
└── tests/
```

La estructura objetivo se creará gradualmente cuando comience la
implementación. Esta carpeta sirve por ahora como fuente inicial de verdad.
