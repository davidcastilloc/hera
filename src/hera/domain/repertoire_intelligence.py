"""Motor de Inteligencia y Auditoría de Repertorio Musical & Video-DJ (Géneros, Zonas BPM, Camelot, Años de Inactividad, Calidad y ADN del Artista)."""

from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
import os
import uuid
import xml.etree.ElementTree as ET

from hera.contracts.track import TrackStatus
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.dedup import DeduplicationEngine
from providers.local.scanner import (
    DEFAULT_LOCAL_FORMATS,
    LOSSLESS_EXTENSIONS,
    VIDEO_EXTENSIONS,
    parse_dj_filename,
)


class RepertoireAuditor:
    """Ejecuta diagnóstico integral del repertorio local (Audio, Video-DJ HD y Producciones Propias)."""

    def __init__(self, config: HeraConfig):
        self.config = config
        self.supported_exts = {
            f".{ext.lower().lstrip('.')}"
            for ext in (config.analysis.supported_formats or DEFAULT_LOCAL_FORMATS)
        }

    def scan_inventory(self) -> list[dict]:
        skip_prefixes = ("00_CRATES_DE_COMBATE", "00_SETS_ESTRATEGICOS", "curated_library")
        seen_paths: set[str] = set()
        items: list[dict] = []

        for folder_str in self.config.providers.local_folders:
            root_p = Path(folder_str)
            if not root_p.exists() or not root_p.is_dir():
                continue
            root_abs = os.path.abspath(str(root_p))
            root_lower = root_abs.lower()

            if "_videos" in root_lower or "video" in root_p.name.lower():
                category = "video_dj"
            elif "_mis_producciones" in root_lower or "masters" in root_p.name.lower():
                category = "own_productions"
            else:
                category = "audio_dj"

            for dirpath, dirs, filenames in os.walk(root_abs):
                dirs[:] = [d for d in dirs if not d.startswith(skip_prefixes)]
                rel_dir = os.path.relpath(dirpath, root_abs)
                top_genre = root_p.name if rel_dir == "." else rel_dir.split(os.sep)[0]

                for fname in filenames:
                    if fname == ".gitkeep" or fname.startswith("_00_"):
                        continue
                    dot = fname.rfind(".")
                    if dot == -1:
                        continue
                    ext = fname[dot:].lower()
                    if ext not in self.supported_exts:
                        continue

                    full_p = os.path.join(dirpath, fname)
                    norm_p = os.path.normcase(full_p)
                    if norm_p in seen_paths:
                        continue
                    seen_paths.add(norm_p)

                    try:
                        st = os.stat(full_p)
                        fsize = st.st_size
                        mtime_year = datetime.fromtimestamp(st.st_mtime).year
                    except OSError:
                        continue

                    artist, title, version, bpm, camelot = parse_dj_filename(fname)
                    ext_clean = ext.lstrip(".")
                    is_video = ext_clean in VIDEO_EXTENSIONS or category == "video_dj"

                    items.append({
                        "path": full_p,
                        "filename": fname,
                        "category": category,
                        "genre_folder": top_genre,
                        "ext": ext_clean,
                        "is_video": is_video,
                        "size_bytes": fsize,
                        "mtime_year": mtime_year,
                        "artist": artist,
                        "title": title,
                        "version": version,
                        "bpm": bpm,
                        "camelot": camelot,
                    })

        return items

    def run_full_audit(
        self,
        check_queries: list[str] | None = None,
    ) -> dict:
        items = self.scan_inventory()
        total_files = len(items)

        by_cat = Counter(it["category"] for it in items)
        tagged_count = sum(1 for it in items if it["bpm"] and it["camelot"])

        # 1. Zonas de energía (BPM)
        bpm_zones = {
            "warmup_urbano_70_105": {"audio": 0, "video": 0, "own_productions": 0},
            "tropical_salsa_106_121": {"audio": 0, "video": 0, "own_productions": 0},
            "club_afro_guaracha_122_132": {"audio": 0, "video": 0, "own_productions": 0},
            "hora_loca_merengue_133_plus": {"audio": 0, "video": 0, "own_productions": 0},
        }
        camelot_audio: Counter[str] = Counter()
        camelot_video: Counter[str] = Counter()
        camelot_own: Counter[str] = Counter()
        own_bpms: list[float] = []

        genre_counts: dict[str, dict[str, int]] = defaultdict(lambda: {"audio": 0, "video": 0, "total": 0})
        low_bitrate_suspects = 0
        short_clip_suspects = 0
        mtime_years: Counter[int] = Counter()

        for it in items:
            cat = it["category"]
            b = it["bpm"]
            cam = it["camelot"]
            g = it["genre_folder"]
            sz_mb = it["size_bytes"] / (1024 * 1024)
            mtime_years[it["mtime_year"]] += 1

            bucket = "video" if it["is_video"] else ("own_productions" if cat == "own_productions" else "audio")
            genre_counts[g]["video" if it["is_video"] else "audio"] += 1
            genre_counts[g]["total"] += 1

            if b:
                if b <= 105.9:
                    bpm_zones["warmup_urbano_70_105"][bucket] += 1
                elif b <= 121.9:
                    bpm_zones["tropical_salsa_106_121"][bucket] += 1
                elif b <= 132.9:
                    bpm_zones["club_afro_guaracha_122_132"][bucket] += 1
                else:
                    bpm_zones["hora_loca_merengue_133_plus"][bucket] += 1
                if cat == "own_productions":
                    own_bpms.append(b)

            if cam:
                if it["is_video"]:
                    camelot_video[cam] += 1
                elif cat == "own_productions":
                    camelot_own[cam] += 1
                else:
                    camelot_audio[cam] += 1

            if not it["is_video"] and "tools" not in g.lower() and "acappella" not in g.lower():
                if sz_mb < 1.4:
                    short_clip_suspects += 1
                elif sz_mb < 2.6:
                    low_bitrate_suspects += 1

        # 2. Detectar años con baja densidad entre 2000 y 2026
        year_source = mtime_years
        sparse_years = [yr for yr in range(2000, 2027) if year_source.get(yr, 0) < 15]

        # 3. Balance de géneros ordenado de mayor a menor
        sorted_genres = [
            {
                "genre": k,
                "audio": v["audio"],
                "video": v["video"],
                "total": v["total"],
                "share_pct": round((v["total"] / max(1, total_files)) * 100.0, 2),
            }
            for k, v in sorted(genre_counts.items(), key=lambda x: x[1]["total"], reverse=True)
        ]

        # 4. Perfil ADN de Producciones Propias + ADN Musical Integral del DJ (DJ Style Profiler)
        avg_own_bpm = round(sum(own_bpms) / len(own_bpms), 1) if own_bpms else 122.0
        artist_dna = {
            "total_own_productions": by_cat["own_productions"],
            "average_bpm": avg_own_bpm,
            "dominant_camelot_keys": [k for k, _ in camelot_own.most_common(5)],
        }

        artist_counter: Counter[str] = Counter()
        all_bpms: list[float] = []
        combined_camelot: Counter[str] = Counter()
        for it in items:
            art_name = (it.get("artist") or "").strip()
            if art_name and art_name.lower() not in ("unknown", "unknown artist", "varios", "va"):
                artist_counter[art_name] += 1
            if it.get("bpm"):
                all_bpms.append(float(it["bpm"]))
            if it.get("camelot"):
                combined_camelot[str(it["camelot"]).upper()] += 1

        zone_totals = {
            z_name: (z_vals["audio"] + z_vals["video"] + z_vals["own_productions"])
            for z_name, z_vals in bpm_zones.items()
        }
        dominant_bpm_zone = (
            max(zone_totals.items(), key=lambda kv: kv[1])[0]
            if any(zone_totals.values())
            else "club_afro_guaracha_122_132"
        )
        total_camelot_tagged = sum(combined_camelot.values())
        minor_count = sum(cnt for cam_k, cnt in combined_camelot.items() if cam_k.endswith("A"))
        minor_ratio_pct = round((minor_count / max(1, total_camelot_tagged)) * 100.0, 1)
        overall_avg_bpm = round(sum(all_bpms) / len(all_bpms), 1) if all_bpms else 124.0

        dj_style_dna = {
            "dominant_bpm_zone": dominant_bpm_zone,
            "overall_average_bpm": overall_avg_bpm,
            "dominant_camelot_keys": [k for k, _ in combined_camelot.most_common(5)],
            "minor_key_ratio_pct": minor_ratio_pct,
            "top_anchor_artists": [
                {"artist": name, "track_count": cnt}
                for name, cnt in artist_counter.most_common(15)
            ],
            "media_mix_profile": {
                "audio_pct": round((by_cat["audio_dj"] / max(1, total_files)) * 100.0, 1),
                "video_pct": round((by_cat["video_dj"] / max(1, total_files)) * 100.0, 1),
                "own_productions_pct": round((by_cat["own_productions"] / max(1, total_files)) * 100.0, 1),
            },
        }

        report = {
            "summary": {
                "total_unique_files": total_files,
                "audio_dj_files": by_cat["audio_dj"],
                "video_dj_files": by_cat["video_dj"],
                "own_productions_files": by_cat["own_productions"],
                "tagged_bpm_camelot_count": tagged_count,
                "tagged_pct": round((tagged_count / max(1, total_files)) * 100.0, 2),
            },
            "bpm_zones": bpm_zones,
            "dominant_camelot": {
                "audio_top5": dict(camelot_audio.most_common(5)),
                "video_top5": dict(camelot_video.most_common(5)),
            },
            "genre_breakdown": sorted_genres[:25],
            "chronology_and_inactivity": {
                "sparse_or_inactive_years_2000_2026": sparse_years,
                "recent_years_distribution": {str(y): year_source.get(y, 0) for y in range(2015, 2027)},
            },
            "quality_audit": {
                "low_bitrate_audio_suspects_under_2_6mb": low_bitrate_suspects,
                "short_clip_or_fx_outside_tools_under_1_4mb": short_clip_suspects,
            },
            "artist_dna_profile": artist_dna,
            "dj_style_dna": dj_style_dna,
        }

        if check_queries:
            report["deduplication_audit"] = DeduplicationEngine.audit_queries_against_folders(
                queries=check_queries,
                local_folders=self.config.providers.local_folders,
                fuzzy_threshold=self.config.identity_confidence_threshold,
            )

        return report


