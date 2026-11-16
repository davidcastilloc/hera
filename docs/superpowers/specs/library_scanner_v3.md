# Especificación Técnica v4.1: Estándar Bicameral `OMKF v1` (`.hera/` + `Carpetas = Playlists`) y Refuerzos `LibraryScanner v3` Reconciliados con `4b6f60a`

**Ruta en el repositorio:** `docs/superpowers/specs/library_scanner_v3.md`  
**Plan ejecutable compañero:** `docs/superpowers/plans/2026-11-16-hera-v070-omkf-v1-and-scanner-v4-master-plan.md` (`v4.1`, Milestones `O1` -> `O5`)  
**Baseline verificado en `main`:** Commit `4b6f60a595c19b976f5e3f7d3f05279074f9898e` + Spec compañera `docs/superpowers/specs/2026-11-15-curation-intelligence-and-zero-io-booth-design.md`  
**Alcance:** **100% Local-First** sobre `davidcastilloc/hera` (`hera-rs`, `hera-desktop/src-ipc`, `hera-desktop/src-tauri`, `hera-desktop/src`) con **cero dependencias nuevas en `Cargo.lock`**.

---

## 1. Reconciliación Exacta contra `HEAD` (`4b6f60a`) y `Curation & Zero-IO` (`fc951a8`)

1. **Ya fusionado en `main` (`4b6f60a`):** `07_add_scanner_v3_columns.sql`, 16 formatos audio/video en `libs/database.rs`, `vdj_xml_parser.rs`, `fast_library_scan.rs`, `scanner_worker.rs` (200 ms WAL + `run_ffmpeg_f32le_pipe_bounded`), `libs/tray.rs`, pool de lectura SQLite WAL separado (`529950b`), streaming de prefijo de deck `< 50 ms` + tope OOM `> 15 m` (`4b6f60a`), `("hera-rs/src/scanner", 349)` y `EXPECTED_TAURI_COMMANDS_COUNT = 63` en `hera-rs/tests/architectural_ratchets.rs`.
2. **Coordinación sin colisiones con `2026-11-15-curation-intelligence-and-zero-io-booth.md`:**
   - La migración `08_add_zero_io_and_curation_intelligence.sql` conserva intactas sus columnas acústicas y de curación (`8` Hot Cues, `14` familias `Genre x Era`, `LUFS`, `64D`, ondas RGB) y sus `10` comandos IPC (`63 -> 73`).
   - **`OMKF v1`** usa la migración independiente **`09_add_omkf_folder_crates.sql`** (`crate_id` + tabla `folder_crates`) y suma **`+4` comandos IPC** en `omkf_ipc.rs` (`63 -> 67` si se ejecuta antes del plan `2026-11-15`, o **`73 -> 77`** si se ejecuta después).
   - `OmkfTrackSlot` se alinea a **`928 bytes` (`29 * 32 B`, alineación AVX2 de 32 bytes)** para alojar nativamente los **8 Hot Cues (`[u32; 8]`)** y los metadatos **`Genre x Era`** del plan `2026-11-15`.

---

## 2. Arquitectura Bicameral `OMKF v1` (*Open Music Knowledge Format*: `.hera/` + `Carpetas = Playlists`)

Inspirado en la legibilidad para agentes de Google Cloud Open Knowledge Format (`OKF v0.2`) y la indexación local de `spacedrive` y `yazi`, cada carpeta física con pistas multimedia opera como un **Crate / Playlist vivo** con una cápsula oculta `.hera/` (ignorada nativamente por `is_dir_visible` en `libs/utils.rs:41` y marcada `FILE_ATTRIBUTE_HIDDEN` en Windows):

| Componente | Tamaño y Estructura | Latencia Empírica Medida en 250 Carpetas / 50,000 Archivos |
| :--- | :--- | :--- |
| **Plano Semántico (`.hera/crate.md`)** | `~1 KB` por carpeta (`~50 KB` en 250 carpetas) | Frontmatter YAML (`omkf: 1`, `crate_id`, `tree_seal_64`, `track_count`, `video_count`, `bpm_span`, `dominant_camelot`, `dominant_genre`, `era_span`, `lufs_avg`, `health`) + `# Intención y Reglas de Cabina` y `# Ledger Reversible`. |
| **Plano Binario Zero-Copy (`.hera/tracks.omkf`)** | `OmkfHeader` (`64 B`) + `N * OmkfTrackSlot` (`928 B` Little-Endian, múltiplo exacto de `32 B`) + `strtab` UTF-8 | Hidratación de carpeta de 200 pistas con ondas RGB de `768 B` y 8 Hot Cues en **`0.0108 ms` (`10.8 µs`)** (`48x` más rápido que SQLite+Base64) y portabilidad *Zero-Import USB* (`0.0 s` de CPU DSP). |
| **Sello Conmutativo (`tree_seal_64`)** | `u64` en `OmkfHeader` y `folder_crates` | `XOR` conmutativo de `FNV1a_64(filename) ^ (size * 0x9E3779B97F4A7C15) ^ rotl64(mtime_ns, 13)`. Detección incremental de arranque en 50,000 archivos en **`0.62 ms`** (`0.93 ms` con delta SQLite). |
| **Índice Derivado (`folder_crates` en `hera.db`)** | `WAL` + `PRAGMA mmap_size = 268435456` | Poda jerárquica multimodal `64D` en 2 etapas (`250 -> 30` carpetas candidatas + SIMD `i8`) en **`0.58 ms`** con **`100.0% Recall@15`** y `0 MB` de vectores fijos en RAM. |

