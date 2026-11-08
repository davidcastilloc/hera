# DJ Laptop-First Product & Community Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Alinear Hera y Hera Cloud al flujo real del DJ en su propia laptop (organización local en `hera.db`, descarga física a disco, exportación nativa a VirtualDJ/Rekordbox/Traktor y uso sin depender de Antigravity/API Keys).

**Architecture:** Mantiene el núcleo local-first en Python + Rust (`src/hera/` y `hera-rs/`) donde toda la música aterriza físicamente en el disco del DJ (`quarantine/` -> `library/` -> `sets/`), extiende `CrateExporter` para soportar VirtualDJ y Traktor junto a Rekordbox/Ableton, y expone flujos de 1 clic en la UI local para DJs sin herramientas de desarrollo.

**Tech Stack:**
- **Frontend & Desktop (`HERA Desktop` — Clon Directo de `martpie/museeks` MIT `v0.23.4`, 100% Rust Nativo `~8 MB` sin Python Empaquetado):** `Tauri v2` (`src-tauri`) + **`React 19` (`babel-plugin-react-compiler` 1.0)** + **`StyleX` (Zero-Runtime Atomic CSS)** + **`@tanstack/react-virtual`** + **`@tanstack/react-router`** + **`Zustand`** + **`@dnd-kit`** + **`ts-rs`** (clonando el 100% de los `+40` componentes UI/UX curados de `martpie/museeks` y adaptándolos a HERA).
- **Backend Local Desktop (`100% Rust en el Instalador`: `hera-rs` + `src-tauri`):** **`symphonia`, `rustfft`, `axum`, `sqlx` (SQLite), `lofty`, `2-Opt TSP`, `u64 SimHash POPCNT`, 5 capas Post-P2P, `MCP`, `export.rs` (`VirtualDJ`, `Rekordbox`, `Traktor`, `M3U8`) y `dj_brain.rs` (`DJ Style DNA` + cliente `Cloudflare Workers AI Free`)**. El instalador `.exe`/`.dmg`/`.deb` pesa `~8 MB` y tiene **Cero dependencia de Python** (el paquete Python `src/hera/` queda exclusivamente como CLI/SDK opcional para desarrolladores).
- **Backend Cloud (`hera-cloud` en Cloudflare Edge):** **Cloudflare Workers (`Rust` -> `wasm32`)** + **Cloudflare Workers AI (`10,000 Neurons/día` gratis por defecto)** + **AI Gateway** + **Cloudflare D1 (`5 GB` SQLite gratis)** + **Cloudflare Vectorize** + **Cloudflare R2 (`Zero-Egress`)**.

**Spec:** `docs/superpowers/specs/2026-11-08-dj-laptop-first-product-alignment-design.md`

---

## 📌 Backlog Vivo de Ideas de Planeación (En Construcción con David)

- [x] **Idea 1 (Caso de Uso Estrella — Organización e Inteligencia de Repertorio Local):**
  - Consolidar el flujo de auditoría y sincronización en `hera.db` (`hera-rs/src/dj_brain.rs` y `src/hera/domain/repertoire_intelligence.py`) como la puerta de entrada principal para cualquier DJ que instala Hera en su laptop (Audio, Video-DJ y Producciones Propias).
- [x] **Idea 2 (Soporte Nativo VirtualDJ + Rekordbox + Traktor en `hera-rs/src/export.rs` y `src/hera/domain/export.py`):**
  - Agregar exportador nativo 100% en Rust (`hera-rs/src/export.rs`) y Python para **VirtualDJ** (`.vdjfolder` y marcadores `<Poi>` para los 4 Hot Cues acústicos) y **Traktor Pro** (`collection.nml` con `<CUE_V2>`), manteniendo Rekordbox XML, M3U8 y Ableton Live.
- [x] **Idea 3 (Sin Internet = Botones Deterministas Reales en el Front, Cero Heurísticas Tramposas de Chat):**
  - Construidos paneles y botones **100% deterministas** en `hera-desktop/src/components/HeraDjToolbar.tsx` y `hera-desktop/src/lib/bridge-hera.ts`:
    1. Botón directo **[🧬 Mi ADN DJ]** (ejecuta `computeDjStyleDna` contra `POST /api/v1/dj/style-dna` en `hera-rs` sin LLM ni internet).
    2. Selector y botón directo **[🎛️ Exportar VDJ / Rekordbox / Traktor]** que genera `.vdjfolder` + `_vdj_database.xml` (`VirtualDJ`), `_rekordbox.xml` (`Rekordbox`), `_traktor.nml` (`Traktor Pro`) y `.m3u8` vía `POST /api/v1/crates/export`.
    3. Botón **[🌍 Puente P2P & Copiloto IA]** que conecta la búsqueda armónica y el copiloto IA.
