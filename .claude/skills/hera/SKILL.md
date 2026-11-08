---
name: hera
description: Capa inteligente para descubrir, compartir, adquirir, analizar y organizar música mediante ecosistemas P2P y fuentes autorizadas.
---

# ⚡ Paso 0 (OBLIGATORIO AL INICIAR): Auto-Verificación e Instalación/Actualización Automática

**REGLA ESTRICTA DE ARRANQUE:** Antes de ejecutar cualquier búsqueda, auditoría, análisis DSP o armado de crates, el agente **DEBE verificar primero si Hera, esta Skill y todos los binarios necesarios (`hera`, `hera-node`, `slskd`, `rclone`, `ffmpeg`, `fpcalc`) están instalados y en su última versión**, actualizándolos automáticamente para que el usuario siempre tenga la versión más reciente:

1. **Vía Tool MCP (Preferido si el servidor MCP ya responde):**
   Invoca `ensure_hera_updated(auto_compile_rust=True, sync_skills=True)` al comenzar la sesión.
2. **Vía Terminal (Multiplataforma — Windows / Linux / macOS / Raspberry Pi):**
   - Si el repositorio ya existe localmente:
     ```bash
     python scripts/ensure_latest_hera.py
     ```
     *(Verifica `git fetch/pull` preservando cambios locales, recompila automáticamente `hera-rs` -> `bin/hera-node(.exe)` si hay commits nuevos en Rust, actualiza `.venv` y sincroniza `skills/hera` y `skills/hera-dj-set-curator` hacia `~/.gemini/config/skills/`, `~/.claude/skills/` y `~/.agents/skills/`).*
   - Si Hera **no está instalado** aún en la máquina:
     - **Linux / macOS / Raspberry Pi:** `curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/install.sh | bash` (o `./install.sh`)
     - **Windows (PowerShell):** `powershell -ExecutionPolicy Bypass -File .\install.ps1`

---

# Objetivo

Convertir intención musical en búsquedas federadas, candidatos comparables, análisis reproducible (DSP forense + estructura de 4 Hot Cues + SimHash 64-bit POPCNT) y crates DJ exportables mediante el servidor MCP híbrido (Python + Rust `hera-node`) de Hera.

# Reglas obligatorias para el agente

- **Paso 0 Auto-Update:** Ejecutar siempre primero `ensure_hera_updated` o `scripts/ensure_latest_hera.py` para garantizar que las skills y los binarios (`hera` y `hera-node`) estén al día.
- **Fase 1 Anti-Duplicados:** Antes de iniciar cualquier descarga (`download_track`), validar contra el repertorio local con `audit_repertoire(check_queries=[...])` (fuzzy matching `>= 85%`) y deduplicación global CAS (`content_sha256` / `u64` SimHash) para omitir pistas existentes (`[SKIP - YA EXISTE]`).
- **Promoción Local In-Place:** Para pistas locales (`provider="local"`, `cand_*` o rutas en disco), invocar directamente `analyze_track`, `analyze_batch` o `identify_track` sin pasar por `download_track` (se promueven *in-place* a `trk_*` sin duplicar en `quarantine/`).
- **Calibración Acústica de BPM Exacto:** En proyectos propios con archivo `.als` (Ableton Live), extraer el `<Tempo>` exacto del proyecto DAW como fuente de verdad; para pistas sin DAW, aplicar corrección de octava Half/Double-Time por género y desempate armónico antes de exportar a VirtualDJ/Rekordbox.
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
- **Onboarding Zero-Tech para DJs vía Google Drive / Cloudflare R2 (`connect_dj_drive` / `hera.connectDjDrive`):**
  Un DJ no técnico (ej. `DJFoo`) no necesita desplegar servidores ni buckets propios: solo pega su link compartido de Google Drive (`https://drive.google.com/drive/folders/...`) usando `connect_dj_drive` (o `await hera.connectDjDrive(...)` en Code Mode). El nodo central de `DaviDJ` extrae el `folder_id` / `file_id`, aplica la auditoría forense Zero-Trust (rechazando falsos lossless `<16 kHz` o tonos sintéticos en cuarentena), deduplica mediante CAS global (`content_sha256`) y publica únicamente los masters verificados con links de descarga directa (`https://drive.google.com/uc?export=download&id=...`) consultables en `GET /api/v1/p2p/stream/:track_id`.
