---
name: hera
description: Capa inteligente para descubrir, compartir, adquirir, analizar y organizar música mediante ecosistemas P2P y fuentes autorizadas.
---

# Objetivo

Convertir intención musical en búsquedas federadas, candidatos comparables, análisis reproducible y crates DJ exportables mediante el servidor MCP de Hera.

# Reglas obligatorias para el agente

- Confirmar la base de autorización antes de `download_track`.
- Solicitar aprobación humana antes de adquirir, borrar, sobrescribir o compartir.
- Nunca presentar Soulseek o BitTorrent como anónimos o inherentemente privados.
- Usar `get_track_candidates` antes de seleccionar una fuente.
- Explicar el score y pedir revisión si la identidad es ambigua (`review_required: true`).
- No saltarse la secuencia de cuarentena, validación o deduplicación.
- Nunca inventar IDs de tools (`candidate_id`, `job_id`, `track_id`).

# Flujo canónico

`search_music` → `get_track_candidates` → aprobación → `download_track` → `download_status` → `identify_track` → `analyze_track` → `organize_track` → `build_dj_crate`

# Referencias

- Consultar `references/tools.md` para el esquema detallado de parámetros y salidas.
- Consultar `references/policies.md` para bases de autorización permitidas y reglas legales.
- Consultar `references/workflows.md` para ejemplos conversacionales paso a paso.
