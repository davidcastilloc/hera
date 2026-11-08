# DJ Laptop-First Product & Community Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Alinear Hera y Hera Cloud al flujo real del DJ en su propia laptop (organización local en `hera.db`, descarga física a disco, exportación nativa a VirtualDJ/Rekordbox/Traktor y uso sin depender de Antigravity/API Keys).

**Architecture:** Mantiene el núcleo local-first en Python + Rust (`src/hera/` y `hera-rs/`) donde toda la música aterriza físicamente en el disco del DJ (`quarantine/` -> `library/` -> `sets/`), extiende `CrateExporter` para soportar VirtualDJ y Traktor junto a Rekordbox/Ableton, y expone flujos de 1 clic en la UI local para DJs sin herramientas de desarrollo.

**Tech Stack:** Python `>=3.11` (`uv`, `pydantic`, `aiosqlite`, `streamlit`, `click`), Rust (`hera-node-rs`), SQLite (`hera.db`).

**Spec:** `docs/superpowers/specs/2026-11-08-dj-laptop-first-product-alignment-design.md`

---

## 📌 Backlog Vivo de Ideas de Planeación (En Construcción con David)

- [ ] **Idea 1 (Caso de Uso Estrella — Organización e Inteligencia de Repertorio Local):**
  - Consolidar el flujo de auditoría y sincronización en `hera.db` (`src/hera/domain/repertoire_intelligence.py`) como la puerta de entrada principal para cualquier DJ que instala Hera en su laptop (Audio, Video-DJ y Producciones Propias).
- [ ] **Idea 2 (Soporte Nativo VirtualDJ + Rekordbox + Traktor en `src/hera/domain/export.py`):**
  - Agregar exportador nativo para **VirtualDJ** (`.vdjfolder` y marcadores `<Poi>` para los 4 Hot Cues acústicos) y **Traktor Pro** (`collection.nml` con `<CUE_V2>`), manteniendo Rekordbox XML, M3U8 y Ableton Live.
- [ ] **Idea 3 (Sin Internet = Botones Deterministas Reales en el Front, Cero Heurísticas Tramposas de Chat):**
  - Hoy en `src/hera/ui/app.py`, los botones de "Acciones Rápidas" solo mandan un texto al chat (que falla si no hay internet o LLM).
  - **Solución:** Construir paneles y botones **100% deterministas** en el frontend (`hera ui`):
    1. Botón directo **[🔍 Auditar y Sincronizar Repertorio Local]** (ejecuta `handle_audit_repertoire` y muestra gráficas de BPM/Camelot/Calidad en vivo).
    2. Formulario visual **[🎛️ Armar Set / Crate Armónico]** con selectores de Género, Rango BPM, Llave Camelot, Duración y check de formatos (**VirtualDJ, Rekordbox, Traktor, Ableton**).
    3. Buscador P2P directo **[📥 Buscar y Descargar Canciones]** que consulta `slskd` y descarga a `quarantine/` ➔ `library/` con un clic.
- [ ] **Idea 4 (Capa Gratuita por Defecto Exprimida al Máximo con la Suite de Cloudflare AI):**
  - Basado en los números oficiales de `developers.cloudflare.com/ai/`:
    1. **Workers AI (`10,000 Neurons/día` Gratis):** Usar `@cf/meta/llama-3.1-8b-instruct-fp8-fast` / `@cf/meta/llama-3.2-3b-instruct` como modelo por defecto para *tool calling* (~6.6 Neurons por turno de DJ = **~1,500 mensajes/día gratis** a `$0.00` por cuenta, consumiendo `0%` de RAM/CPU en la laptop del DJ).
    2. **Cloudflare AI Gateway (Gratis):** Activar **Caching** (consultas repetidas de Camelot/catálogo cuestan `0 Neurons`), **Rate Limiting** por DJ (ej. `30 msgs/hora` para que nadie abuse de la cuota comunitaria) y **Dynamic Model Fallback** (si se agotan las `10,000 Neurons` del día, salta automáticamente a un modelo `:free` de OpenRouter/Gemini Free Tier sin caerse).
    3. **Cloudflare Vectorize (Gratis: 5M dimensiones guardadas / 30M consultadas al mes):** Búsqueda semántica musical en `hera-cloud` sin pagar servidores de vectores.
    4. **Federación de Neuronas Comunitarias (Bonus):** Igual que el enjambre R2 suma `9 GB` por cada nodo comunitario, cada `Edge Guardian` con un Cloudflare Worker gratuito aporta otras **`10,000 Neurons/día` gratis** al pool de IA de la comunidad.
