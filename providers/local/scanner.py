"""Proveedor local para escanear e importar carpetas del sistema de archivos."""

from pathlib import Path
import os
import shutil
import uuid
from hera.contracts.candidate import Candidate, ScoreComponents, AuthorizationState
from hera.contracts.search import SearchFilters


class LocalProvider:
    name = "local"

    def __init__(self, folders: list[Path | str] | None = None, supported_formats: list[str] | None = None):
        self.folders = [Path(f) for f in (folders or [])]
        self.supported_formats = {
            f.lower().lstrip(".") for f in (supported_formats or ["flac", "alac", "mp3", "aac", "ogg", "opus", "wav", "aiff"])
        }

    async def capabilities(self) -> list[str]:
        return ["search", "local_import"]

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
        tokens = query.lower().split()
        search_id = f"srch_{uuid.uuid4().hex[:6]}"

        for folder in self.folders:
            if not folder.exists() or not folder.is_dir():
                continue

            for root, _, files in os.walk(folder):
                for file_name in files:
                    ext = file_name.rsplit(".", 1)[-1].lower() if "." in file_name else ""
                    if ext not in self.supported_formats:
                        continue

                    # Filtro de formato si fue especificado
                    if filters and filters.format:
                        allowed_exts = {fmt.lower().lstrip(".") for fmt in filters.format}
                        if ext not in allowed_exts:
                            continue

                    file_path = Path(root) / file_name
                    name_without_ext = file_name.rsplit(".", 1)[0]
                    name_lower = name_without_ext.lower()

                    # Matching de tokens de consulta en el nombre de archivo
                    matches = sum(1 for token in tokens if token in name_lower)
                    if matches == 0:
                        continue

                    match_ratio = matches / max(1, len(tokens))

                    # Parse heurístico de artista y título
                    artist = "Unknown Artist"
                    title = name_without_ext
                    version = None

                    if " - " in name_without_ext:
                        parts = name_without_ext.split(" - ", 1)
                        artist = parts[0].strip()
                        title = parts[1].strip()

                    # Extraer versión entre corchetes o paréntesis si existe
                    for delimiter in [("[", "]"), ("(", ")")]:
                        if delimiter[0] in title and delimiter[1] in title:
                            start = title.find(delimiter[0])
                            end = title.find(delimiter[1])
                            if start < end:
                                version = title[start + 1 : end].strip()
                                title = (title[:start] + title[end + 1 :]).strip()

                    file_size = file_path.stat().st_size
                    is_lossless = ext in {"flac", "alac", "wav", "aiff"}
                    bitrate = 1411 if is_lossless else 320

                    # Puntuación explicable
                    tech_score = 0.95 if is_lossless else 0.80
                    ident_score = min(1.0, match_ratio)
                    source_score = 0.95  # Local folder es fuente confiable
                    avail_score = 1.0    # Inmediatamente accesible
                    pref_score = 0.90 if is_lossless else 0.70
                    meta_score = 0.75 if " - " in name_without_ext else 0.50
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

                    reasons = [
                        f"Archivo local existente ({ext.upper()})",
                        f"Coincidencia de consulta: {int(match_ratio * 100)}%",
                    ]
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