- [x] **Idea 4 (Capa Gratuita por Defecto Exprimida al Máximo con la Suite de Cloudflare AI en `hera-rs/src/dj_brain.rs` y `hera-cloud`):**
  - Basado en los números oficiales de `developers.cloudflare.com/ai/`:
    1. **Workers AI (`10,000 Neurons/día` Gratis):** Usar `@cf/meta/llama-3.1-8b-instruct-fp8-fast` / `@cf/meta/llama-3.2-3b-instruct` como modelo por defecto para *tool calling* (~6.6 Neurons por turno de DJ = **~1,500 mensajes/día gratis** a `$0.00` por cuenta, consumiendo `0%` de RAM/CPU en la laptop del DJ).
    2. **Cloudflare AI Gateway (Gratis):** Activar **Caching** (consultas repetidas de Camelot/catálogo cuestan `0 Neurons`), **Rate Limiting** por DJ (ej. `30 msgs/hora` para que nadie abuse de la cuota comunitaria) y **Dynamic Model Fallback** (si se agotan las `10,000 Neurons` del día, salta automáticamente a un modelo `:free` de OpenRouter/Gemini Free Tier sin caerse).
    3. **Cloudflare Vectorize (Gratis: 5M dimensiones guardadas / 30M consultadas al mes):** Búsqueda semántica musical en `hera-cloud` sin pagar servidores de vectores.
    4. **Federación de Neuronas Comunitarias (Bonus):** Igual que el enjambre R2 suma `9 GB` por cada nodo comunitario, cada `Edge Guardian` con un Cloudflare Worker gratuito aporta otras **`10,000 Neurons/día` gratis** al pool de IA de la comunidad.
- [x] **Idea 5 (Catálogo de Motores LLM `/api/v1/ai/backends` en Rust + `backends.py`):**
  - Expuesto en `hera-rs/src/dj_brain.rs` (`GET /api/v1/ai/backends`) y `src/hera/agent/backends.py`:
    - **Por defecto:** `⚡ Hera Cloud Free (Cloudflare Workers AI)` (`cloudflare_free`) — listo para usar sin claves.
    - **Cloud BYOK (Bring Your Own Key):** `OpenRouter` (con selector de modelos `:free` y pagos), `Google Gemini`, `OpenAI`, `Anthropic`.
    - **Local (Opcional para laptops potentes):** `Ollama` / `LM Studio` con autodetección de puerto y advertencia amigable del uso de RAM/CPU.
- [x] **Idea 6 (Alineación de `hera-cloud` al modelo Laptop-First + Comunidad P2P Solidaria):**
  - Posicionar `hera-cloud` como:
    1. El **Gateway de IA Gratuito (Cloudflare Workers AI)** (`POST /api/v1/ai/chat`) para los DJs de la comunidad.
    2. El **Directorio Global de Inteligencia Acústica** (`hera_seal`, 4 Hot Cues, puentes armónicos `resolve-bridge`), mientras que los archivos de música siempre aterrizan físicamente en el disco duro de la laptop del DJ.
- [x] **Idea 7 (El "DJ Brain" & Harness Persistente — `hera-rs/src/dj_brain.rs` + Local SQLite + Sync en `hera-cloud` D1 a `$0.00`):**
  - Implementado tanto en Rust (`hera-rs/src/dj_brain.rs`) como en Python (`repertoire_intelligence.py`):
    1. **Memoria Estructurada del DJ (`DJ Brain`):** Construye automáticamente un `prompt_block` compacto (~250 tokens) con: resumen del repertorio, software preferido del DJ (ej. `VirtualDJ`), zonas BPM/Camelot dominantes (`dj_style_dna`) y reglas de oro.
    2. **Por qué cuesta `$0.00` en `hera-cloud`:** El texto es ultraligero (~350 bytes por mensaje). Los **`5 GB` gratuitos de Cloudflare D1** caben **~14 millones de mensajes** y permite **5 millones de lecturas/día + 100,000 escrituras/día gratis**.