- [ ] **Idea 5 (Panel de Configuración de Motores LLM Intuitivo y Bonito en el Front):**
  - Igual que el 99% de las herramientas modernas de IA, agregar en `src/hera/ui/app.py` una sección visual limpia y fácil de usar para configurar el motor de IA y guardarlo en `config/hera.toml`:
    - **Por defecto:** `⚡ Hera Cloud Free (Cloudflare Workers AI)` — listo para usar sin claves.
    - **Cloud BYOK (Bring Your Own Key):** `OpenRouter` (con selector de modelos `:free` y pagos), `Google Gemini`, `OpenAI`, `Anthropic`.
    - **Local (Opcional para laptops potentes):** `Ollama` / `LM Studio` con autodetección de puerto y advertencia amigable del uso de RAM/CPU.
- [ ] **Idea 6 (Alineación de `hera-cloud` al modelo Laptop-First + Comunidad P2P Solidaria):**
  - Posicionar `hera-cloud` como:
    1. El **Gateway de IA Gratuito (Cloudflare Workers AI)** para los DJs de la comunidad.
    2. El **Directorio Global de Inteligencia Acústica** (`hera_seal`, 4 Hot Cues, puentes armónicos `resolve-bridge`), mientras que los archivos de música siempre aterrizan físicamente en el disco duro de la laptop del DJ.
- [ ] **Idea 7 (El "DJ Brain" & Harness Persistente — Local `hera.db` + Sync en `hera-cloud` D1 a `$0.00`):**
  - Aprovechar que `src/hera/agent/brain.py` ya corre sobre el SDK `google-antigravity` y que `src/hera/domain/database.py` ya tiene la tabla `dj_preferences`:
    1. **Memoria Estructurada del DJ (`DJ Brain`):** Inyectar automáticamente al inicio de cada sesión un contexto compacto (~300 tokens) con: resumen del repertorio (`hera.db`), software preferido del DJ (ej. `VirtualDJ`), géneros/BPM favoritos (`dj_preferences`) y últimos sets creados (`crates`).
    2. **Persistencia de Conversaciones (`conversation_sessions` y `conversation_messages`):** Guardar todos los hilos de conversación en `hera.db` local y sincronizarlos con **Cloudflare D1 en `hera-cloud`** para todos los planes.
    3. **Por qué cuesta `$0.00` en `hera-cloud`:** El texto es ultraligero (~350 bytes por mensaje). Los **`5 GB` gratuitos de Cloudflare D1** caben **~14 millones de mensajes** y permite **5 millones de lecturas/día + 100,000 escrituras/día gratis**.
- [ ] **Idea 8 (Onboarding Inteligente del DJ + Agente Calculador de Estilo y ADN Musical — `DJ Style Profiler`):**
  - Al entrar por primera vez (o desde el panel **[Mi Perfil & Estilo DJ]**), Hera ejecuta un onboarding en 2 fases combinando cálculo automático + confirmación humana:
    1. **Auto-Análisis del Estilo del DJ desde su Disco (`Zero-Typing Style Discovery`):** Expandir `artist_dna_profile` en `src/hera/domain/repertoire_intelligence.py` + `analyzers/audio_features/style_embeddings.py` para analizar automáticamente la biblioteca local del DJ y extraer:
       - Sus zonas de BPM reales (ej. *Warm-up 70-105*, *Tropical 106-121*, *Club/Afro/Guaracha 122-132*, *Hora Loca 133+*).
       - Sus llaves Camelot dominantes (`8A`, `9A`, `10A`...), proporción de Audio vs. Video-DJ HD vs. Producciones Propias, y su Top 15 de artistas insignia.
    2. **Entrevista Corta de Onboarding (UI / Chat):** Presentarle al DJ su "Radiografía Musical" calculada y preguntarle solo lo que la máquina no puede adivinar:
       - Nombre artístico (`dj_handle`), software principal (`VirtualDJ`, `Rekordbox`, `Traktor`, `Serato`, `Ableton`), tipo de transiciones preferidas (*largas progresivas* vs *cortes rápidos / open-format*) y sus **Reglas de Oro** (ej. *"siempre meter al menos 1 producción propia"*, *"nunca bajar de 320 kbps"*).
    3. **Aprendizaje Continuo en el Brain:** Guardar todo este perfil en `dj_preferences` (`hera.db` local + `hera-cloud` D1) y actualizarlo automáticamente cada vez que el DJ exprese una preferencia o corrección en futuras conversaciones.
