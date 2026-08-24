# Referencia de Tools MCP de Hera

## 1. `search_music`
- **Entrada:** `query` (str), `filters` (dict opcional: format, version, min_bitrate_kbps), `providers` (list[str] opcional: ["local", "slskd"])
- **Salida:** `search_id` (str), `providers_completed` (list[str]), `providers_failed` (list[str]), `candidate_count` (int)

## 2. `get_track_candidates`
- **Entrada:** `search_id` (str), `limit` (int default 10)
- **Salida:** Lista de `Candidate` con:
  - `candidate_id`, `provider`, `artist`, `title`, `version`, `duration_ms`, `format`, `bitrate_kbps`
  - `score` (0-100), `score_components`, `score_reasons`, `authorization_state`

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
- **Entrada:** `track_id` (str), `profile` (str default "dj-standard")
- **Salida:** `bpm`, `bpm_confidence`, `musical_key`, `camelot`, `key_confidence`, `energy`, `danceability`, `loudness_lufs`, `analysis_version`

## 7. `organize_track`
- **Entrada:** `track_id` (str), `template` (str opcional), `collision_policy` (str: review, suffix, skip)
- **Salida:** `track_id`, `status`, `source_path`, `destination_path`, `collision_detected`

## 8. `build_dj_crate`
- **Entrada:** `brief` (str), `duration_minutes` (int), `constraints` (dict opcional: bpm, camelot_max_step, exclude_versions), `export` (list[str]: ["m3u8", "rekordbox_xml"])
- **Salida:** `crate_id`, `name`, `track_count`, `total_duration_ms`, `exports` (map de rutas generadas), `constraints_unmet`
