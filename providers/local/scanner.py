"""Proveedor local para escanear e importar carpetas del sistema de archivos (Audio y Video-DJ HD)."""

from collections.abc import Sequence
from pathlib import Path
import os
import re
import shutil
import unicodedata
import uuid
from hera.contracts.candidate import Candidate, ScoreComponents, AuthorizationState
from hera.contracts.search import SearchFilters


DEFAULT_LOCAL_FORMATS = [
    "flac", "alac", "mp3", "aac", "ogg", "opus", "wav", "aiff",
    "m4a", "mp4", "webm", "mkv", "mov",
]
VIDEO_EXTENSIONS = {"mp4", "webm", "mkv", "mov", "m4v"}
LOSSLESS_EXTENSIONS = {"flac", "alac", "wav", "aiff"}


def strip_accents(text: str) -> str:
    """Elimina tildes y diacríticos para búsqueda insensible a acentos."""
    return "".join(
        c for c in unicodedata.normalize("NFD", text) if unicodedata.category(c) != "Mn"
    )


def parse_dj_filename(file_name: str) -> tuple[str, str, str | None, float | None, str | None]:
    """Extrae (artist, title, version, bpm, camelot) limpiando prefijos de track, basura web y tags DJ."""
    stem = file_name.rsplit(".", 1)[0] if "." in file_name else file_name
    bpm_found: float | None = None
    camelot_found: str | None = None

    # 1. Extraer etiqueta [124 BPM - 10B] o [124.5 BPM - 8A]
    m_bracket = re.search(
        r"\[\s*(\d{2,3}(?:\.\d+)?)\s*BPM\s*-\s*([0-9]{1,2}[AB])\s*\]",
        stem,
        flags=re.IGNORECASE,
    )
    if m_bracket:
        bpm_found = round(float(m_bracket.group(1)), 1)
        camelot_found = m_bracket.group(2).upper()
        stem = (stem[: m_bracket.start()] + " " + stem[m_bracket.end() :]).strip()

    # 2. Limpiar iterativamente prefijos de descarga web, prefijos Ableton (124_8A_) y numeración de pista (01 - )
    for _ in range(4):
        prev = stem
        stem = re.sub(r"^(?:y2mate\.com|ytconvert\.in|yt1s\.com|savefrom\.net)\s*-\s*", "", stem, flags=re.IGNORECASE).strip()

        # Formato Ableton: 124_8A_Artista - Titulo
        m_ableton = re.match(r"^(\d{2,3}(?:\.\d+)?)_([0-9]{1,2}[AB])_(.+)$", stem, flags=re.IGNORECASE)
        if m_ableton:
            if bpm_found is None:
                bpm_found = round(float(m_ableton.group(1)), 1)
            if camelot_found is None:
                camelot_found = m_ableton.group(2).upper()
            stem = m_ableton.group(3).strip()
            continue

        # Prefijo corto: [124 - 8A] o 01 [124B - 8A]
        m_short = re.match(
            r"^(?:\d{1,3}\s*)?\[\s*(\d{2,3})B?\s*-\s*([0-9]{1,2}[AB])\s*\]\s*(.+)$",
            stem,
            flags=re.IGNORECASE,
        )
        if m_short:
            if bpm_found is None:
                bpm_found = float(m_short.group(1))
            if camelot_found is None:
                camelot_found = m_short.group(2).upper()
            stem = m_short.group(3).strip()
            continue

        # Prefijo numérico de crate/track: "01 - ", "01. ", "001 "
        stem = re.sub(r"^\d{1,3}\s*[-_.]\s*", "", stem).strip()
        stem = re.sub(r"^\d{3}\s+(?=[A-Za-z\[])", "", stem).strip()
        if stem == prev:
            break

    # 3. Limpiar entidades HTML y sufijos de bitrate
    stem = re.sub(r"&#x[0-9A-Fa-f]+;?", "", stem).strip()
    stem = re.sub(r"_(?:128|160|192|256|320)k\b", "", stem, flags=re.IGNORECASE).strip()
    stem = re.sub(r"\s+", " ", stem).strip(" -_")

    # 4. Separar Artista y Título
    artist = "Unknown Artist"
    title = stem
    version: str | None = None

    if " - " in stem:
        parts = stem.split(" - ", 1)
        cand_artist, cand_title = parts[0].strip(), parts[1].strip()
        if cand_artist and not cand_artist.isdigit():
            artist = cand_artist
            title = cand_title

    # 5. Extraer versión entre corchetes o paréntesis si existe (sin confundir con BPM/Camelot)
    for delimiter in [("[", "]"), ("(", ")")]:
        if delimiter[0] in title and delimiter[1] in title:
            start = title.find(delimiter[0])
            end = title.find(delimiter[1], start + 1)
            if 0 <= start < end:
                candidate_ver = title[start + 1 : end].strip()
                if candidate_ver:
                    version = candidate_ver
                    title = (title[:start] + " " + title[end + 1 :]).strip()
                    title = re.sub(r"\s+", " ", title)
                    break

    return artist, title, version, bpm_found, camelot_found


