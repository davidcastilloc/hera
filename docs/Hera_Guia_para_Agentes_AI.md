# 🤖 HERA (`v0.6.0`) — Guía Técnica y Operativa para Agentes de IA y Clientes MCP

**Versión:** `0.6.0` (100% Pure Rust + Tauri v2 + Hexagonal `hera-ipc`)  
**Audiencia:** Agentes de IA (`Antigravity`, `Claude Code`, `Cursor`, `Codex`), desarrolladores de Skills y operadores MCP  
**Protocolos Soportados:** MCP `stdio` (`hera-node --mcp`), MCP HTTP (`POST /mcp`), TypeScript Code Mode (`POST /mcp/code-mode`) y Desktop Loopback (`127.0.0.1:0` Bearer HTTP MCP con `44 tools`)  
**Invariante Central:** *«La mejor decisión es por el bien del enjambre siempre. Mientras más nodos de Hera, más fuerte es el sistema de enjambres.»*

---

## 1. Objetivo del Agente en HERA `v0.6.0`

El agente convierte una intención musical del DJ en un flujo determinista, auditable y respaldado por motores nativos en Rust (`hera-node-rs` + `hera-desktop`):

```text
Intención del DJ (@ Contexto / comando / lenguaje natural)
  → Perfilado acústico (DJ Style DNA + SimHash u64)
  → Búsqueda en Biblioteca SQLite local y Enjambre P2P (5-Stage Cascade)
  → Descarga a cuarentena (bloques Merkle BEP 52 de 4 MB)
  → Auditoría Forense Zero-Trust en Rust (Symphonia + rustfft < 80 ms)
  → Detección estructural de 4 Hot Cues (1:Intro, 2:Breakdown, 3:Drop, 4:Outro)
  → Optimización global de secuencia con 2-Opt TSP (4 capas de sinergia)
  → Pre-escucha en DualDeckEngine (#![forbid(unsafe_code)])
  → Exportación 1-clic a VirtualDJ (.vdjfolder en vivo), Rekordbox XML, Traktor NML o M3U8
```

---

## 2. Arquitectura Hexagonal 100% Pure Rust (Cero Python)

![Concepto y Arquitectura del Enjambre P2P Simbiótico de HERA v0.6.0](../assets/hera_swarm_concept.jpg)

```text
Agente IA (Cloudflare Workers AI Free 10k Neurons/día | agy | Claude | Codex | Ollama)
                │
                │  MCP stdio (21 tools) / Desktop Loopback Bearer HTTP (44 tools)
                ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  HERA v0.6.0 (Workspace Cargo: hera-node-rs + hera-ipc + hera-desktop)     │
│                                                                            │
│  ├── hera-rs/src/forensics.rs     → Symphonia (8 codecs) + rustfft Welch   │
│  ├── hera-rs/src/structure.rs     → BPM, 24-key Camelot & 4 Hot Cues       │
│  ├── hera-rs/src/synergy.rs       → 4-Layer Synergy + 2-Opt TSP + SimHash  │
│  ├── hera-rs/src/audio_engine/    → DualDeckEngine SPSC [AtomicU64; 64]    │
│  ├── hera-rs/src/export.rs        → VirtualDJ, Rekordbox XML, Traktor, M3U8│
│  ├── hera-rs/src/dj_brain/        → DJ Style DNA + ReAct HeraAgentHarness  │
│  ├── hera-rs/src/p2p/             → 5-Stage P2P + Federated R2 Edge Swarm  │
│  └── hera-desktop/src-ipc/        → Contratos ts-rs + Guardián ACL Triad   │
└────────────────────────────────────────────────────────────────────────────┘
```

### Reglas de Arquitectura e Invariantes (`v0.6.0`)

1. **Cero Dependencias de Python o Binarios Externos:** Toda la decodificación de audio (`WAV`, `FLAC`, `MP3`, `M4A`, `AAC`, `ALAC`, `OGG`, `AIFF`), análisis espectral FFT, detección de Hot Cues y exportación XML/NML ocurre en Rust puro (`Symphonia` + `rustfft`).
2. **Invariante del Enjambre (*«Por el bien del enjambre»*):**
   - El 100% del motor DSP, 4 Hot Cues, `2-Opt TSP`, exportadores, LLMs locales y `Cloudflare Workers AI Free` (`10,000 Neurons/día` a `$0.00`) son gratuitos.
   - A cambio, todas las cuentas gratuitas/locales (`beta_mvp`, `free`, `local_unlimited`) mantienen siempre activo el enjambre P2P (`p2p_sharing_enabled = true`, `can_disable_p2p_sharing = false`). Solo las cuentas `pro_monthly` (`50 GB` R2) o `founder_unlimited` pueden desactivar el P2P para dubplates privados.
3. **Cuarentena Primero (`Quarantine-First`):** Ninguna pista entra a la biblioteca ni al enjambre sin superar la auditoría forense (`VERIFIED_STUDIO_MASTER`) y recibir el doble sello criptográfico (`hera_seal` + `audio_sha256`).
4. **Compuertas de Confirmación (`requires_confirmation`):** En el espacio `/agent` de HERA Desktop (`44 tools`), las herramientas que modifican o eliminan recursos requieren confirmación explícita cuando el usuario opera en modo `Confirmar`.

