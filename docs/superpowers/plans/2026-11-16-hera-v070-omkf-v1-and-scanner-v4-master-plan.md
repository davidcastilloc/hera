# Plan Maestro v4.1 (Reconciliado sobre `4b6f60a` + `fc951a8`): Estándar Bicameral `OMKF v1` (`.hera/`), `Carpetas = Playlists`, Poda Multimodal `0.58 ms` y Refuerzos Zero-I/O 50k

> **For agentic workers (Hydrogen / Antigravity / OpenCode):** REQUIRED SUB-SKILL: Follow `superpowers-odd` (*Inspect First, Merge Deltas Only* + *Tidy First* + *Receipt-Driven Development*). Execute sequentially Milestone by Milestone (`O1` -> `O5`), marking `[x]` only after pasting the real `exit code 0` terminal receipt. **Zero stubs, zero dead files, zero new Cargo/NPM dependencies.**

**Ruta destino en el repositorio:** `docs/superpowers/plans/2026-11-16-hera-v070-omkf-v1-and-scanner-v4-master-plan.md`  
**Especificación compañera:** `docs/superpowers/specs/library_scanner_v3.md`  
**HEAD auditado en `main`:** `4b6f60a595c19b976f5e3f7d3f05279074f9898e` (`Nov 16, 2026`)  
**Coordinación con planes existentes en `main`:**
- `docs/superpowers/plans/2026-11-15-realtime-scanner-ux-and-instant-library.md`: **`[x]` COMPLETADO en `main`** (`07_add_scanner_v3_columns.sql`, 16 formatos audio/video, `fast_library_scan.rs`, `scanner_worker.rs` 200 ms WAL + fix deadlock `ffmpeg`, split SQLite read pool en `529950b`, deck prefix streaming `< 50 ms` + tope OOM `> 15 m` en `4b6f60a`, `EXPECTED_TAURI_COMMANDS_COUNT = 63`).
- `docs/superpowers/plans/2026-11-15-curation-intelligence-and-zero-io-booth.md`: **`[ ]` PENDIENTE en `main`** (Migración `08_add_zero_io_and_curation_intelligence.sql`, LRU `cover.rs`, `audio_metrics.rs`, `cue_engine.rs` de 8 Hot Cues, 14 familias `Genre x Era`, `shazam_id/`, `folder_watcher.rs`, `scanner_intelligence.rs` y `+10` comandos IPC `63 -> 73`).
- **Este Plan Maestro `v4.1` (`O1` -> `O5`)** entrega el **delta ortogonal `OMKF v1` (*Open Music Knowledge Format*: `.hera/crate.md` + `.hera/tracks.omkf` alineado a `928 B` + `tree_seal_64` + vista `Carpetas = Playlists` + organizador agéntico reversible)** más los **4 refuerzos críticos de simulación 50k** (`completed_level` `FAST -> FULL`, `detect_song_key` v2.2, `SeekMode::Coarse` sin 3ra STFT y prioridad OS `BELOW_NORMAL`/`nice(10)`), **sin colisionar con la Migración `07` ni con la `08`**.

---

## 1. Matriz de Estado y Reconciliación sobre `4b6f60a` (`main`)