- **Internal P2P Data Broker & SimHash 64-bit POPCNT en RAM (`/api/v1/broker/*`):**
  - Búsqueda instantánea de similitud acústica en RAM mediante distancia de Hamming (`(simhash_a ^ simhash_b).count_ones()`) sobre embeddings de 64 dimensiones (`POST /api/v1/broker/similar`).
  - Resolución de puentes armónicos de emergencia (`POST /api/v1/broker/resolve-bridge`), telemetría de enjambre y tiering automático Hot/Cold en Cloudflare R2 (`POST /api/v1/broker/optimize-tiering`).
- **Enjambre R2 Federado & Nodos `Edge Guardian` (`/api/v1/swarm/*` y `hera-rs/src/p2p_exchange.rs`):**
  - *Lema:* «Mientras más nodos de Hera, más fuerte es el sistema de enjambres.»
  - **Constrained Water-Filling (`get_swarm_strength` / `hera.getSwarmStrength`):** Reparte fragmentos inmutables de `4 MB` (`shard-blobs/{hmac}.hbin`) entre Workers comunitarios saludables respetando estrictamente los techos seguros de `9,000 MB` R2 (`CF_WORKER_SAFE_R2_CAP_MB`) y `85,000 req/día` (`CF_WORKER_SAFE_REQ_CAP`) para costo `$0.00` en Cloudflare Free Tier.
  - **Balanceo `Power-of-Two-Choices (P2C)` y Tickets de `60s` (`swarm_route_ticket` / `hera.swarmRouteTicket`):** Enruta descargas al nodo saludable con menor carga y emite un ticket `X-Hera-Swarm-Ticket` firmado con HMAC-SHA256 de un solo uso (`TTL <= 60s`), acreditando `1.5 MB` de Crédito Turbo por cada `1.0 MB` servido (`SWARM_TURBO_CREDIT_MULTIPLIER = 1.5`).
  - **Auditoría `Proof-of-Custody` de 64 Bytes y Cuarentena Anti-Poisoning (`swarm_custody_audit` / `hera.swarmCustodyAudit`):** Verifica rangos aleatorios de 64 bytes (`Range: bytes=offset..offset+63`); revoca inmediatamente `Edge Guardian` ante `404` y aísla ataques *Bait-and-Switch* en `GET /api/v1/swarm/quarantine` usando doble sello (`hera_seal` + `audio_sha256`).
- **Auditoría e Inteligencia de Repertorio (Audio + Video-DJ HD + Producciones Propias):**
  `ensure_hera_updated()` → `audit_repertoire(sync_db=True, check_queries=[...])` → `analyze_batch(paths_or_ids=[...])` → `build_dj_crate(..., export=["rekordbox_xml", "ableton", "cue_sheet", "m3u8"])`
- **Adquisición Externa Autorizada:**
  `ensure_hera_updated()` → `audit_repertoire(check_queries=[...])` → `search_music` → `get_track_candidates` → aprobación → `download_track` → `download_status` → `identify_track` → `analyze_track` → `organize_track` → `build_dj_crate`

# Conexión con Agentes Gratuitos (Gemini CLI / Google AI Studio / MCP)

- **Auto-Verificador / Bootstrapper:** `python scripts/ensure_latest_hera.py` (o `hera-update`)
- **Binario Nativo Rust (`stdio`):** `hera-node --mcp`
- **Seeder Local CLI (`hera-node --seed-dir`):** `hera-node --seed-dir "<ruta_repertorio>" --dj-handle "DaviDJ"`
- **Endpoint MCP HTTP / Cloudflare Workers:** `POST /mcp` (`GET /mcp/tools`)
- **Especificación OpenAPI 3.1.0 (Google AI Studio / Actions):** `GET /openapi.json`

# Referencias

- Consultar `references/tools.md` para el esquema detallado de parámetros y salidas.
- Consultar `references/policies.md` para bases de autorización permitidas y reglas legales.
- Consultar `references/workflows.md` para ejemplos conversacionales paso a paso.