---

## 3. Catálogo de Herramientas MCP (`21 Core Tools` + `23 Desktop Tools` = `44 Tools`)

### 3.1 Herramientas Core en `hera-node-rs` (`hera-rs/src/mcp/`)

| Categoría | Herramienta MCP | Descripción |
| :--- | :--- | :--- |
| **DSP & Forense** | `analyze_audio_forensics` | Ejecuta auditoría espectral `Symphonia + rustfft`, detecta falsos lossless, fase sub-grave (`< 120 Hz`), BPM, Camelot y 4 Hot Cues. |
| **Curaduría & Crates** | `build_dj_crate` | Ordena un crate completo usando el motor de sinergia de 4 capas y optimización global **2-Opt TSP**. |
| **Curaduría & Crates** | `build_set_spine` | Calcula el puente armónico óptimo (`Actual → Puente → Destino`) entre dos pistas distantes en BPM/Camelot. |
| **Exportación Cabina** | `export_dj_crate` | Exporta un crate con sus 4 Hot Cues a **VirtualDJ (`.vdjfolder`)**, **Rekordbox XML**, **Traktor NML** o **M3U8**. |
| **DJ Brain & ADN** | `compute_dj_style_dna` | Sintetiza el perfil `DJ Style DNA` (`SimHash u64`, BPM habitual, llaves Camelot dominantes y afinidad de estilo). |
| **DJ Brain & IA** | `discover_ai_backends` / `run_agent_turn` | Descubre los 12 backends de IA (`Cloudflare Workers AI Free`, `agy`, `claude`, `ollama`, etc.) y ejecuta turnos ReAct. |
| **P2P & Enjambre R2** | `p2p_search_catalog` / `p2p_announce_master` | Busca masters verificados en la red P2P o anuncia un nuevo `VERIFIED_STUDIO_MASTER` con su `hera_seal`. |
| **P2P & Enjambre R2** | `get_swarm_strength` / `swarm_heartbeat` | Consulta métricas en vivo del enjambre (`Edge Guardians`, $K$ elástico) o registra un heartbeat con `Constrained Water-Filling`. |
| **P2P & Enjambre R2** | `swarm_route_ticket` / `swarm_parallel_manifest` | Obtiene tickets firmados de `60s` vía `Latency-Aware P2C` o manifiestos multi-nodo `HTTP/3-QUIC` para descarga paralela de bloques de `4 MB`. |
| **P2P & Enjambre R2** | `swarm_custody_challenge` / `swarm_custody_audit` | Emite y verifica desafíos `Proof-of-Custody` aleatorios de 64 bytes sobre buckets R2 comunitarios. |
| **P2P & Enjambre R2** | `swarm_crdt_sync` / `swarm_subscribe` | Sincroniza estado *Offline-First* (`Delta-CRDT`) o suscribe el nodo a tópicos armónicos `hera://swarm/<camelot>/<bpm>`. |

### 3.2 Herramientas Adicionales en `HERA Desktop` (`hera-desktop/src-ipc/src/agent_catalog_schemas.rs`)

El servidor loopback `127.0.0.1:0` de `HERA Desktop` añade **23 herramientas nativas de escritorio** para controlar toda la aplicación:
- **Biblioteca SQLite y Búsqueda:** `desktop_search_library`, `desktop_get_library_stats`, `desktop_import_folder`, `desktop_get_track_details`, `desktop_filter_tracks_by_bpm_key`.
- **Reproductor y Hot Cues en Vivo:** `desktop_player_play_track`, `desktop_player_pause_resume`, `desktop_player_seek_hot_cue`, `desktop_preview_transition_bridge`.
- **Playlists y Cabina VirtualDJ:** `desktop_list_playlists`, `desktop_create_playlist`, `desktop_push_to_virtualdj_live` (inyecta en `HERA En Vivo.vdjfolder`), `desktop_toggle_mini_mode` (`360×408` / `560×52`).

---

## 4. Comandos Canónicos de Verificación para Agentes (RDD Gate)

Antes de dar por finalizada cualquier modificación en el repositorio, todo agente debe ejecutar y verificar con `exit code 0`:

```bash
# 1. Verificación rápida de compilación del workspace Cargo (hera-rs + hera-ipc)
cargo check

# 2. Suite completa de tests del motor DSP, DualDeckEngine, Exportadores, MCP y Enjambre P2P
cargo test -p hera-node-rs

# 3. Guardián Hexagonal: contratos ts-rs, Tríada ACL de Tauri v2 (8 plugins) y activos de documentación
cargo test -p hera-ipc

# 4. Generación de tipos, linter/typecheck (Vite+), tests unitarios (Vitest) y build de producción en HERA Desktop
cd hera-desktop && bun run gen:types && bun run check && bun run test && bun run build
```