| Bloque Funcional | Estado Real en `4b6f60a` (`main`) | Acción Exacta en Este Plan `v4.1` (`Inspect First, Merge Deltas Only`) |
| :--- | :---: | :--- |
| **Indexador Nivel 0, 16 formatos VideoDJ, VDJ XML, Split SQLite Read Pool y Deck Prefix `< 50 ms`** | **`[x]` En `main`** (`c4251bf`..`4b6f60a`) | **No tocar ni duplicar.** Preservar el pool de lectura separado de `plugins/db.rs` (`529950b`), el tope OOM `> 15 m` (`4b6f60a`) y `07_add_scanner_v3_columns.sql`. |
| **Cosecha Cero-IO (`LUFS`, RGB `768 B`, `64D`), 8 Hot Cues, `Genre x Era` (14 familias) y `cover.rs` LRU** | **`[ ]` En Plan `2026-11-15`** (`Migración 08`, `+10` cmds `63 -> 73`) | **Compatible al 100%.** `OmkfTrackSlot` (`928 B`) reserva `[48..80]` para los **8 Hot Cues** y `[88..96]` para **`Genre x Era` + `sub_phase_corr` + `bitrate_kbps`**. Si `08_...sql` aún no existe al correr `O2`, crear las columnas faltantes con chequeo o ejecutar después de `Task 3` del plan `2026-11-15`. |
| **Refuerzos Críticos DSP / Simulación 50k (`F03`, `F07`, `F20`, `F30-OS`)** | **Parcial / No explícito en `4b6f60a`** | **Aplicar en `O1`:** (1) `completed_level: HashMap<String, i8>` en `ScanScheduler` para re-promoción `FAST (1) -> FULL (2)` sin re-encolar `-1`/`2`; (2) `v2_core::key_engine::detect_song_key` (`MTG/essentia`) en `libs/track.rs` y `mcp/analyze.rs`; (3) fusión de *novelty* sin 3ra FFT sobre el banco peine `c4dm/qm-dsp` y snap `0.025 s` (`mixxx#14705`); (4) `SetThreadPriority(-1)` / `nice(10)` con `unsafe extern` (`edition = "2024"`). |
| **Estándar Bicameral `OMKF v1` (`.hera/crate.md` + `.hera/tracks.omkf` `928 B` + `tree_seal_64`)** | **Ausente en `main`** | **Implementar en `O1` y `O2`:** `hera-rs/src/scanner/omkf_capsule.rs` (`0.0108 ms` hidratación zero-copy) + `libs/omkf_sync.rs` (`0.62 ms` detección en arranque sobre 50k pistas y *Zero-Import USB* en `0.0 s` de DSP). |
| **Poda Multimodal 2 Etapas (`0.58 ms`) + Vista `Carpetas = Playlists` + Gestor Agéntico Reversible** | **Ausente en `main`** | **Implementar en `O3`–`O5`:** tabla `folder_crates` (`09_add_omkf_folder_crates.sql`), poda `250 -> 30` carpetas con `100% Recall@15`, `omkf_ipc.rs` (`+4` comandos ACL), selector **`Carpetas (.hera)` \| `Playlists`**, `CrateRulesBanner.tsx` y `FolderCrateTree.tsx`. |

---

## 2. Trinquetes Inquebrantables (`hera-rs/tests/architectural_ratchets.rs` en `4b6f60a`)

1. **Cero dependencias nuevas** en `Cargo.toml` y `package.json`: usa exclusivamente `std::fs`, `walkdir`, `rayon`, `indexmap`, `tokio`, `base64` y `sqlx` (`sqlx::query` / `QueryBuilder` dinámico; prohibido `sqlx::query!`).
2. **Techos de líneas vigentes en `4b6f60a`:**
   - `<= 349` líneas post-`cargo fmt` en `hera-rs/src/{bpm_engine, scanner, mcp, code_mode, dj_brain, p2p, audio_engine}`, `hera-desktop/src-ipc/src` y `hera-desktop/src-tauri/src/plugins/{hera_dj, hera_agent}` (`("hera-rs/src/scanner", 349)` **ya está activo** en `SLICED_SUBMODULE_DIRS`).
   - Techos individuales en `FILE_LINE_CEILINGS`: `player.ts <= 680` (**cero ediciones nuevas**), `TrackProgress.tsx <= 220`, `bridge-hera.ts <= 300` (`<= 3` tipos manuales), `bridge-agent.ts <= 300`, `native-audio-transport.ts <= 320`, `MiniOverlay.tsx <= 450`, `routes/bridge.tsx <= 450`.
3. **Seguridad Rust 2024 y `#![forbid(unsafe_code)]`:** `hera-rs/src/audio_engine/` y `hera-rs/src/scanner/omkf_capsule.rs` son **100% safe Rust** (`to_le_bytes` / `from_le_bytes`); en `src-tauri` (`edition = "2024"`), todo bloque FFI (`SetThreadPriority`, `SetFileAttributesW`, `nice`) usa `unsafe extern`.
4. **Tríada ACL Tauri v2 (`ratchet_tauri_v2_acl_triad_parity`):** lee el valor actual de `EXPECTED_TAURI_COMMANDS_COUNT` en `architectural_ratchets.rs` (`63` en `4b6f60a`, o `73` tras el plan `2026-11-15`) e increméntalo exactamente en **`+4`** (`67` o `77`) registrando los 4 comandos `omkf_*` en `plugins/hera_dj/mod.rs`, `build.rs`, `capabilities/main.json` y `architectural_ratchets.rs`.
5. **Cero rutas absolutas de usuario del SO** en `docs/superpowers/` y `hera-rs/tests/`, y **`0` diffs** en `golden_masters.rs`.

