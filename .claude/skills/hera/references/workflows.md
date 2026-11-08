# Flujos de Trabajo Canónicos de Hera

## Flujo 1: Búsqueda y Presentación de Candidatos

1. El usuario solicita una búsqueda en lenguaje natural: *"Encuentra Drive de Black Coffee en lossless"*.
2. El agente invoca `search_music(query="Black Coffee Drive", filters={"format": ["FLAC", "ALAC"]})`.
3. El agente invoca `get_track_candidates(search_id=...)`.
4. El agente presenta la lista comparativa explicando las razones del score y las incertidumbres. No descarga nada automáticamente.

## Flujo 2: Adquisición Autorizada

1. El usuario selecciona un candidato y declara su base de autorización: *"Descarga el primer candidato, compré el álbum en Beatport"*.
2. El agente invoca `download_track(candidate_id="cand_07", authorization={"basis": "purchased_copy", "evidence_ref": "receipt:beatport:12345"}, idempotency_key="dl-cand07-12345")`.
3. El agente consulta `download_status(job_id=...)`.
4. Cuando el archivo está en cuarentena y validado, invoca `identify_track` y `analyze_track`.
5. Tras confirmación, invoca `organize_track` para promover a biblioteca.

## Flujo 3: Creación de Crate DJ (Rekordbox XML, Ableton Live, Audio & Video-DJ)

1. El usuario pide: *"Arma un set Afro House de 60 minutos a 122 BPM para Rekordbox y Ableton incluyendo mis producciones propias"*.
2. El agente invoca `build_dj_crate(brief="Afro House 122 BPM", duration_minutes=60, constraints={"bpm": [120, 124], "genre": "Afro_House", "media_type": "all", "include_own_productions": True}, export=["rekordbox_xml", "ableton", "cue_sheet", "m3u8"])`.
3. El agente entrega los archivos generados (`rekordbox_xml` con 4 Hot Cues acústicos, `.m3u8`, lote Ableton `[BPM]_[Camelot]_[Artista] - [Titulo].ext` + `_ABLETON_CURATION_INDEX.json`, y `_00_SET_GUIDE.txt`) y explica la progresión armónica Camelot/BPM.

## Flujo 4: Auditoría de Repertorio y Fase 1 Anti-Duplicados

1. El usuario pide analizar su repertorio o validar una lista de canciones antes de descargar.
2. El agente invoca `audit_repertoire(sync_db=True, check_queries=["Rauw Alejandro - Cosa Nuestra", ...])`.
3. El agente reporta las canciones que ya existen (`[SKIP - YA EXISTE]`), las faltantes (`[MISSING]`), la distribución por géneros (Audio y Video HD), años de inactividad (`2000-2026`), ADN del artista y bitrates sospechosos.