async def sync_local_library_to_db(db: Database, config: HeraConfig) -> dict:
    """Sincroniza in-place todos los archivos de `local_folders` en la tabla `tracks` de `hera.db` sin copiar archivos,
    purgando rutas obsoletas (archivos renombrados) y actualizando BPM/Camelot (`hera-pre/3.0`)."""
    auditor = RepertoireAuditor(config)
    items = auditor.scan_inventory()
    conn = await db.connect()

    valid_disk_paths = {os.path.normcase(it["path"]): it for it in items}
    local_roots_norm = tuple(
        os.path.normcase(os.path.abspath(str(Path(f))))
        for f in config.providers.local_folders
        if Path(f).exists()
    )

    cursor = await conn.execute(
        "SELECT id, library_path, quarantine_path, bpm, camelot FROM tracks WHERE deleted_at IS NULL"
    )
    rows = await cursor.fetchall()
    existing_paths: set[str] = set()
    stale_ids_to_delete: list[tuple[str]] = []
    rows_to_update: list[tuple[float, str, str, str, str]] = []
    now_iso = datetime.now(timezone.utc).isoformat()

    for r in rows:
        lib_p = r["library_path"]
        quar_p = r["quarantine_path"]
        if lib_p:
            norm_lib = os.path.normcase(lib_p)
            if norm_lib in valid_disk_paths:
                existing_paths.add(norm_lib)
                it = valid_disk_paths[norm_lib]
                new_bpm = it["bpm"] or 124.0
                new_cam = it["camelot"] or "8A"
                if (
                    r["bpm"] is None
                    or abs(float(r["bpm"]) - float(new_bpm)) > 0.05
                    or r["camelot"] != new_cam
                ):
                    rows_to_update.append((new_bpm, new_cam, "hera-pre/3.0", now_iso, r["id"]))
            elif local_roots_norm and norm_lib.startswith(local_roots_norm) and not os.path.exists(lib_p):
                stale_ids_to_delete.append((r["id"],))
            else:
                existing_paths.add(norm_lib)
        if quar_p:
            existing_paths.add(os.path.normcase(quar_p))

    if stale_ids_to_delete:
        await conn.executemany("DELETE FROM tracks WHERE id = ?", stale_ids_to_delete)

    if rows_to_update:
        await conn.executemany(
            "UPDATE tracks SET bpm = ?, camelot = ?, analysis_version = ?, updated_at = ? WHERE id = ?",
            rows_to_update,
        )

    to_insert = []

    for it in items:
        norm_p = os.path.normcase(it["path"])
        if norm_p in existing_paths:
            continue
        existing_paths.add(norm_p)

        ext = it["ext"]
        is_lossless = ext in LOSSLESS_EXTENSIONS
        bitrate = 1411 if is_lossless else 320
        tid = f"trk_{uuid.uuid4().hex[:10]}"

        to_insert.append((
            tid,
            TrackStatus.ORGANIZED.value,
            it["title"] or it["filename"],
            it["artist"] or "Unknown Artist",
            it["version"],
            210000,
            ext.upper(),
            bitrate,
            it["size_bytes"],
            it["path"],
            it["bpm"] or 124.0,
            0.95 if it["bpm"] else 0.70,
            it["camelot"] or "8A",
            0.92 if it["camelot"] else 0.70,
            0.80,
            0.85,
            -9.5,
            "hera-pre/3.0",
            "owned_original",
            now_iso,
            now_iso,
        ))

    if to_insert:
        await conn.executemany(
            """
            INSERT OR IGNORE INTO tracks (
                id, status, canonical_title, canonical_artist, version,
                duration_ms, codec, bitrate_kbps, file_size_bytes, library_path,
                bpm, bpm_confidence, camelot, key_confidence,
                energy, danceability, loudness_lufs, analysis_version,
                license_basis, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            to_insert,
        )

    if stale_ids_to_delete or rows_to_update or to_insert:
        await conn.commit()

    total_cursor = await conn.execute("SELECT COUNT(*) AS cnt FROM tracks WHERE deleted_at IS NULL")
    total_row = await total_cursor.fetchone()
    total_in_db = int(total_row["cnt"]) if total_row else len(existing_paths)

    return {
        "scanned_files": len(items),
        "purged_stale_renamed_tracks": len(stale_ids_to_delete),
        "updated_in_place_tracks": len(rows_to_update),
        "newly_indexed_tracks": len(to_insert),
        "total_tracks_in_db": total_in_db,
    }


async def persist_dj_style_dna_to_db(
    db: Database,
    audit_report: dict,
    profile_id: str = "default",
    onboarding_answers: dict | None = None,
) -> dict:
    """Persiste el ADN musical auto-calculado (`dj_style_dna`) y las respuestas de onboarding
    en la tabla `dj_preferences` de `hera.db` (`DJ Brain` local)."""
    import json

    await db.init_schema()
    conn = await db.connect()
    now_iso = datetime.now(timezone.utc).isoformat()

    dna = audit_report.get("dj_style_dna") or {}
    artist_dna = audit_report.get("artist_dna_profile") or {}
    summary = audit_report.get("summary") or {}

    entries: list[tuple[str, str, str, str, float, str, str]] = [
        (
            profile_id,
            "style_dna",
            "dominant_bpm_zone",
            str(dna.get("dominant_bpm_zone", "club_afro_guaracha_122_132")),
            1.0,
            "repertoire_auto_profiler",
            now_iso,
        ),
        (
            profile_id,
            "style_dna",
            "overall_average_bpm",
            str(dna.get("overall_average_bpm", 124.0)),
            1.0,
            "repertoire_auto_profiler",
            now_iso,
        ),
        (
            profile_id,
            "style_dna",
            "dominant_camelot_keys",
            json.dumps(dna.get("dominant_camelot_keys", []), ensure_ascii=False),
            1.0,
            "repertoire_auto_profiler",
            now_iso,
        ),
        (
            profile_id,
            "style_dna",
            "top_anchor_artists",
            json.dumps(dna.get("top_anchor_artists", []), ensure_ascii=False),
            1.0,
            "repertoire_auto_profiler",
            now_iso,
        ),
        (
            profile_id,
            "style_dna",
            "media_mix_profile",
            json.dumps(dna.get("media_mix_profile", {}), ensure_ascii=False),
            1.0,
            "repertoire_auto_profiler",
            now_iso,
        ),
        (
            profile_id,
            "style_dna",
            "own_productions_summary",
            json.dumps(artist_dna, ensure_ascii=False),
            1.0,
            "repertoire_auto_profiler",
            now_iso,
        ),
        (
            profile_id,
            "style_dna",
            "library_size",
            str(summary.get("total_unique_files", 0)),
            1.0,
            "repertoire_auto_profiler",
            now_iso,
        ),
    ]

    if onboarding_answers:
        for k, v in onboarding_answers.items():
            val_str = json.dumps(v, ensure_ascii=False) if isinstance(v, (dict, list)) else str(v)
            entries.append((
                profile_id,
                "onboarding",
                str(k),
                val_str,
                1.0,
                "dj_onboarding_confirmation",
                now_iso,
            ))

    await conn.executemany(
        """
        INSERT OR REPLACE INTO dj_preferences (
            profile_id, subject, feature, value, weight, evidence, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        """,
        entries,
    )
    await conn.commit()

    return {
        "profile_id": profile_id,
        "persisted_features": len(entries),
        "updated_at": now_iso,
    }


async def load_dj_brain_context(db: Database, profile_id: str = "default") -> dict:
    """Carga desde `hera.db` el contexto compacto del `DJ Brain` (~250 tokens) con el ADN musical,
    software preferido (`VirtualDJ`, `Rekordbox`, `Traktor`, `Ableton`), reglas de oro y últimos crates."""
    await db.init_schema()
    conn = await db.connect()

    pref_cursor = await conn.execute(
        "SELECT subject, feature, value FROM dj_preferences WHERE profile_id = ?",
        (profile_id,),
    )
    pref_rows = await pref_cursor.fetchall()
    preferences: dict[str, dict[str, str]] = {"style_dna": {}, "onboarding": {}}
    for r in pref_rows:
        subj = r["subject"]
        preferences.setdefault(subj, {})[r["feature"]] = r["value"]

    trk_cursor = await conn.execute("SELECT COUNT(*) AS cnt FROM tracks WHERE deleted_at IS NULL")
    trk_row = await trk_cursor.fetchone()
    total_tracks = int(trk_row["cnt"]) if trk_row else 0

    crate_cursor = await conn.execute(
        "SELECT name, brief, duration_target_minutes FROM crates ORDER BY updated_at DESC LIMIT 5"
    )
    crate_rows = await crate_cursor.fetchall()
    recent_crates = [
        {
            "name": cr["name"],
            "brief": cr["brief"],
            "duration_minutes": cr["duration_target_minutes"],
        }
        for cr in crate_rows
    ]

    dna = preferences.get("style_dna", {})
    onb = preferences.get("onboarding", {})
    preferred_sw = onb.get("preferred_dj_software", "VirtualDJ / Rekordbox / Traktor")
    dj_handle = onb.get("dj_handle", "DJ")
    golden_rules = onb.get("golden_rules", "Priorizar alta calidad (FLAC/320k) y flujo armónico Camelot")

    prompt_block = (
        f"[MEMORIA ACTIVA DEL DJ BRAIN — PERFIL '{dj_handle}']\n"
        f"- Tracks indexados en hera.db: {total_tracks}\n"
        f"- Zona BPM dominante: {dna.get('dominant_bpm_zone', 'club_afro_guaracha_122_132')} "
        f"(Promedio: {dna.get('overall_average_bpm', '124.0')} BPM)\n"
        f"- Llaves Camelot dominantes: {dna.get('dominant_camelot_keys', '[]')}\n"
        f"- Software principal de cabina: {preferred_sw}\n"
        f"- Reglas de oro del DJ: {golden_rules}\n"
        f"- Últimos crates armados: {[c['name'] for c in recent_crates]}"
    )

    return {
        "profile_id": profile_id,
        "total_tracks_in_db": total_tracks,
        "preferences": preferences,
        "recent_crates": recent_crates,
        "prompt_block": prompt_block,
    }