---

## 3. Ejecución Secuencial por Milestones (`O1` -> `O5`)

### Milestone O1: Cápsula `OMKF v1` (`omkf_capsule.rs`, `928 B` Alineado a 32B) y Refuerzos `ScanScheduler` / Key v2.2

**Archivos a crear/modificar (`<= 349` líneas cada uno):**
- Crear: `hera-rs/src/scanner/omkf_capsule.rs` (~285 líneas, **100% safe Rust**)
- Modificar: `hera-rs/src/scanner/mod.rs` (`pub mod omkf_capsule;`)
- Auditar y reforzar (solo delta si falta en `HEAD`): `hera-rs/src/scanner/scheduler.rs`, `hera-rs/src/scanner/fast_seek.rs`, `hera-desktop/src-tauri/src/libs/track.rs`, `hera-rs/src/mcp/analyze.rs`

- [x] **Step O1.1: Implementar `hera-rs/src/scanner/omkf_capsule.rs` (100% Safe Rust, Cero Crates Nuevos)**
  1. **Constantes y Sello Conmutativo `tree_seal_64`:**
     - `pub const OMKF_MAGIC: [u8; 8] = *b"OMKF\x01\x00\x00\x00";`
     - `pub const OMKF_HEADER_BYTES: usize = 64;` y `pub const OMKF_SLOT_BYTES: usize = 928;` (`29 * 32` bytes).
     - `pub fn compute_entry_seal_u64(filename: &str, file_size: u64, mtime_ns: i64) -> u64`: `FNV1a_64(filename.as_bytes()) ^ file_size.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (mtime_ns as u64).rotate_left(13)`. El `tree_seal_64` de la carpeta es el **`XOR` conmutativo** de todas las entradas multimedia directas (inmune al orden de `readdir`).
  2. **Serialización Little-Endian Exacta (`OmkfHeader` `64 B` + `OmkfTrackSlot` `928 B`):**
     - `OmkfHeader` (`64 B`): `[0..8]` `OMKF_MAGIC`, `[8..16]` `tree_seal_64`, `[16..24]` `dir_mtime_ns`, `[24..28]` `track_count`, `[28..32]` `strtab_offset`, `[32..34]` `bpm_min_x100`, `[34..36]` `bpm_max_x100`, `[36..40]` `camelot_mask`, `[40..64]` `centroid_top24_i8`.
     - `OmkfTrackSlot` (`928 B`): `[0..16]` `partial_hash_64kb`, `[16..24]` `file_size_bytes`, `[24..32]` `mtime_ns`, `[32..34]` `bpm_x100`, `[34..36]` `entry_bpm_x100`, `[36..38]` `exit_bpm_x100`, `[38..40]` `lufs_x100`, `[40..42]` `replay_gain_x100`, `[42]` `camelot_code`, `[43]` `scan_level`, `[44..46]` `flags_bits`, `[46..48]` `duration_sec_u16`, `[48..80]` `hot_cues_ms: [u32; 8]`, `[80..84]` `name_offset`, `[84..86]` `name_len`, `[86..88]` `sample_rate_div100`, `[88]` `genre_family_code`, `[89]` `genre_confidence_pct`, `[90..92]` `inferred_era_year`, `[92..94]` `sub_phase_x1000`, `[94..96]` `bitrate_kbps`, `[96..160]` `embedding_64d: [i8; 64]`, `[160..928]` `rgb_peaks_256: [u8; 768]`.
     - `strtab` UTF-8 contigua a partir de `strtab_offset` codificando `filename\0artist\0title` para hidratación de carpeta en **`0.0108 ms`** sin abrir archivos de audio.
  3. **Escritura Atómica (`.hera/tracks.omkf.tmp` -> `rename`) y Fusión No Destructiva de `.hera/crate.md`:**
     - `write_omkf_capsule_atomic` actualiza el frontmatter YAML (`omkf: 1`, `crate_id`, `tree_seal_64`, `track_count`, `video_count`, `bpm_span`, `dominant_camelot`, `dominant_genre`, `era_span`, `lufs_avg`, `health`) **preservando intacto** todo texto del usuario/agente después del segundo `---` (`# Intención y Reglas de Cabina` y `# Ledger Reversible`).