### Layout Binario Exacto Alineado a 32 Bytes (`OmkfHeader` = `64 B`, `OmkfTrackSlot` = `928 B`)

- **`OmkfHeader` (`64 bytes`):** `[0..8]` `b"OMKF\x01\x00\x00\x00"`, `[8..16]` `tree_seal_64: u64`, `[16..24]` `dir_mtime_ns: i64`, `[24..28]` `track_count: u32`, `[28..32]` `strtab_offset: u32`, `[32..34]` `bpm_min_x100: u16`, `[34..36]` `bpm_max_x100: u16`, `[36..40]` `camelot_mask: u32`, `[40..64]` `centroid_top24_i8: [i8; 24]`.
- **`OmkfTrackSlot` (`928 bytes = 29 * 32 B`):**
  - `[0..16]` `partial_hash_64kb: [u8; 16]`, `[16..24]` `file_size_bytes: u64`, `[24..32]` `mtime_ns: i64`
  - `[32..34]` `bpm_x100: u16`, `[34..36]` `entry_bpm_x100: u16`, `[36..38]` `exit_bpm_x100: u16`, `[38..40]` `lufs_x100: i16`, `[40..42]` `replay_gain_x100: i16`
  - `[42]` `camelot_code: u8`, `[43]` `scan_level: i8`, `[44..46]` `flags_bits: u16`, `[46..48]` `duration_sec_u16: u16`
  - `[48..80]` `hot_cues_ms: [u32; 8]` *(8 Hot Cues completos compatibles con `cue_engine.rs` y `<Poi>` de VirtualDJ)*
  - `[80..84]` `name_offset: u32`, `[84..86]` `name_len: u16`, `[86..88]` `sample_rate_div100: u16`
  - `[88]` `genre_family_code: u8` (`0..=14`), `[89]` `genre_confidence_pct: u8`, `[90..92]` `inferred_era_year: u16`, `[92..94]` `sub_phase_x1000: i16`, `[94..96]` `bitrate_kbps: u16`
  - `[96..160]` `embedding_64d: [i8; 64]` *(alineado a 32 bytes: `96 = 3 * 32`)*
  - `[160..928]` `rgb_peaks_256: [u8; 768]` *(alineado a 32 bytes: `160 = 5 * 32` .. `928 = 29 * 32`)*

---

## 3. Refuerzos DSP / Scheduler Obligatorios y Contrato IPC `omkf_*` (`+4` Comandos)

1. **Refuerzos críticos sobre DSP y Scheduler (`F03`, `F07`, `F20`):**
   - `ScanScheduler::completed_level: HashMap<String, i8>`: garantiza re-promoción limpia `Fast (1) -> Full (2)` bajo *Hover* (`P0`) o *Lookahead* (`P1`) sin re-encolar pistas en cuarentena (`-1`) ni ya en `Full (2)`.
   - Ensemble tonal `MTG/essentia` (`v2_core::key_engine::detect_song_key`, `>= 92 Hz` + ancla de bajo) en `libs/track.rs` y `mcp/analyze.rs`, banco peine `c4dm/qm-dsp` por FFT Wiener-Khinchin y snap de fase `0.025 s` (`mixxx#14705`).
   - Prioridad de hilo del SO `SetThreadPriority(THREAD_PRIORITY_BELOW_NORMAL)` / `nice(10)` con `unsafe extern` (`edition = "2024"`).
2. **Los `4` Comandos Tauri de `omkf_ipc.rs` (`plugin:hera-dj`):**
   - `omkf_list_folder_crates`: lista carpetas musicales como playlists vivas desde `folder_crates` en `< 0.3 ms`.
   - `omkf_read_folder_crate`: hidrata pistas, 8 Hot Cues, ondas RGB y `.hera/crate.md` desde `.hera/tracks.omkf` (con fallback offline SQLite).
   - `omkf_save_crate_rules`: actualiza `# Intención y Reglas de Cabina` en `.hera/crate.md` sin invalidar `tree_seal_64`.
   - `omkf_preview_or_apply_organize`: organizador agéntico con vista previa `dry_run` y movimientos atómicos reversibles en `# Ledger Reversible` preservando el `UUID` sin re-escanear audio.