class LocalProvider:
    name = "local"

    def __init__(self, folders: Sequence[Path | str] | None = None, supported_formats: Sequence[str] | None = None):
        self.folders = [Path(f) for f in (folders or [])]
        self.supported_formats = {
            f.lower().lstrip(".") for f in (supported_formats or DEFAULT_LOCAL_FORMATS)
        }

    async def capabilities(self) -> list[str]:
        return ["search", "local_import", "video_dj"]

    async def health(self) -> dict:
        valid_folders = [str(f) for f in self.folders if f.exists() and f.is_dir()]
        return {
            "provider": self.name,
            "status": "healthy" if valid_folders or not self.folders else "degraded",
            "folders_configured": len(self.folders),
            "folders_accessible": len(valid_folders),
        }

    async def search(self, query: str, filters: SearchFilters | None = None) -> list[Candidate]:
        candidates: list[Candidate] = []
        raw_q = (query or "").strip()
        is_wildcard = raw_q in {"*", "", "all", "*.*"}
        tokens = [] if is_wildcard else [t for t in strip_accents(raw_q.lower()).split() if t]
        search_id = f"srch_{uuid.uuid4().hex[:6]}"

        skip_prefixes = ("00_CRATES_DE_COMBATE", "00_SETS_ESTRATEGICOS", "curated_library")

        for folder in self.folders:
            if not folder.exists() or not folder.is_dir():
                continue

            for root, dirs, files in os.walk(folder):
                dirs[:] = [d for d in dirs if not d.startswith(skip_prefixes)]
                folder_context = strip_accents(Path(root).name.lower())

                for file_name in files:
                    ext = file_name.rsplit(".", 1)[-1].lower() if "." in file_name else ""
                    if ext not in self.supported_formats:
                        continue

                    if filters and filters.format:
                        allowed_exts = {fmt.lower().lstrip(".") for fmt in filters.format}
                        if ext not in allowed_exts:
                            continue

                    file_path = Path(root) / file_name
                    name_without_ext = file_name.rsplit(".", 1)[0]
                    searchable_text = f"{folder_context} {strip_accents(name_without_ext.lower())} {ext}"

                    if is_wildcard:
                        match_ratio = 1.0
                    else:
                        matches = sum(1 for token in tokens if token in searchable_text)
                        if matches == 0:
                            continue
                        match_ratio = matches / max(1, len(tokens))

                    artist, title, version, bpm_found, camelot_found = parse_dj_filename(file_name)

                    try:
                        file_size = file_path.stat().st_size
                    except OSError:
                        continue

                    is_lossless = ext in LOSSLESS_EXTENSIONS
                    is_video = ext in VIDEO_EXTENSIONS
                    bitrate = 1411 if is_lossless else 320

                    tech_score = 0.95 if (is_lossless or is_video) else 0.82
                    ident_score = min(1.0, match_ratio)
                    source_score = 0.95
                    avail_score = 1.0
                    pref_score = 0.92 if (is_lossless or is_video) else 0.75
                    meta_score = 0.90 if (bpm_found and camelot_found) else (0.75 if artist != "Unknown Artist" else 0.50)
                    risk_score = 0.95

                    total_score = (
                        30 * ident_score
                        + 25 * tech_score
                        + 15 * source_score
                        + 10 * avail_score
                        + 10 * pref_score
                        + 5 * meta_score
                        + 5 * risk_score
                    )

                    score_comps = ScoreComponents(
                        identity=ident_score,
                        technical=tech_score,
                        source=source_score,
                        availability=avail_score,
                        preference=pref_score,
                        metadata=meta_score,
                        risk=risk_score,
                    )

                    media_kind = "Video-DJ HD" if is_video else "Audio Local"
                    reasons = [
                        f"{media_kind} ({ext.upper()})",
                        f"Coincidencia de consulta: {int(match_ratio * 100)}%",
                    ]
                    if bpm_found and camelot_found:
                        reasons.append(f"Analizado DSP: {bpm_found:g} BPM - {camelot_found}")
                    if is_lossless:
                        reasons.append("Formato lossless de alta fidelidad")

                    cand = Candidate(
                        search_id=search_id,
                        provider=self.name,
                        native_ref=str(file_path.resolve()),
                        artist=artist,
                        title=title,
                        version=version,
                        format=ext.upper(),
                        bitrate_kbps=bitrate,
                        file_size_bytes=file_size,
                        score=round(total_score, 1),
                        score_components=score_comps,
                        score_reasons=reasons,
                        availability="immediate_local",
                        authorization_state=AuthorizationState.USER_CONFIRMATION_REQUIRED,
                    )
                    candidates.append(cand)

        candidates.sort(key=lambda c: c.score, reverse=True)
        return candidates

    async def resolve(self, native_ref: str) -> dict:
        path = Path(native_ref)
        if not path.exists():
            return {"exists": False, "path": native_ref}
        return {
            "exists": True,
            "path": str(path.resolve()),
            "size_bytes": path.stat().st_size,
            "filename": path.name,
        }

    async def start_transfer(self, candidate: Candidate, target_path: str) -> str:
        src = Path(candidate.native_ref)
        dst = Path(target_path)
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
        return f"local_copy_{uuid.uuid4().hex[:8]}"

    async def transfer_status(self, transfer_id: str) -> dict:
        return {"transfer_id": transfer_id, "state": "completed", "progress": 1.0}

    async def cancel_transfer(self, transfer_id: str) -> bool:
        return True