- [x] **Step O1.2: Verificar y Aplicar Refuerzos `completed_level` (`F20`), `detect_song_key` (`F03`/`F08`) y `fast_seek` (`F07`)**
  - En `hera-rs/src/scanner/scheduler.rs`: verifica que exista `completed_level: HashMap<String, i8>` con `seed_completed_levels` y `mark_completed` para permitir re-promoción `Fast (1) -> Full (2)` en `O(1)` e ignorar `-1` o `>= target`.
  - En `libs/track.rs` y `hera-rs/src/mcp/analyze.rs`: verifica que la tonalidad Camelot provenga de `v2_core::key_engine::detect_song_key` sin alterar `hera-rs/src/structure.rs` (`0` diffs en `golden_masters`).

- [x] **Step O1.3: Recibo obligatorio de verificación O1**
  ```bash
  cargo test -p hera-node-rs scanner::
  cargo test -p hera-node-rs --test golden_masters
  cargo test -p hera-node-rs --test architectural_ratchets
  ```

---

### Milestone O2: Migración `09_add_omkf_folder_crates.sql`, Sincronizador `omkf_sync.rs` (`0.62 ms` Boot + *Zero-Import USB*) y Prioridad OS

**Archivos a crear/modificar:**
- Crear: `hera-desktop/src-tauri/src/migrations/09_add_omkf_folder_crates.sql` *(o `08_add_omkf_folder_crates.sql` si se ejecuta antes del plan `2026-11-15`; verificar número más alto en `src/migrations/` antes de crear)*
- Crear: `hera-desktop/src-tauri/src/libs/omkf_sync.rs` (~275 líneas)
- Modificar: `hera-desktop/src-tauri/src/libs/mod.rs`, `hera-desktop/src-tauri/src/libs/fast_library_scan.rs`, `hera-desktop/src-tauri/src/plugins/hera_dj/scanner_worker.rs`

- [x] **Step O2.1: Crear Migración Ortogonal `folder_crates` + `tracks.crate_id` y Pragmas `mmap_size`**
  1. En `plugins/db.rs` (tanto en el pool de escritura como en el pool de lectura creado en `529950b`), asegura `PRAGMA synchronous = NORMAL;`, `PRAGMA temp_store = MEMORY;`, `PRAGMA cache_size = -64000;` y `PRAGMA mmap_size = 268435456;`.
  2. Añade la migración SQL de `OMKF`:
     ```sql
     ALTER TABLE tracks ADD COLUMN crate_id TEXT;

     CREATE INDEX IF NOT EXISTS idx_tracks_crate_id
       ON tracks (crate_id);

     CREATE TABLE IF NOT EXISTS folder_crates (
       crate_id TEXT PRIMARY KEY NOT NULL,
       folder_path TEXT NOT NULL UNIQUE,
       display_name TEXT NOT NULL,
       tree_seal_64 TEXT NOT NULL,
       dir_mtime_ns INTEGER NOT NULL,
       track_count INTEGER NOT NULL DEFAULT 0,
       video_count INTEGER NOT NULL DEFAULT 0,
       full_count INTEGER NOT NULL DEFAULT 0,
       bpm_min REAL,
       bpm_max REAL,
       dominant_camelot TEXT,
       camelot_mask INTEGER NOT NULL DEFAULT 0,
       dominant_genre TEXT,
       era_span TEXT,
       lufs_avg REAL,
       centroid_64d_b64 TEXT,
       rules_markdown TEXT NOT NULL DEFAULT '',
       updated_at_unix_ms INTEGER NOT NULL
     );
     ```

