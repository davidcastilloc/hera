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

## Flujo 3: Creación de Crate DJ

1. El usuario pide: *"Arma un set Afro House de 60 minutos a 122 BPM"*.
2. El agente invoca `build_dj_crate(brief="Afro House 122 BPM", duration_minutes=60, constraints={"bpm": [120, 124]}, export=["m3u8", "rekordbox_xml"])`.
3. El agente entrega los archivos de exportación generados y explica la coherencia del set.
