---
name: hera
description: Capa inteligente para descubrir, compartir, adquirir, analizar y organizar música mediante ecosistemas P2P y fuentes autorizadas.
---

# Objetivo

Convertir intención musical en búsquedas federadas, candidatos comparables, análisis reproducible y crates DJ exportables mediante el servidor MCP de Hera.

# Reglas obligatorias para el agente

- **Fase 1 Anti-Duplicados:** Antes de iniciar cualquier descarga (`download_track`), validar contra el repertorio local con `audit_repertoire(check_queries=[...])` (fuzzy matching `>= 85%`) para omitir pistas existentes (`[SKIP - YA EXISTE]`).
- **Promoción Local In-Place:** Para pistas locales (`provider="local"`, `cand_*` o rutas en disco), invocar directamente `analyze_track`, `analyze_batch` o `identify_track` sin pasar por `download_track` (se promueven *in-place* a `trk_*` sin duplicar en `quarantine/`).
- Confirmar la base de autorización antes de `download_track` en fuentes externas.
- Solicitar aprobación humana antes de adquirir, borrar, sobrescribir o compartir.
- Nunca presentar Soulseek o BitTorrent como anónimos o inherentemente privados.
- Usar `get_track_candidates` antes de seleccionar una fuente externa.
- Explicar el score y pedir revisión si la identidad es ambigua (`review_required: true`).
- Nunca inventar IDs de tools (`candidate_id`, `job_id`, `track_id`).

# Flujos canónicos

- **Modo Cloudflare Code Mode (`hera_code_mode` — Recomendado para control total en 1 solo turno):**
  En lugar de encadenar múltiples llamadas MCP una por una, los agentes pueden invocar `get_code_mode_sdk` (o `GET /mcp/code-mode.d.ts`) y ejecutar un bloque TypeScript completo sobre el namespace `hera.*` mediante `hera_code_mode` (`POST /mcp/code-mode`):
  ```ts
  const tracks = await hera.searchMusic({ camelot: "8A", bpm_min: 120, bpm_max: 130 });
  const clean = tracks.filter(t => t.quality_score >= 95 && t.mono_compatible_sub);
  const crate = await hera.buildDjCrate({ brief: "Peak Hour Set", energy_arc: "peak_hour", candidates: clean });
  return { selected_count: clean.length, synergy: crate.mean_synergy_score, rekordbox_xml: crate.rekordbox_xml_export };
  ```
- **Auditoría e Inteligencia de Repertorio (Audio + Video-DJ HD + Producciones Propias):**
  `audit_repertoire(sync_db=True, check_queries=[...])` → `analyze_batch(track_ids=[...], genre_hint=...)` → `build_dj_crate(..., export=["rekordbox_xml", "ableton", "cue_sheet", "m3u8"])`
- **Adquisición Externa Autorizada:**
  `audit_repertoire(check_queries=[...])` → `search_music` → `get_track_candidates` → aprobación → `download_track` → `download_status` → `identify_track` → `analyze_track` → `organize_track` → `build_dj_crate`

# Conexión con Agentes Gratuitos (Gemini CLI / Google AI Studio / MCP)

- **Binario Nativo Rust (`stdio`):** `hera-node --mcp`
- **Endpoint MCP HTTP / Cloudflare Workers:** `POST /mcp` (`GET /mcp/tools`)
- **Especificación OpenAPI 3.1.0 (Google AI Studio / Actions):** `GET /openapi.json`

# Referencias

- Consultar `references/tools.md` para el esquema detallado de parámetros y salidas.
- Consultar `references/policies.md` para bases de autorización permitidas y reglas legales.
- Consultar `references/workflows.md` para ejemplos conversacionales paso a paso.