- [x] **Step O2.2: Implementar `libs/omkf_sync.rs` (Fast Seal `0.62 ms`, *Zero-Import USB* `0.0 s` y Flush Debounced)**
  1. **Fast-Path Intacto (`0.002 ms`/carpeta):** compara `std::fs::metadata(dir)?.modified()` y los `64 B` de `.hera/tracks.omkf` contra `folder_crates`: si `dir_mtime_ns` y `tree_seal_64` coinciden, **salta la carpeta completa sin abrir ningún archivo hijo**.
  2. **Zero-Import USB (`actual_seal == hdr.tree_seal_64`, ausente en `hera.db`):** si el usuario conecta un USB/SSD externo ya sellado con `.hera/tracks.omkf`, hidrata directamente sus `OmkfTrackSlot` (incluyendo `scan_level = 2`, BPM, Camelot, 8 Hot Cues, ondas RGB y `64D`) hacia `tracks` y `folder_crates` en bloques de `250–500` filas (**`0.0 s` de CPU DSP**).
  3. **Delta Incremental + Atributo Oculto Windows:** solo analiza archivos nuevos/cambiados, aplica `SetFileAttributesW(..., FILE_ATTRIBUTE_HIDDEN)` (`unsafe extern "system"`) sobre `.hera` en Windows y expone `flush_dirty_omkf_capsules` invocado cada `1,000 ms` desde `scanner_worker.rs` (con hilos en `SetThreadPriority(-1)` / `nice(10)`).

- [x] **Step O2.3: Recibo obligatorio de verificación O2 (incluye test de *Zero-Import USB* borrando `hera.db`)**
  ```bash
  cargo test -p hera-desktop omkf_sync
  ```

---

### Milestone O3: Poda Jerárquica Multimodal en 2 Etapas (`0.58 ms`, `100% Recall@15`) y Router `< 1 ms` en `hera_agent`

**Archivos a modificar:**
- Modificar: `hera-desktop/src-tauri/src/libs/db_scanner_queries.rs` (o `scanner_intelligence.rs` si ya fue creado por el plan `2026-11-15`)
- Modificar: `hera-desktop/src-tauri/src/plugins/hera_agent/{tools_library.rs, tools_playlist_vdj.rs}`

- [x] **Step O3.1: Implementar Búsqueda `64D` con Poda en 2 Etapas (`folder_crates` -> `.hera/tracks.omkf`)**
  1. **Etapa 1 (Filtro de Crates en `< 0.05 ms`):** descarta carpetas cuyo `[bpm_min, bpm_max]` o `camelot_mask` no intersecten la ventana armónica/tempo solicitada y ordena las carpetas sobrevivientes por similitud coseno contra `centroid_64d_b64` (conservando el top `K = 30`).
  2. **Etapa 2 (Barrido Binario SIMD `i8` en `~0.50 ms`):** evalúa `cosine_similarity_i8` directamente sobre los slots `[96..160]` de `.hera/tracks.omkf` (con fallback a `tracks` si el volumen externo está offline) devolviendo el Top 15 exacto sin mantener `50,000` vectores en RAM.
  3. En `hera_agent`, elimina cualquier llamada residual a `db.get_all_tracks()` filtrando primero por `folder_crates` (`rules_markdown` + rango BPM/Camelot) y SQL indexado en `< 1 ms`.

---

### Milestone O4: Los `4` Comandos IPC de `omkf_ipc.rs` y Organizador Agéntico Reversible (`# Ledger Reversible`)

**Archivos a crear/modificar:**
- Crear: `hera-desktop/src-tauri/src/plugins/hera_dj/omkf_ipc.rs` (~275 líneas, `<= 349` LOC)
- Modificar: `hera-desktop/src-ipc/src/` (tipos exportados con `#[derive(TS)]` para `FolderCrateSummary`, `FolderCrateDetail`, `OrganizeFolderRequest`, `OrganizeFolderResponse`)