- [x] **Idea 8 (Onboarding Inteligente del DJ + Agente Calculador de Estilo y ADN Musical — `DJ Style Profiler`):**
  - Al entrar por primera vez (o desde el panel **[Mi Perfil & Estilo DJ]**), Hera ejecuta un onboarding en 2 fases combinando cálculo automático + confirmación humana:
    1. **Auto-Análisis del Estilo del DJ desde su Disco (`Zero-Typing Style Discovery`):** Expandir `artist_dna_profile` en `src/hera/domain/repertoire_intelligence.py` + `analyzers/audio_features/style_embeddings.py` para analizar automáticamente la biblioteca local del DJ y extraer:
       - Sus zonas de BPM reales (ej. *Warm-up 70-105*, *Tropical 106-121*, *Club/Afro/Guaracha 122-132*, *Hora Loca 133+*).
       - Sus llaves Camelot dominantes (`8A`, `9A`, `10A`...), proporción de Audio vs. Video-DJ HD vs. Producciones Propias, y su Top 15 de artistas insignia.
    2. **Entrevista Corta de Onboarding (UI / Chat):** Presentarle al DJ su "Radiografía Musical" calculada y preguntarle solo lo que la máquina no puede adivinar:
       - Nombre artístico (`dj_handle`), software principal (`VirtualDJ`, `Rekordbox`, `Traktor`, `Serato`, `Ableton`), tipo de transiciones preferidas (*largas progresivas* vs *cortes rápidos / open-format*) y sus **Reglas de Oro** (ej. *"siempre meter al menos 1 producción propia"*, *"nunca bajar de 320 kbps"*).
    3. **Aprendizaje Continuo en el Brain:** Guardar todo este perfil en `dj_preferences` (`hera.db` local + `hera-cloud` D1) y actualizarlo automáticamente cada vez que el DJ exprese una preferencia o corrección en futuras conversaciones.
- [x] **Idea 9 (Replanteamiento Integral de la UI y la App de Escritorio — Reproductor Simple + Centro de Mando del DJ):**
  - **Implementado en `hera-desktop/`:**
    1. **Reproductor Simple de Pre-Escucha (`Header.tsx` + `PlayingBar.tsx` + `TrackProgress.tsx`):** Reproductor limpio de 1 sola pista con barra de progreso y los **4 Hot Cues clicables** (`1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro`).
    2. **Barra Determinista del DJ (`HeraDjToolbar.tsx` en `Navigation.tsx`):** Acceso de 1 clic a `🧬 Mi ADN DJ`, selector `VirtualDJ`/`Rekordbox`/`Traktor`/`M3U8`, exportación determinista y Copiloto IA.
    3. **App de Escritorio (`hera-desktop/src-tauri` — Tauri v2 + Tray):** Binario nativo ultraligero (`~8 MB`, 100% Rust sin Python empaquetado).
- [x] **Idea 10 (Clonado Directo de `martpie/museeks` MIT `v0.23.4` y Adaptación a `HERA Desktop` en `hera-desktop/`):**
  - Clonado el código base oficial de **[`martpie/museeks`](https://github.com/martpie/museeks)** (`v0.23.4`, MIT) dentro de `hera-desktop/` (`Tauri v2 + React 19 Compiler + StyleX + @tanstack/react-virtual + @tanstack/react-router + Zustand + @dnd-kit + SQLite sqlx + ts-rs`):
    1. **Preservación del 100% de los `+40` componentes curados:** `TrackList.tsx` virtualizada, `TrackRow.tsx`, `TrackListHeader.tsx`, `Header.tsx`, `PlayingBar.tsx`, `TrackProgress.tsx`, `Navigation.tsx`, `DropzoneImport.tsx` y temas con `StyleX` zero-runtime.
    2. **Integración Nativa con `hera-rs` y SQLite (`04_add_hera_dj_columns.sql`):** Extendido `Track` (`src-tauri/src/libs/track.rs` y `src/generated/typings.ts`), `TrackListHeader.tsx` y `TrackRow.tsx` con `BPM`, badge `Camelot Key` y `🟢 🔵 🔴 🟡` (`4 Hot Cues`).
- [x] **Idea 11 (Nuevas Funcionalidades Visuales Diferenciadoras en `HERA Desktop`):**
  1. **Barra de Progreso Superior con los 4 Pines de Hot Cues Clicables (`TrackProgress.tsx`):** Pines interactivos (`1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro`) que saltan al segundo exacto del tema con `PlayerAPI.jumpTo(cue.seconds)`.
  2. **ADN Musical & Exportación Directa 1-Clic (`HeraDjToolbar.tsx` + `bridge-hera.ts`):** Cálculo determinista de `DJ Style DNA` (`dominant_bpm_zone`, `dominant_camelot_keys`) y exportación instantánea a `VirtualDJ (.vdjfolder)`, `Rekordbox (.xml)` y `Traktor (.nml)`.

---

## 🖼️ Mockups Visuales Versionados en el Repositorio (`docs/superpowers/assets/`)

- **Vista 1 (Biblioteca estilo Museeks + Reproductor de 4 Hot Cues + Botones Deterministas + Copiloto IA):** `docs/superpowers/assets/hera_ui_1_library_crate.jpg`
- **Vista 2 (Mi ADN Musical + Rueda Camelot + Selector VirtualDJ/Rekordbox/Traktor + Reglas del DJ Brain):** `docs/superpowers/assets/hera_ui_2_dna_onboarding.jpg`
- **Vista 3 (Puente Armónico Inteligente P2P + Pre-escucha de Transición + Descarga a Disco Local):** `docs/superpowers/assets/hera_ui_3_harmonic_bridge_p2p.jpg`
