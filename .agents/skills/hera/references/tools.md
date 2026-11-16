# Referencia de Tools MCP de Hera

## 0. `ensure_hera_updated` (Paso 0 Obligatorio al Iniciar)
- **Descripción:** Verifica automáticamente si el repositorio de Hera, el entorno Python (`.venv`), el binario nativo Rust (`hera-node`), los binarios de soporte (`slskd`, `rclone`, `ffmpeg`, `fpcalc`) y las skills globales del agente (`hera` y `hera-dj-set-curator`) están instalados y actualizados a la última versión; si detecta código nuevo o binarios desactualizados, los actualiza, compila y sincroniza en caliente.
- **Entrada:** `auto_compile_rust` (bool default True), `sync_skills` (bool default True), `pull_remote` (bool default True)
- **Salida:** `status`, `git` (`local_commit`, `remote_commit`, `updated`, `dirty_worktree_preserved`), `rust_binary` (`binary_name`, `version`, `rebuilt`, `release_binary_path`), `python_env`, `companion_binaries`, `skills_sync`

## 1. `search_music`
- **Entrada:** `query` (str), `filters` (dict opcional: format, version, min_bitrate_kbps, camelot, bpm_min, bpm_max, subgenre), `providers` (list[str] opcional: ["local", "slskd", "ytdlp"])
- **Salida:** `search_id` (str), `providers_completed` (list[str]), `providers_failed` (list[str]), `candidate_count` (int)

## 2. `get_track_candidates`
- **Entrada:** `search_id` (str), `limit` (int default 10)
- **Salida:** Lista de `Candidate` con:
  - `candidate_id`, `provider`, `artist`, `title`, `version`, `duration_ms`, `format`, `bitrate_kbps`
  - `score` (0-100), `score_components`, `score_reasons`, `authorization_state`, `hera_seal`

## 3. `download_track`
- **Entrada:**
  - `candidate_id` (str)
  - `authorization` (dict con `basis`, `evidence_ref`, `acknowledged_by`)
  - `idempotency_key` (str único)
  - `approval_token` (str opcional)
- **Salida:** `job_id` (str), `status` (str)

## 4. `download_status`
- **Entrada:** `job_id` (str)
- **Salida:** `job_id`, `type`, `state` (queued, running, completed, failed), `progress` (0.0-1.0), `result`, `error_code`, `error_message`

## 5. `identify_track`
- **Entrada:** `asset_id` (str)
- **Salida:** `asset_id`, `fingerprint`, `hypotheses` (lista con recording_mbid, artist, title, confidence), `review_required` (bool)

## 6. `analyze_track`
- **Entrada:** `track_id` (str: soporta `trk_*`, promoción in-place de `cand_*` o ruta local), `profile` (str default "dj-standard"), `genre_hint` (str opcional para corrección Half/Double-Time)
- **Salida:** `bpm`, `bpm_confidence`, `musical_key`, `camelot`, `key_confidence`, `energy`, `danceability`, `loudness_lufs`, `true_peak_dbtp`, `hot_cues` (4 puntos acústicos), `analysis_version`

## 7. `analyze_batch`
- **Entrada:** `paths_or_ids` (list[str]), `profile` (str default "dj-standard"), `max_workers` (int default 6)
- **Salida:** `processed` (int), `results` (list[dict]), `errors` (list[dict])

## 8. `organize_track`
- **Entrada:** `track_id` (str), `template` (str opcional), `collision_policy` (str: review, suffix, skip)
- **Salida:** `track_id`, `status`, `source_path`, `destination_path`, `collision_detected`

## 9. `build_dj_crate`
- **Entrada:** `brief` (str), `duration_minutes` (int), `constraints` (dict opcional: `bpm`, `camelot_max_step`, `exclude_versions`, `genre`, `media_type`, `include_own_productions`, `target_camelot`), `export` (list[str]: `["m3u8", "rekordbox_xml", "ableton", "cue_sheet"]`)
- **Salida:** `crate_id`, `name`, `track_count`, `total_duration_ms`, `exports` (map de rutas generadas incluyendo `rekordbox_xml` con 4 Hot Cues acústicos, `.m3u8`, lote Ableton `[BPM]_[Camelot]_[Artista] - [Titulo].ext` y `_00_SET_GUIDE.txt`), `sequence_preview`, `constraints_unmet`

## 10. `audit_repertoire`
- **Entrada:** `sync_db` (bool default True), `check_queries` (list[str] opcional para validación anti-duplicados Fase 1)
- **Salida:** `total_tracks_scanned`, `audio_count`, `video_count`, `own_productions_count`, `genre_distribution`, `chronology`, `inactivity_gaps_by_genre`, `query_dedup_audit`, `sync_summary`

## 11. `hera_code_mode` & `get_code_mode_sdk`
- **Descripción:** Ejecuta scripts TypeScript/JS o pipelines declarativos sobre el namespace `hera.*` en un solo turno sin enviar JSON intermedio por la ventana de contexto del LLM.
- **Entrada (`hera_code_mode`):** `code` (str TypeScript/JS), `pipeline` (list[dict] opcional), `context` (dict opcional)
- **Salida:** `executed_steps`, `logs`, `result`

## 12. `connect_dj_drive` & `get_node_manifest`
- **Descripción (`connect_dj_drive`):** Onboarding Zero-Tech para DJs conectando su carpeta compartida de Google Drive o bucket Cloudflare R2 con auditoría forense Zero-Trust, deduplicación CAS global (`content_sha256`) y SimHash 64-bit POPCNT.
- **Entrada (`connect_dj_drive`):** `dj_handle` (str), `display_name` (str opcional), `gdrive_folder_link` (str opcional), `r2_bucket_url` (str opcional), `tracks` (list[dict] opcional)
- **Salida:** `profile`, `admitted_count`, `quarantined_count`, `cas_deduplicated_count`, `admitted_tracks`