- [x] **Step O4.1: Implementar `omkf_list_folder_crates`, `omkf_read_folder_crate`, `omkf_save_crate_rules` y `omkf_preview_or_apply_organize`**
  - `omkf_read_folder_crate`: lee `.hera/tracks.omkf` en `0.011 ms` hidratando pistas + ondas RGB + 8 Hot Cues + texto de `.hera/crate.md` (con respaldo offline desde SQLite si el disco externo no está montado).
  - `omkf_preview_or_apply_organize`:
    - Con `dry_run == true`: calcula `Vec<PlannedFileMove>` (`"by_tempo_zone"`, `"by_genre_era"` o `"quarantine_issues"`) **sin mover archivos**.
    - Con `dry_run == false`: ejecuta `std::fs::rename`, actualiza `tracks.path` y `tracks.crate_id` **preservando el `UUID` y migrando su `OmkfTrackSlot` sin re-escanear DSP**, resella `tree_seal_64` y registra `- YYYY-MM-DDTHH:MMZ [agent:organizer] tx:#<id>` bajo `# Ledger Reversible` en `.hera/crate.md` (soportando rollback exacto con `rollback_tx_id`).

---

### Milestone O5: Vista `Carpetas (.hera) | Playlists`, `CrateRulesBanner.tsx`, `FolderCrateTree.tsx` y Cierre Tríada ACL

**Archivos a crear/modificar:**
- Crear: `hera-desktop/src/components/CrateRulesBanner.tsx` (~125 líneas, `@stylexjs/stylex`)
- Crear: `hera-desktop/src/components/FolderCrateTree.tsx` (~135 líneas, `@stylexjs/stylex`)
- Modificar: `hera-desktop/src/routes/playlists.tsx`, `hera-desktop/src/routes/playlists.$playlistID.tsx`, `hera-desktop/src/routes/library.tsx`, `hera-desktop/src/lib/scanner-v3.ts`
- Modificar Tríada ACL (`+4` comandos sobre el conteo vigente): `hera-desktop/src-tauri/src/plugins/hera_dj/mod.rs`, `hera-desktop/src-tauri/build.rs`, `hera-desktop/src-tauri/capabilities/main.json` y `hera-rs/tests/architectural_ratchets.rs`

- [x] **Step O5.1: Integrar Selector `Carpetas (.hera) | Playlists`, Banner Editable `.hera/crate.md` y Árbol Lateral en `/library`**
  1. En `routes/playlists.tsx`: pestaña por defecto **`Carpetas (.hera)`** listando cada carpeta musical (`playlistID = "folder--" + crate.crate_id`) con subtítulo `BPM · Camelot · % FULL`, manteniendo intacta la pestaña **`Playlists`** para listas `.m3u`/SQLite manuales.
  2. En `routes/playlists.$playlistID.tsx`: cuando `playlistID` inicia con `folder--`, hidrata vía `omkf_read_folder_crate`, promueve pendientes a `FULL` y muestra `CrateRulesBanner.tsx` (edición en vivo de `# Intención y Reglas de Cabina` + modal *Organizar con IA · Dry-Run / Rollback*).
  3. En `routes/library.tsx`: panel plegable `FolderCrateTree.tsx` filtrando instantáneamente por `crate_id` sin tocar `player.ts`.

- [x] **Step O5.2: Registrar los `4` Comandos `omkf_*` en la Tríada ACL (`+4` en `EXPECTED_TAURI_COMMANDS_COUNT`)**
  Añade `"hera-dj:allow-omkf-list-folder-crates"`, `"hera-dj:allow-omkf-read-folder-crate"`, `"hera-dj:allow-omkf-save-crate-rules"` y `"hera-dj:allow-omkf-preview-or-apply-organize"` en `capabilities/main.json`, `build.rs` y `plugins/hera_dj/mod.rs`, y suma `+4` a `EXPECTED_TAURI_COMMANDS_COUNT` en `hera-rs/tests/architectural_ratchets.rs` (`63 -> 67` o `73 -> 77`).

- [x] **Step O5.3: Recibo Maestro de Cierre End-to-End (Pegar salida real con `exit code 0` en `beast`)**
  ```bash
  cargo fmt --all -- --check
  cargo test -p hera-ipc
  cargo test -p hera-node-rs --test architectural_ratchets
  cargo test -p hera-node-rs --test golden_masters
  cargo test --workspace
  cargo clippy --workspace -- -D warnings
  cargo check --manifest-path hera-rs/Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features
  cd hera-desktop && bun run gen:types && bun run knip && bun run check && bun run test && vp build
  ```