- [ ] **Idea 9 (Replanteamiento Integral de la UI y la App de Escritorio — Reproductor Simple + Centro de Mando del DJ):**
  - **Diagnóstico:** Hoy `src/hera/ui/dj_player.py` dedica 1,142 líneas a un reproductor Dual-Deck CDJ dentro de Streamlit (es demasiado pesado e innecesario porque el DJ mezcla en VirtualDJ/Rekordbox).
  - **Nuevo Enfoque de la UI y Desktop App:**
    1. **Reproductor Simple de Pre-Escucha (`Single-Track Quick Player`):** Reemplazar el Dual-Deck gigante por un **reproductor simple y limpio de 1 sola pista** (play/pause, barra de onda/tiempo, volumen y botones rápidos para saltar a los **4 Hot Cues**: `Intro`, `Entrada`, `Drop`, `Salida`), reutilizando el servidor ligero `_AudioRangeHandler` para streaming local instantáneo sin recargar la página.
    2. **Estructura en 5 Vistas Claras (100% funcionales con Botones Offline + Agente IA):**
       - `🧬 Mi ADN & Onboarding DJ` (Perfil de estilo auto-calculado, software por defecto y reglas del Brain).
       - `🗂️ Mi Repertorio & Auditoría` (Salud de las carpetas locales, zonas BPM, llaves Camelot, calidad FFT y limpieza de duplicados).
       - `🎛️ Constructor de Crates & Sets` (Controles deterministas de BPM/Camelot/Duración + Chat del Agente + Exportación 1-clic a VirtualDJ/Rekordbox/Traktor).
       - `🌍 Comunidad P2P & Buscador de Puentes` (Búsqueda y descarga directa al disco duro de la laptop + buscador de puentes armónicos entre 2 canciones).
       - `⚙️ Motor de IA & Conexiones` (Cloudflare Workers AI Gratis por defecto, OpenRouter/Gemini BYOK, Ollama local opcional).
    3. **App de Escritorio (`hera desktop` / Tray + Ventana Nativa Ligera):** Que el icono de bandeja del sistema (`src/hera/desktop/tray.py`) y el lanzador abran la interfaz como una app limpia de escritorio (modo ventana/app sin parecer una página técnica) mientras mantiene vivo el nodo P2P en segundo plano.
- [ ] **Idea 10 (Investigación y Selección del Stack Ideal para el Frontend + Desktop App — Referencia `martpie/museeks`):**
  - Tomar **[`martpie/museeks`](https://github.com/martpie/museeks)** (`Tauri v2 + Rust + SQLite + React/TypeScript + Vite`, licencia MIT) como referencia directa de arquitectura y simplicidad visual:
    1. **Por qué calza como un guante en Hera:** Museeks migró de Electron a **Tauri v2 + Rust + SQLite** logrando una app de música multiplataforma ultraligera, con una tabla de pistas rapidísima, barra de reproducción simple de 1 pista, playlists/crates a la izquierda y consumo mínimo de RAM.
    2. **Adaptación a Hera:** Nosotros ya tenemos el backend Rust (`hera-rs/`) y la base SQLite (`hera.db`). Podemos usar el mismo patrón arquitectónico de Museeks (`TrackList` virtualizada + `PlayerBar` de 1 pista + `Tauri v2`) sumándole las columnas DJ (`BPM`, `Camelot`, `4 Hot Cues`, `Sello FFT`), los botones deterministas de armado de Crates/Exportación (VirtualDJ, Rekordbox, Traktor) y el panel lateral del **Copiloto IA (DJ Brain)**.
- [ ] **Idea 11 (Nuevas Funcionalidades Visuales Diferenciadoras en `HERA Desktop`):**
  1. **Waveform Superior con los 4 Pines de Hot Cues Clicables (`🟢 Cue 1 Intro`, `🔵 Cue 2 Phrase`, `🔴 Cue 3 Drop`, `🟡 Cue 4 Mix Out`):** En la barra simple de reproducción estilo Museeks, permitir saltar con 1 clic a cada punto estructural del tema.
  2. **Dashboard Visual de "Mi ADN DJ" & Switches de Reglas de Oro del Brain:** Radar Camelot de tonos dominantes, barras de zonas BPM, selector de 1 clic de software principal (`VirtualDJ`, `Rekordbox`, `Traktor`, `Ableton`) y switches visuales para las reglas del `DJ Brain` (*Priorizar mis producciones propias*, *Calidad mínima FLAC/320k*, *Sinergia Camelot estricta*).
  3. **Visualizador de "Puente Armónico Inteligente" + Botón `[Pre-escuchar Transición]`:** Conector visual que muestra las 2 canciones puente encontradas en la comunidad P2P entre una Pista Origen y una Pista Destino incompatibles, con botón para descargarlas directo al disco local y opción de pre-escuchar 15 segundos del cruce (`Cue 4 Mix Out` de A + `Cue 1 Intro` de B).

---

## 🖼️ Mockups Visuales Versionados en el Repositorio (`docs/superpowers/assets/`)

- **Vista 1 (Biblioteca estilo Museeks + Reproductor de 4 Hot Cues + Botones Deterministas + Copiloto IA):** `docs/superpowers/assets/hera_ui_1_library_crate.jpg`
- **Vista 2 (Mi ADN Musical + Rueda Camelot + Selector VirtualDJ/Rekordbox/Traktor + Reglas del DJ Brain):** `docs/superpowers/assets/hera_ui_2_dna_onboarding.jpg`
- **Vista 3 (Puente Armónico Inteligente P2P + Pre-escucha de Transición + Descarga a Disco Local):** `docs/superpowers/assets/hera_ui_3_harmonic_bridge_p2p.jpg`
